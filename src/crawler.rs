//! Shared process lifecycle for manual and scheduled problem crawlers.
use crate::job::apply_terminal_update;
use crate::models::{
    ActiveCrawlerPid, CrawlerJob, CrawlerSource, CrawlerStatus, CrawlerTrigger, JobType,
};
use crate::utils::{
    inject_job_environment, persist_crawler_running_progress, persist_crawler_terminal_progress,
    push_or_replace_crawler_history,
};
use crate::{api::error::ProblemDetail, AppState};
use std::{collections::HashMap, sync::Arc};

pub(crate) fn manual_trigger_conflicts(manual_guard: &Option<String>) -> bool {
    manual_guard.is_some()
}

pub(crate) fn manual_crawler_launch_allowed(
    manual_guard: &Option<String>,
    job: Option<&CrawlerJob>,
    job_id: &str,
) -> bool {
    manual_guard.as_deref() == Some(job_id)
        && job
            .map(|job| job.job_id == job_id && job.status == CrawlerStatus::Running)
            .unwrap_or(false)
}

pub(crate) fn clear_manual_guard_if_matches(manual_guard: &mut Option<String>, job_id: &str) {
    if manual_guard.as_deref() == Some(job_id) {
        *manual_guard = None;
    }
}

#[cfg(test)]
pub(crate) fn with_owned_manual_crawler_job<T>(
    jobs: &mut HashMap<String, CrawlerJob>,
    id: &str,
    f: impl FnOnce(&mut CrawlerJob) -> T,
) -> Option<T> {
    with_owned_job(jobs, crate::models::manual_crawler_runtime_key(), id, f)
}

#[cfg(test)]
pub(crate) fn finalize_owned_manual_crawler_job(
    jobs: &mut HashMap<String, CrawlerJob>,
    id: &str,
    status: Option<CrawlerStatus>,
    capture: std::io::Result<crate::utils::CapturedOutput>,
) -> Option<CrawlerJob> {
    finalize_job(
        jobs,
        crate::models::manual_crawler_runtime_key(),
        id,
        status,
        capture,
    )
}

pub(crate) fn effective_source<'a>(source: &'a str, args: &'a [String]) -> &'a str {
    if source == "luogu" {
        args.windows(2)
            .find(|p| p[0] == "--source")
            .map(|p| p[1].as_str())
            .unwrap_or(source)
    } else {
        source
    }
}
fn with_owned_job<T>(
    jobs: &mut HashMap<String, CrawlerJob>,
    key: &str,
    id: &str,
    update: impl FnOnce(&mut CrawlerJob) -> T,
) -> Option<T> {
    let job = jobs.get_mut(key)?;
    if job.job_id != id {
        return None;
    }
    Some(update(job))
}

