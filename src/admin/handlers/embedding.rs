use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ProblemDetail;
use crate::models::{CrawlerProgress, CrawlerStatus, EmbeddingJob, JobType};
use crate::AppState;

use super::common::{
    clear_embedding_launch_guard_if_matches, embedding_launch_allowed, embedding_trigger_conflicts,
    finalize_owned_embedding_job, inject_job_environment, persist_embedding_terminal_progress,
    push_or_replace_embedding_history, read_job_progress_json, EmbeddingStatusResponse,
};

// Embeddings

#[derive(Deserialize, utoipa::ToSchema)]
pub struct TriggerEmbeddingRequest {
    pub source: String,
    #[serde(default)]
    pub rebuild: bool,
    #[serde(default)]
    pub dry_run: bool,
    pub batch_size: Option<u16>,
    pub filter: Option<String>,
}

#[utoipa::path(
    get,
    path = "/admin/api/embeddings/stats",
    responses(
        (status = 200, description = "Embedding statistics", body = serde_json::Value),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn embedding_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = state.ro_pool.clone();
    let stats =
        tokio::task::spawn_blocking(move || crate::db::embeddings::get_embedding_stats(&pool))
            .await
            .unwrap_or_default();

    Json(stats).into_response()
}

#[utoipa::path(
    post,
    path = "/admin/api/embeddings/trigger",
    request_body = TriggerEmbeddingRequest,
    responses(
        (status = 202, description = "Embedding triggered", body = serde_json::Value),
        (status = 400, description = "Invalid parameters", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 409, description = "Embedding already running", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn trigger_embedding(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TriggerEmbeddingRequest>,
) -> impl IntoResponse {
    let source = body.source.trim().to_lowercase();
    if source != "all"
        && !["leetcode", "atcoder", "codeforces", "luogu", "uva", "spoj"].contains(&source.as_str())
    {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }

    if let Some(bs) = body.batch_size {
        if !(1..=256).contains(&bs) {
            return ProblemDetail::bad_request("batch_size must be between 1 and 256")
                .into_response();
        }
    }

    if let Some(ref f) = body.filter {
        if f.trim().is_empty() {
            return ProblemDetail::bad_request("filter must not be empty").into_response();
        }
    }

    let job_id = uuid::Uuid::new_v4().to_string();

    let mut launch_guard = state.embedding_launch_guard.lock().await;
    if embedding_trigger_conflicts(&launch_guard) {
        return ProblemDetail::conflict("an embedding job is already running").into_response();
    }

    let mut lock = state.embedding_lock.lock().await;
    if let Some(ref job) = *lock {
        if job.status == CrawlerStatus::Running {
            return ProblemDetail::conflict("an embedding job is already running").into_response();
        }
    }

    *launch_guard = Some(job_id.clone());
    let started_at = chrono::Utc::now().to_rfc3339();

    let mut args = vec!["--source".to_string(), source.clone()];
    if body.rebuild {
        args.push("--rebuild".to_string());
    } else if body.dry_run {
        args.push("--dry-run".to_string());
    } else {
        args.push("--build".to_string());
    }
    if let Some(bs) = body.batch_size {
        args.push("--batch-size".to_string());
        args.push(bs.to_string());
    }
    if let Some(ref f) = body.filter {
        args.push("--filter".to_string());
        args.push(f.clone());
    }
    args.push("--job-id".to_string());
    args.push(job_id.clone());

    let job = EmbeddingJob {
        job_id: job_id.clone(),
        source: source.clone(),
        args: args.clone(),
        started_at,
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    };

    *lock = Some(job.clone());

    let artifact_paths = crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id)
        .expect("uuid job id should produce safe artifact paths");
    if let Err(err) = crate::utils::persist_job_metadata(
        &artifact_paths,
        crate::models::JobArtifactMetadata::from(&job),
    )
    .await
    {
        tracing::warn!("failed to persist embedding metadata: {}", err);
        let failed_job = if let Some(slot) = lock.as_mut() {
            slot.status = CrawlerStatus::Failed;
            slot.finished_at = Some(chrono::Utc::now().to_rfc3339());
            Some(slot.clone())
        } else {
            None
        };
        drop(lock);
        clear_embedding_launch_guard_if_matches(&mut launch_guard, &job_id);
        drop(launch_guard);
        if let Some(failed_job) = failed_job {
            let mut history = state.embedding_history.lock().await;
            push_or_replace_embedding_history(&mut history, failed_job);
        }
        return ProblemDetail::internal("failed to persist embedding metadata").into_response();
    }
    drop(lock);
    drop(launch_guard);
    if let Err(err) = crate::utils::refresh_retained_job_state_now(state.as_ref(), true, true).await
    {
        tracing::warn!(
            "failed to reconcile retained job history before embedding launch: {}",
            err
        );
    }

    let state_clone = state.clone();
    let timeout_secs = state.config.embedding.batch_timeout_secs;
    let spawned_job_id = job_id.clone();

    tokio::spawn(async move {
        let mut launch_guard = state_clone.embedding_launch_guard.lock().await;
        {
            let lock = state_clone.embedding_lock.lock().await;
            if !embedding_launch_allowed(&launch_guard, lock.as_ref(), &spawned_job_id) {
                clear_embedding_launch_guard_if_matches(&mut launch_guard, &spawned_job_id);
                return;
            }
        }

        let mut cmd = tokio::process::Command::new("uv");
        cmd.args(["run", "python3", "embedding_cli.py"]);
        cmd.args(&args);
        cmd.current_dir("scripts/");
        cmd.kill_on_drop(true);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ref cp) = state_clone.config_path {
            cmd.env("CONFIG_PATH", cp);
        }
        inject_job_environment(
            &mut cmd,
            &spawned_job_id,
            JobType::Embedding,
            &artifact_paths,
        );

        let mut child = match crate::utils::spawn_with_pgid(cmd) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to spawn embedding job: {}", e);
                let finished_job = {
                    let mut lock = state_clone.embedding_lock.lock().await;
                    let finished_job = finalize_owned_embedding_job(
                        &mut lock,
                        &spawned_job_id,
                        Some(CrawlerStatus::Failed),
                        Err(std::io::Error::other(e.to_string())),
                    );
                    if let Some(job) = finished_job.as_ref() {
                        persist_embedding_terminal_progress(&artifact_paths, job).await;
                    }
                    finished_job
                };
                clear_embedding_launch_guard_if_matches(&mut launch_guard, &spawned_job_id);
                drop(launch_guard);
                if let Some(job) = finished_job {
                    let mut history = state_clone.embedding_history.lock().await;
                    push_or_replace_embedding_history(&mut history, job);
                }
                return;
            }
        };

        let capture =
            match crate::utils::start_live_output_capture(&mut child, &artifact_paths).await {
                Ok(capture) => capture,
                Err(err) => {
                    tracing::error!("failed to start embedding output capture: {}", err);
                    if let Some(pid) = child.id() {
                        crate::utils::kill_pgid(pid);
                    }
                    let _ = child.wait().await;
                    let finished_job = {
                        let mut lock = state_clone.embedding_lock.lock().await;
                        let finished_job = finalize_owned_embedding_job(
                            &mut lock,
                            &spawned_job_id,
                            Some(CrawlerStatus::Failed),
                            Err(std::io::Error::other(err.to_string())),
                        );
                        if let Some(job) = finished_job.as_ref() {
                            persist_embedding_terminal_progress(&artifact_paths, job).await;
                        }
                        finished_job
                    };
                    clear_embedding_launch_guard_if_matches(&mut launch_guard, &spawned_job_id);
                    drop(launch_guard);
                    if let Some(job) = finished_job {
                        let mut history = state_clone.embedding_history.lock().await;
                        push_or_replace_embedding_history(&mut history, job);
                    }
                    return;
                }
            };

        let pid = child.id().expect("child should have a pid");
        {
            let mut active_pid = state_clone.active_embedding_pid.lock().await;
            *active_pid = Some(pid);
        }
        clear_embedding_launch_guard_if_matches(&mut launch_guard, &spawned_job_id);
        drop(launch_guard);

        let mut wait_task = tokio::spawn(async move { child.wait().await });
        let timed =
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), &mut wait_task)
                .await;

        let capture_result = match timed {
            Ok(Ok(Ok(status))) => {
                let terminal_status = if status.success() {
                    CrawlerStatus::Completed
                } else {
                    CrawlerStatus::Failed
                };
                (Some(terminal_status), capture.finish().await)
            }
            Ok(Ok(Err(e))) => {
                tracing::error!("embedding job error: {}", e);
                (Some(CrawlerStatus::Failed), capture.finish().await)
            }
            Ok(Err(e)) => {
                tracing::error!("embedding job join error: {}", e);
                (Some(CrawlerStatus::Failed), capture.finish().await)
            }
            Err(_) => {
                tracing::warn!("embedding job {} timed out", spawned_job_id);
                crate::utils::kill_pgid(pid);
                let _ = wait_task.await;
                (Some(CrawlerStatus::TimedOut), capture.finish().await)
            }
        };

        let finished_job = {
            let mut lock = state_clone.embedding_lock.lock().await;
            {
                let mut active_pid = state_clone.active_embedding_pid.lock().await;
                if lock
                    .as_ref()
                    .is_some_and(|job| job.job_id == spawned_job_id)
                    && active_pid.as_ref().is_some_and(|active| *active == pid)
                {
                    *active_pid = None;
                }
            }

            let finished_job = finalize_owned_embedding_job(
                &mut lock,
                &spawned_job_id,
                capture_result.0,
                capture_result.1,
            );
            if let Some(job) = finished_job.as_ref() {
                persist_embedding_terminal_progress(&artifact_paths, job).await;
            }
            finished_job
        };

        if let Some(job) = finished_job {
            let mut history = state_clone.embedding_history.lock().await;
            push_or_replace_embedding_history(&mut history, job);
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
    path = "/admin/api/embeddings/cancel",
    responses(
        (status = 200, description = "Embedding cancelled"),
        (status = 409, description = "No running embedding job", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn cancel_embedding(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let _launch_guard = state.embedding_launch_guard.lock().await;
    let cancelled_job = {
        let mut lock = state.embedding_lock.lock().await;
        if let Some(ref mut job) = *lock {
            if job.status == CrawlerStatus::Running {
                let mut pid_lock = state.active_embedding_pid.lock().await;
                if let Some(pid) = pid_lock.take() {
                    crate::utils::kill_pgid(pid);
                }
                job.status = CrawlerStatus::Cancelled;
                job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                Some(job.clone())
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some(job) = cancelled_job {
        let artifact_paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job.job_id)
                .expect("existing embedding job id should map to artifact paths");
        persist_embedding_terminal_progress(&artifact_paths, &job).await;
        let mut history = state.embedding_history.lock().await;
        push_or_replace_embedding_history(&mut history, job);
        return StatusCode::OK.into_response();
    }

    ProblemDetail::conflict("no running embedding job to cancel").into_response()
}

#[utoipa::path(
    get,
    path = "/admin/api/embeddings/status",
    responses(
        (status = 200, description = "Embedding status", body = EmbeddingStatusResponse),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn embedding_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if let Err(err) = crate::utils::maybe_refresh_retained_job_state(state.as_ref()).await {
        tracing::warn!(
            "failed to reconcile retained job state for embedding_status: {}",
            err
        );
    }

    let current_job = state.embedding_lock.lock().await.clone();
    let history_vec: Vec<_> = {
        let history = state.embedding_history.lock().await;
        history
            .iter()
            .rev()
            .map(|j| {
                let mut j = j.clone();
                j.stdout = None;
                j.stderr = None;
                j
            })
            .collect()
    };

    match current_job {
        Some(mut job) if job.status == CrawlerStatus::Running => {
            job.stdout = None;
            job.stderr = None;
            let progress = read_job_progress_json(JobType::Embedding, &job.job_id)
                .await
                .unwrap_or_else(|| serde_json::json!({ "phase": "queued" }));
            Json(EmbeddingStatusResponse {
                running: true,
                current_job: Some(job),
                last_job: None,
                progress: Some(progress),
                history: history_vec,
            })
            .into_response()
        }
        Some(mut job) => {
            job.stdout = None;
            job.stderr = None;
            let progress = read_job_progress_json(JobType::Embedding, &job.job_id)
                .await
                .unwrap_or_else(|| serde_json::json!({ "phase": "unknown" }));
            Json(EmbeddingStatusResponse {
                running: false,
                current_job: None,
                last_job: Some(job),
                progress: Some(progress),
                history: history_vec,
            })
            .into_response()
        }
        None => Json(EmbeddingStatusResponse {
            running: false,
            current_job: None,
            last_job: None,
            progress: None,
            history: history_vec,
        })
        .into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/admin/api/embeddings/{job_id}/output",
    params(
        ("job_id" = String, Path, description = "Embedding job ID"),
    ),
    responses(
        (status = 200, description = "Embedding output", body = serde_json::Value),
        (status = 400, description = "Invalid job ID", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 404, description = "Job output not found", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn embedding_output(
    State(_state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if uuid::Uuid::parse_str(&job_id).is_err() {
        return ProblemDetail::bad_request("invalid job_id").into_response();
    }
    let paths = match crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id) {
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
    path = "/admin/api/embeddings/{job_id}/progress",
    params(
        ("job_id" = String, Path, description = "Embedding job ID"),
    ),
    responses(
        (status = 200, description = "Embedding progress", body = CrawlerProgress),
        (status = 400, description = "Invalid job ID", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 404, description = "Embedding progress not found", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn embedding_progress(Path(job_id): Path<String>) -> impl IntoResponse {
    if uuid::Uuid::parse_str(&job_id).is_err() {
        return ProblemDetail::bad_request("invalid job_id").into_response();
    }

    match read_job_progress_json(JobType::Embedding, &job_id).await {
        Some(progress) => Json(progress).into_response(),
        None => ProblemDetail::not_found("embedding progress not found").into_response(),
    }
}
