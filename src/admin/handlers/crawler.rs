use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use super::common::{
    clear_manual_guard_if_matches, persist_crawler_terminal_progress,
    push_or_replace_crawler_history, read_job_progress_json, take_owned_manual_crawler_pid,
    CrawlerStatusResponse,
};
use crate::api::error::ProblemDetail;
use crate::models::{CrawlerProgress, CrawlerStatus, CrawlerTrigger, JobType};
use crate::AppState;

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
    match crate::crawler::launch(state, body.source, body.args, CrawlerTrigger::Admin).await {
        Ok(job_id) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({"job_id":job_id})),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
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
