use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ProblemDetail;
use crate::models::{
    ActiveCrawlerPid, CrawlerJob, CrawlerProgress, CrawlerSource, CrawlerStatus, CrawlerTrigger,
    JobType,
};
use crate::AppState;

use super::common::{
    clear_manual_guard_if_matches, finalize_owned_manual_crawler_job, inject_job_environment,
    manual_crawler_launch_allowed, manual_trigger_conflicts, persist_crawler_running_progress,
    persist_crawler_terminal_progress, push_or_replace_crawler_history, read_job_progress_json,
    take_owned_manual_crawler_pid, with_owned_manual_crawler_job, CrawlerStatusResponse,
};

// Crawler

#[derive(Deserialize, utoipa::ToSchema)]
pub struct TriggerCrawlerRequest {
    pub source: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/admin/api/crawlers/trigger",
    request_body = TriggerCrawlerRequest,
    responses(
        (status = 202, description = "Crawler triggered", body = serde_json::Value),
        (status = 400, description = "Invalid parameters", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 409, description = "Crawler already running", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn trigger_crawler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TriggerCrawlerRequest>,
) -> impl IntoResponse {
    let source = match CrawlerSource::parse(&body.source) {
        Ok(s) => s,
        Err(e) => return ProblemDetail::bad_request(e).into_response(),
    };

    let args = match crate::models::validate_args(&source, &body.args) {
        Ok(a) => a,
        Err(e) => return ProblemDetail::bad_request(e).into_response(),
    };

    let job_id = uuid::Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now().to_rfc3339();
    let runtime_key = crate::models::manual_crawler_runtime_key().to_string();

    {
        let mut manual_guard = state.manual_crawler_guard.lock().await;
        if manual_trigger_conflicts(&manual_guard) {
            return ProblemDetail::conflict("a manual crawler is already running").into_response();
        }
        *manual_guard = Some(job_id.clone());
    }

    let job = CrawlerJob {
        job_id: job_id.clone(),
        source: body.source.clone(),
        args: args.clone(),
        trigger: CrawlerTrigger::Admin,
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
            if let Some(job) = crawler_jobs.get_mut(&runtime_key) {
                job.status = CrawlerStatus::Failed;
                job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                let failed_job = job.clone();
                drop(crawler_jobs);
                *state.manual_crawler_guard.lock().await = None;
                let mut history = state.crawler_history.lock().await;
                push_or_replace_crawler_history(&mut history, failed_job);
            } else {
                drop(crawler_jobs);
                *state.manual_crawler_guard.lock().await = None;
            }
        }
        return ProblemDetail::internal("failed to persist crawler metadata").into_response();
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
        .get(&body.source)
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

        let mut manual_guard = state_clone.manual_crawler_guard.lock().await;
        let launch_allowed = {
            let crawler_jobs = state_clone.crawler_jobs.lock().await;
            manual_crawler_launch_allowed(
                &manual_guard,
                crawler_jobs.get(&runtime_key),
                &spawned_job_id,
            )
        };
        if !launch_allowed {
            let finished_job = {
                let mut crawler_jobs = state_clone.crawler_jobs.lock().await;
                with_owned_manual_crawler_job(&mut crawler_jobs, &spawned_job_id, |job| {
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
                    with_owned_manual_crawler_job(&mut crawler_jobs, &spawned_job_id, |job| {
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
                        with_owned_manual_crawler_job(&mut crawler_jobs, &spawned_job_id, |job| {
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

        let mut wait_task = tokio::spawn(async move { child.wait().await });
        let timed =
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), &mut wait_task)
                .await;

        {
            let mut pid_lock = state_clone.active_crawler_pids.lock().await;
            take_owned_manual_crawler_pid(&mut pid_lock, &spawned_job_id);
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
            finalize_owned_manual_crawler_job(
                &mut crawler_jobs,
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

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "job_id": job_id })),
    )
        .into_response()
}

#[utoipa::path(
    post,
    path = "/admin/api/crawlers/cancel",
    responses(
        (status = 200, description = "Crawler cancelled"),
        (status = 409, description = "No running crawler", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn cancel_crawler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let runtime_key = crate::models::manual_crawler_runtime_key().to_string();
    let mut manual_guard = state.manual_crawler_guard.lock().await;
    let mut crawler_jobs = state.crawler_jobs.lock().await;
    if let Some(job) = crawler_jobs.get_mut(&runtime_key) {
        if job.status == CrawlerStatus::Running {
            let job_id = job.job_id.clone();
            let mut pid_lock = state.active_crawler_pids.lock().await;
            if let Some(pid) = take_owned_manual_crawler_pid(&mut pid_lock, &job_id) {
                crate::utils::kill_pgid(pid);
            }
            job.status = CrawlerStatus::Cancelled;
            job.finished_at = Some(chrono::Utc::now().to_rfc3339());
            let cancelled_job = job.clone();
            clear_manual_guard_if_matches(&mut manual_guard, &job_id);
            drop(crawler_jobs);
            drop(manual_guard);

            let artifact_paths =
                crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id)
                    .expect("existing manual job id should map to artifact paths");
            persist_crawler_terminal_progress(&artifact_paths, &cancelled_job).await;
            let mut history = state.crawler_history.lock().await;
            push_or_replace_crawler_history(&mut history, cancelled_job);
            return StatusCode::OK.into_response();
        }
    }

    ProblemDetail::conflict("no running crawler to cancel").into_response()
}

#[utoipa::path(
    get,
    path = "/admin/api/crawlers/status",
    responses(
        (status = 200, description = "Crawler status", body = CrawlerStatusResponse),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn crawler_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if let Err(err) = crate::utils::maybe_refresh_retained_job_state(state.as_ref()).await {
        tracing::warn!(
            "failed to reconcile retained job state for crawler_status: {}",
            err
        );
    }

    let mut running_jobs: Vec<_> = {
        let crawler_jobs = state.crawler_jobs.lock().await;
        crawler_jobs
            .values()
            .filter(|job| job.status == CrawlerStatus::Running)
            .cloned()
            .collect()
    };
    running_jobs.sort_by(|a, b| {
        a.started_at
            .cmp(&b.started_at)
            .then_with(|| a.job_id.cmp(&b.job_id))
    });
    let running_jobs: Vec<_> = running_jobs
        .into_iter()
        .map(|mut job| {
            job.stdout = None;
            job.stderr = None;
            job
        })
        .collect();

    let history = state.crawler_history.lock().await;
    let history_vec: Vec<_> = history
        .iter()
        .rev()
        .map(|j| {
            let mut j = j.clone();
            j.stdout = None;
            j.stderr = None;
            j
        })
        .collect();

    Json(CrawlerStatusResponse {
        running: !running_jobs.is_empty(),
        running_jobs,
        history: history_vec,
    })
    .into_response()
}

#[utoipa::path(
    get,
    path = "/admin/api/crawlers/{job_id}/output",
    params(
        ("job_id" = String, Path, description = "Crawler job ID"),
    ),
    responses(
        (status = 200, description = "Crawler output", body = serde_json::Value),
        (status = 400, description = "Invalid job ID", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 404, description = "Job output not found", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn crawler_output(
    State(_state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if uuid::Uuid::parse_str(&job_id).is_err() {
        return ProblemDetail::bad_request("invalid job_id").into_response();
    }
    let paths = match crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id) {
        Ok(paths) => paths,
        Err(err) => return ProblemDetail::bad_request(err).into_response(),
    };
    let output = match crate::utils::read_job_output(&paths, None, None).await {
        Ok(Some(output)) => output,
        Ok(None) => return ProblemDetail::not_found("job output not found").into_response(),
        Err(err) => {
            return ProblemDetail::internal(format!("failed to read job output: {}", err))
                .into_response()
        }
    };

    Json(serde_json::json!({
        "stdout": output.stdout.unwrap_or_default(),
        "stderr": output.stderr.unwrap_or_default(),
        "python_log": output.python_log.unwrap_or_default(),
    }))
    .into_response()
}

#[utoipa::path(
    get,
    path = "/admin/api/crawlers/{job_id}/progress",
    params(
        ("job_id" = String, Path, description = "Crawler job ID"),
    ),
    responses(
        (status = 200, description = "Crawler progress", body = CrawlerProgress),
        (status = 400, description = "Invalid job ID", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 404, description = "Crawler progress not found", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn crawler_progress(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if uuid::Uuid::parse_str(&job_id).is_err() {
        return ProblemDetail::bad_request("invalid job_id").into_response();
    }

    if let Some(progress) = read_job_progress_json(JobType::Crawler, &job_id).await {
        return Json(progress).into_response();
    }

    let is_running = {
        let jobs = state.crawler_jobs.lock().await;
        jobs.values()
            .any(|job| job.job_id == job_id && job.status == CrawlerStatus::Running)
    };
    if is_running {
        return Json(serde_json::json!({ "phase": "queued" })).into_response();
    }

    ProblemDetail::not_found("crawler progress not found").into_response()
}