pub(crate) async fn launch(
    state: Arc<AppState>,
    source_name: String,
    raw_args: Vec<String>,
    trigger: CrawlerTrigger,
) -> Result<String, ProblemDetail> {
    let source = match CrawlerSource::parse(&source_name) {
        Ok(s) => s,
        Err(e) => return Err(ProblemDetail::bad_request(e)),
    };

    let args = match crate::models::validate_args(&source, &raw_args) {
        Ok(a) => a,
        Err(e) => return Err(ProblemDetail::bad_request(e)),
    };

    let job_id = uuid::Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now().to_rfc3339();
    let is_manual = trigger == CrawlerTrigger::Admin;
    let runtime_key = if is_manual {
        crate::models::manual_crawler_runtime_key().to_string()
    } else {
        format!("scheduled:{source_name}")
    };
    let launch_guard = state.crawler_schedule.launch.lock().await;
    if state
        .crawler_schedule
        .stopping
        .load(std::sync::atomic::Ordering::Acquire)
    {
        return Err(ProblemDetail::conflict("server is shutting down"));
    }

    {
        let mut manual_guard = state.manual_crawler_guard.lock().await;
        if is_manual && manual_trigger_conflicts(&manual_guard) {
            return Err(ProblemDetail::conflict(
                "a manual crawler is already running",
            ));
        }
        let jobs = state.crawler_jobs.lock().await;
        let effective = effective_source(&source_name, &args);
        if jobs.values().any(|job| {
            job.status == CrawlerStatus::Running
                && effective_source(&job.source, &job.args) == effective
                && (!is_manual || job.trigger == CrawlerTrigger::Scheduled)
        }) {
            return Err(ProblemDetail::conflict(
                "a crawler for this source is already running",
            ));
        }
        if is_manual {
            *manual_guard = Some(job_id.clone());
        }
    }

    let job = CrawlerJob {
        job_id: job_id.clone(),
        source: source_name.clone(),
        args: args.clone(),
        trigger,
        started_at: started_at.clone(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    };

    state
        .crawler_jobs
        .lock()
        .await
        .insert(runtime_key.clone(), job.clone());

    drop(launch_guard);

    let artifact_paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id)
        .expect("uuid job id should produce safe artifact paths");
    if let Err(err) = crate::utils::persist_job_metadata(
        &artifact_paths,
        crate::models::JobArtifactMetadata::from(&job),
    )
    .await
    {
        tracing::warn!("failed to persist crawler metadata: {}", err);
        {
            let mut crawler_jobs = state.crawler_jobs.lock().await;
            if let Some(job) = crawler_jobs
                .get_mut(&runtime_key)
                .filter(|job| job.job_id == job_id)
            {
                job.status = CrawlerStatus::Failed;
                job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                let failed_job = job.clone();
                drop(crawler_jobs);
                clear_manual_guard_if_matches(
                    &mut *state.manual_crawler_guard.lock().await,
                    &job_id,
                );
                let mut history = state.crawler_history.lock().await;
                push_or_replace_crawler_history(&mut history, failed_job);
            } else {
                drop(crawler_jobs);
                clear_manual_guard_if_matches(
                    &mut *state.manual_crawler_guard.lock().await,
                    &job_id,
                );
            }
        }
        return Err(ProblemDetail::internal(
            "failed to persist crawler metadata",
        ));
    }
    if let Err(err) = crate::utils::refresh_retained_job_state_now(state.as_ref(), true, true).await
    {
        tracing::warn!(
            "failed to reconcile retained job history before crawler launch: {}",
            err
        );
    }

    let script = source.script_name();
    let state_clone = state.clone();
    let spawned_job_id = job_id.clone();
    let timeout_secs = state
        .config
        .crawler
        .per_source_timeout
        .get(&source_name)
        .copied()
        .unwrap_or(state.config.crawler.timeout_secs);

    tokio::spawn(async move {
        let mut cmd = tokio::process::Command::new("uv");
        cmd.args(["run", "python3", script]);
        cmd.args(&args);
        cmd.current_dir("scripts/");
        cmd.kill_on_drop(true);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ref cp) = state_clone.config_path {
            cmd.env("CONFIG_PATH", cp);
        }
        inject_job_environment(&mut cmd, &spawned_job_id, JobType::Crawler, &artifact_paths);

        let launch_guard = state_clone.crawler_schedule.launch.lock().await;
        let mut manual_guard = state_clone.manual_crawler_guard.lock().await;
        let launch_allowed = {
            let crawler_jobs = state_clone.crawler_jobs.lock().await;
            !state_clone
                .crawler_schedule
                .stopping
                .load(std::sync::atomic::Ordering::Acquire)
                && (!is_manual
                    || manual_crawler_launch_allowed(
                        &manual_guard,
                        crawler_jobs.get(&runtime_key),
                        &spawned_job_id,
                    ))
                && crawler_jobs.get(&runtime_key).is_some_and(|job| {
                    job.job_id == spawned_job_id && job.status == CrawlerStatus::Running
                })
        };
        if !launch_allowed {
            let finished_job = {
                let mut crawler_jobs = state_clone.crawler_jobs.lock().await;
                with_owned_job(&mut crawler_jobs, &runtime_key, &spawned_job_id, |job| {
                    if job.status == CrawlerStatus::Running {
                        job.status = CrawlerStatus::Cancelled;
                    }
                    if job.finished_at.is_none() {
                        job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                    }
                    job.clone()
                })
            };
            clear_manual_guard_if_matches(&mut manual_guard, &spawned_job_id);
            drop(manual_guard);
            if let Some(job) = finished_job {
                persist_crawler_terminal_progress(&artifact_paths, &job).await;
                let mut history = state_clone.crawler_history.lock().await;
                push_or_replace_crawler_history(&mut history, job);
            }
            return;
        }

        let mut child = match crate::utils::spawn_with_pgid(cmd) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to spawn crawler: {}", e);
                let finished_job = {
                    let mut crawler_jobs = state_clone.crawler_jobs.lock().await;
                    with_owned_job(&mut crawler_jobs, &runtime_key, &spawned_job_id, |job| {
                        job.status = CrawlerStatus::Failed;
                        job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                        job.clone()
                    })
                };
                clear_manual_guard_if_matches(&mut manual_guard, &spawned_job_id);
                drop(manual_guard);
                if let Some(job) = finished_job {
                    persist_crawler_terminal_progress(&artifact_paths, &job).await;
                    let mut history = state_clone.crawler_history.lock().await;
                    push_or_replace_crawler_history(&mut history, job);
                }
                return;
            }
        };

        let capture =
            match crate::utils::start_live_output_capture(&mut child, &artifact_paths).await {
                Ok(capture) => capture,
                Err(err) => {
                    tracing::error!("failed to start crawler output capture: {}", err);
                    if let Some(pid) = child.id() {
                        crate::utils::kill_pgid(pid);
                    }
                    let _ = child.wait().await;
                    let finished_job = {
                        let mut crawler_jobs = state_clone.crawler_jobs.lock().await;
                        with_owned_job(&mut crawler_jobs, &runtime_key, &spawned_job_id, |job| {
                            job.status = CrawlerStatus::Failed;
                            job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                            job.clone()
                        })
                    };
                    clear_manual_guard_if_matches(&mut manual_guard, &spawned_job_id);
                    drop(manual_guard);
                    if let Some(job) = finished_job {
                        persist_crawler_terminal_progress(&artifact_paths, &job).await;
                        let mut history = state_clone.crawler_history.lock().await;
                        push_or_replace_crawler_history(&mut history, job);
                    }
                    return;
                }
            };

        let pid = child.id().expect("child should have a pid");
        state_clone.active_crawler_pids.lock().await.insert(
            runtime_key.clone(),
            ActiveCrawlerPid {
                job_id: spawned_job_id.clone(),
                pid,
            },
        );
        if let Some(job) = {
            let crawler_jobs = state_clone.crawler_jobs.lock().await;
            crawler_jobs
                .get(&runtime_key)
                .cloned()
                .filter(|job| job.job_id == spawned_job_id && job.status == CrawlerStatus::Running)
        } {
            persist_crawler_running_progress(&artifact_paths, &job).await;
        }
        drop(manual_guard);
        drop(launch_guard);

        let mut wait_task = tokio::spawn(async move { child.wait().await });
        let timed =
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), &mut wait_task)
                .await;

        {
            let mut pid_lock = state_clone.active_crawler_pids.lock().await;
            if pid_lock
                .get(&runtime_key)
                .is_some_and(|entry| entry.job_id == spawned_job_id)
            {
                pid_lock.remove(&runtime_key);
            }
        }
        let was_cancelled = {
            let crawler_jobs = state_clone.crawler_jobs.lock().await;
            crawler_jobs
                .get(&runtime_key)
                .map(|job| job.job_id == spawned_job_id && job.status == CrawlerStatus::Cancelled)
                .unwrap_or(false)
        };
        let (terminal_status, capture_result) = if was_cancelled {
            (None, capture.finish().await)
        } else {
            match timed {
                Ok(Ok(Ok(status))) => {
                    let terminal_status = if status.success() {
                        CrawlerStatus::Completed
                    } else {
                        CrawlerStatus::Failed
                    };
                    (Some(terminal_status), capture.finish().await)
                }
                Ok(Ok(Err(e))) => {
                    tracing::error!("crawler error: {}", e);
                    (Some(CrawlerStatus::Failed), capture.finish().await)
                }
                Ok(Err(e)) => {
                    tracing::error!("crawler join error: {}", e);
                    (Some(CrawlerStatus::Failed), capture.finish().await)
                }
                Err(_) => {
                    tracing::warn!("crawler job {} timed out", spawned_job_id);
                    crate::utils::kill_pgid(pid);
                    let _ = wait_task.await;
                    (Some(CrawlerStatus::TimedOut), capture.finish().await)
                }
            }
        };
        let finished_job = {
            let mut crawler_jobs = state_clone.crawler_jobs.lock().await;
            finalize_job(
                &mut crawler_jobs,
                &runtime_key,
                &spawned_job_id,
                terminal_status,
                capture_result,
            )
        };

        let mut manual_guard = state_clone.manual_crawler_guard.lock().await;
        if manual_guard.as_deref() == Some(spawned_job_id.as_str()) {
            *manual_guard = None;
        }
        drop(manual_guard);

        if let Some(job) = finished_job {
            persist_crawler_terminal_progress(&artifact_paths, &job).await;
            let mut history = state_clone.crawler_history.lock().await;
            push_or_replace_crawler_history(&mut history, job);
        }
    });

    Ok(job_id)
}

fn finalize_job(
    jobs: &mut HashMap<String, CrawlerJob>,
    key: &str,
    id: &str,
    status: Option<CrawlerStatus>,
    capture: std::io::Result<crate::utils::CapturedOutput>,
) -> Option<CrawlerJob> {
    with_owned_job(jobs, key, id, |job| {
        if job.status == CrawlerStatus::Running && job.finished_at.is_none() {
            job.finished_at = Some(chrono::Utc::now().to_rfc3339());
        }
        apply_terminal_update(job, status, capture, "crawler");
        job.clone()
    })
}
