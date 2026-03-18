use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Form, Json};
use rand::Rng;
use serde::Deserialize;

use crate::api::error::ProblemDetail;
use crate::api::problems::{validate_list_query, ListMeta, ListQuery, ListResponse, VALID_SOURCES};
use crate::auth::{AdminSecret, AdminSessions};
use crate::models::{
    ActiveCrawlerPid, CrawlerJob, CrawlerSource, CrawlerStatus, CrawlerTrigger, EmbeddingJob,
    JobArtifactMetadata, JobArtifactPaths, JobType, Problem,
};
use crate::AppState;

trait TerminalJob {
    fn status(&self) -> &CrawlerStatus;
    fn status_mut(&mut self) -> &mut CrawlerStatus;
    fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>);
}

impl TerminalJob for CrawlerJob {
    fn status(&self) -> &CrawlerStatus {
        &self.status
    }

    fn status_mut(&mut self) -> &mut CrawlerStatus {
        &mut self.status
    }

    fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        CrawlerJob::set_output(self, stdout, stderr);
    }
}

impl TerminalJob for EmbeddingJob {
    fn status(&self) -> &CrawlerStatus {
        &self.status
    }

    fn status_mut(&mut self) -> &mut CrawlerStatus {
        &mut self.status
    }

    fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        EmbeddingJob::set_output(self, stdout, stderr);
    }
}

fn apply_terminal_update<T: TerminalJob>(
    job: &mut T,
    terminal_status: Option<CrawlerStatus>,
    capture_result: std::io::Result<crate::utils::CapturedOutput>,
    job_kind: &str,
) {
    let already_terminal = crate::utils::crawler_status_is_terminal(job.status());

    match capture_result {
        Ok(output) => {
            if let Some(status) = terminal_status.filter(|_| !already_terminal) {
                *job.status_mut() = status;
            }
            job.set_output(output.stdout, output.stderr);
        }
        Err(err) => {
            tracing::error!("{} capture error: {}", job_kind, err);
            if already_terminal {
                return;
            }
            match terminal_status {
                Some(CrawlerStatus::TimedOut) => {
                    *job.status_mut() = CrawlerStatus::TimedOut;
                }
                Some(_) | None => {
                    *job.status_mut() = CrawlerStatus::Failed;
                }
            }
        }
    }
}

fn inject_job_environment(
    cmd: &mut tokio::process::Command,
    job_id: &str,
    job_type: JobType,
    artifact_paths: &JobArtifactPaths,
) {
    crate::utils::inject_job_environment(cmd, job_id, job_type, artifact_paths);
}

async fn persist_crawler_running_progress(artifact_paths: &JobArtifactPaths, job: &CrawlerJob) {
    crate::utils::persist_crawler_running_progress(artifact_paths, job).await;
}

async fn persist_crawler_terminal_progress(artifact_paths: &JobArtifactPaths, job: &CrawlerJob) {
    crate::utils::persist_crawler_terminal_progress(artifact_paths, job).await;
}

fn embedding_phase_is_terminal(phase: &str) -> bool {
    matches!(phase, "completed" | "failed" | "cancelled" | "timed_out")
}

async fn persist_embedding_terminal_progress(
    artifact_paths: &JobArtifactPaths,
    job: &EmbeddingJob,
) {
    if job.status == CrawlerStatus::Running {
        return;
    }

    let final_phase = match job.status {
        CrawlerStatus::Completed => "completed",
        CrawlerStatus::Failed => "failed",
        CrawlerStatus::Cancelled => "cancelled",
        CrawlerStatus::TimedOut => "timed_out",
        CrawlerStatus::Running => return,
    };
    let metadata = JobArtifactMetadata::from(job);
    let progress_path = artifact_paths.progress.clone();
    let result =
        crate::utils::update_json_atomic(&progress_path, |current: Option<serde_json::Value>| {
            let mut prog = current.unwrap_or_else(|| serde_json::json!({}));
            let current_phase = prog.get("phase").and_then(|value| value.as_str());
            if !current_phase.is_some_and(embedding_phase_is_terminal) {
                prog["phase"] = serde_json::json!(final_phase);
            }
            if prog.get("started_at").is_none() {
                prog["started_at"] = serde_json::json!(metadata.started_at.clone());
            }
            prog["updated_at"] = serde_json::json!(metadata.updated_at.clone());
            prog["metadata"] = serde_json::to_value(&metadata).unwrap_or(serde_json::Value::Null);
            Ok(prog)
        })
        .await;
    if let Err(err) = result {
        tracing::warn!("failed to persist final embedding progress: {}", err);
    }
}

fn manual_trigger_conflicts(manual_guard: &Option<String>) -> bool {
    manual_guard.is_some()
}

fn manual_crawler_launch_allowed(
    manual_guard: &Option<String>,
    job: Option<&CrawlerJob>,
    job_id: &str,
) -> bool {
    manual_guard.as_deref() == Some(job_id)
        && job
            .map(|job| job.job_id == job_id && job.status == CrawlerStatus::Running)
            .unwrap_or(false)
}

fn clear_manual_guard_if_matches(manual_guard: &mut Option<String>, job_id: &str) {
    if manual_guard.as_deref() == Some(job_id) {
        *manual_guard = None;
    }
}

fn with_owned_manual_crawler_job<T>(
    crawler_jobs: &mut HashMap<String, CrawlerJob>,
    job_id: &str,
    update: impl FnOnce(&mut CrawlerJob) -> T,
) -> Option<T> {
    let job = crawler_jobs.get_mut(crate::models::manual_crawler_runtime_key())?;
    if job.job_id != job_id {
        return None;
    }
    Some(update(job))
}

fn finalize_owned_manual_crawler_job(
    crawler_jobs: &mut HashMap<String, CrawlerJob>,
    job_id: &str,
    terminal_status: Option<CrawlerStatus>,
    capture_result: std::io::Result<crate::utils::CapturedOutput>,
) -> Option<CrawlerJob> {
    with_owned_manual_crawler_job(crawler_jobs, job_id, |job| {
        if job.status == CrawlerStatus::Running && job.finished_at.is_none() {
            job.finished_at = Some(chrono::Utc::now().to_rfc3339());
        }
        apply_terminal_update(job, terminal_status, capture_result, "crawler");
        job.clone()
    })
}

fn take_manual_crawler_pid(
    active_crawler_pids: &mut HashMap<String, ActiveCrawlerPid>,
) -> Option<u32> {
    active_crawler_pids
        .remove(crate::models::manual_crawler_runtime_key())
        .map(|active_pid| active_pid.pid)
}

fn take_owned_manual_crawler_pid(
    active_crawler_pids: &mut HashMap<String, ActiveCrawlerPid>,
    job_id: &str,
) -> Option<u32> {
    let runtime_key = crate::models::manual_crawler_runtime_key();
    match active_crawler_pids.get(runtime_key) {
        Some(active_pid) if active_pid.job_id == job_id => {
            take_manual_crawler_pid(active_crawler_pids)
        }
        _ => None,
    }
}

fn finalize_owned_embedding_job(
    embedding_slot: &mut Option<EmbeddingJob>,
    job_id: &str,
    terminal_status: Option<CrawlerStatus>,
    capture_result: std::io::Result<crate::utils::CapturedOutput>,
) -> Option<EmbeddingJob> {
    let job = embedding_slot.as_mut()?;
    if job.job_id != job_id {
        return None;
    }
    if job.status == CrawlerStatus::Running && job.finished_at.is_none() {
        job.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }
    apply_terminal_update(job, terminal_status, capture_result, "embedding");
    Some(job.clone())
}

fn embedding_trigger_conflicts(launch_guard: &Option<String>) -> bool {
    launch_guard.is_some()
}

fn embedding_launch_allowed(
    launch_guard: &Option<String>,
    job: Option<&EmbeddingJob>,
    job_id: &str,
) -> bool {
    launch_guard.as_deref() == Some(job_id)
        && job
            .map(|job| job.job_id == job_id && job.status == CrawlerStatus::Running)
            .unwrap_or(false)
}

fn clear_embedding_launch_guard_if_matches(launch_guard: &mut Option<String>, job_id: &str) {
    if launch_guard.as_deref() == Some(job_id) {
        *launch_guard = None;
    }
}

fn push_or_replace_crawler_history(
    history: &mut std::collections::VecDeque<CrawlerJob>,
    job: CrawlerJob,
) {
    crate::utils::push_or_replace_crawler_history(history, job);
}

fn push_or_replace_embedding_history(
    history: &mut std::collections::VecDeque<EmbeddingJob>,
    job: EmbeddingJob,
) {
    if let Some(existing) = history
        .iter_mut()
        .find(|existing| existing.job_id == job.job_id)
    {
        *existing = job;
        return;
    }
    if history.len() >= crate::utils::RETAINED_JOB_HISTORY_LIMIT {
        history.pop_front();
    }
    history.push_back(job);
}

#[cfg(test)]
fn collect_running_job_keys<'a>(
    crawler_jobs: impl IntoIterator<Item = &'a CrawlerJob>,
    embedding_job: Option<&EmbeddingJob>,
) -> std::collections::HashSet<(JobType, String)> {
    let mut active_jobs = std::collections::HashSet::new();
    active_jobs.extend(
        crawler_jobs
            .into_iter()
            .filter(|job| job.status == CrawlerStatus::Running)
            .map(|job| (JobType::Crawler, job.job_id.clone())),
    );
    if let Some(job) = embedding_job.filter(|job| job.status == CrawlerStatus::Running) {
        active_jobs.insert((JobType::Embedding, job.job_id.clone()));
    }
    active_jobs
}

// Problem CRUD

#[derive(Deserialize)]
pub struct CreateProblemRequest {
    pub id: String,
    pub source: String,
    pub slug: String,
    pub title: Option<String>,
    pub title_cn: Option<String>,
    pub difficulty: Option<String>,
    pub ac_rate: Option<f64>,
    pub rating: Option<f64>,
    pub contest: Option<String>,
    pub problem_index: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub link: Option<String>,
    pub category: Option<String>,
    pub paid_only: Option<i32>,
    pub content: Option<String>,
    pub content_cn: Option<String>,
    #[serde(default)]
    pub similar_questions: Vec<String>,
}

impl From<CreateProblemRequest> for Problem {
    fn from(r: CreateProblemRequest) -> Self {
        Problem {
            id: r.id,
            source: r.source,
            slug: r.slug,
            title: r.title,
            title_cn: r.title_cn,
            difficulty: r.difficulty,
            ac_rate: r.ac_rate,
            rating: r.rating,
            contest: r.contest,
            problem_index: r.problem_index,
            tags: r.tags,
            link: r.link,
            category: r.category,
            paid_only: r.paid_only,
            content: r.content,
            content_cn: r.content_cn,
            similar_questions: r.similar_questions,
        }
    }
}

pub async fn create_problem(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateProblemRequest>,
) -> impl IntoResponse {
    let problem: Problem = body.into();
    let pool = state.rw_pool.clone();

    let result =
        tokio::task::spawn_blocking(move || crate::db::problems::insert_problem(&pool, &problem))
            .await;

    match result {
        Ok(Ok(())) => StatusCode::CREATED.into_response(),
        Ok(Err(e)) => ProblemDetail::internal(format!("database error: {}", e)).into_response(),
        Err(_) => ProblemDetail::internal("task join error").into_response(),
    }
}

pub async fn update_problem(
    State(state): State<Arc<AppState>>,
    Path((source, id)): Path<(String, String)>,
    Json(body): Json<CreateProblemRequest>,
) -> impl IntoResponse {
    let problem: Problem = body.into();
    let pool = state.rw_pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        crate::db::problems::update_problem(&pool, &source, &id, &problem)
    })
    .await;

    match result {
        Ok(Ok(n)) if n > 0 => StatusCode::OK.into_response(),
        Ok(Ok(_)) => ProblemDetail::not_found("problem not found").into_response(),
        Ok(Err(e)) => ProblemDetail::internal(format!("database error: {}", e)).into_response(),
        Err(_) => ProblemDetail::internal("task join error").into_response(),
    }
}

pub async fn delete_problem(
    State(state): State<Arc<AppState>>,
    Path((source, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let pool = state.rw_pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        crate::db::problems::delete_problem(&pool, &source, &id)
    })
    .await;

    match result {
        Ok(Ok(true)) => StatusCode::NO_CONTENT.into_response(),
        Ok(Ok(false)) => ProblemDetail::not_found("problem not found").into_response(),
        Ok(Err(e)) => ProblemDetail::internal(format!("database error: {}", e)).into_response(),
        Err(_) => ProblemDetail::internal("task join error").into_response(),
    }
}

pub async fn get_problems_list(
    State(state): State<Arc<AppState>>,
    Path(source): Path<String>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    if !VALID_SOURCES.contains(&source.as_str()) {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }
    if let Err(e) = validate_list_query(&query) {
        return ProblemDetail::bad_request(e).into_response();
    }

    let pool = state.ro_pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let tags: Option<Vec<&str>> = query.tags.as_ref().map(|t| {
            t.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        });

        let params = crate::db::problems::ListParams {
            source: &source,
            page: query.page.unwrap_or(1),
            per_page: query.per_page.unwrap_or(20),
            difficulty: query.difficulty.as_deref(),
            tags,
            search: query.search.as_deref(),
            sort_by: query.sort_by.as_deref(),
            sort_order: query.sort_order.as_deref(),
            tag_mode: query.tag_mode.as_deref().unwrap_or("any"),
            rating_min: query.rating_min,
            rating_max: query.rating_max,
        };
        crate::db::problems::list_problems(&pool, &params)
    })
    .await;

    match result {
        Ok(Some(r)) => Json(ListResponse {
            data: r.data,
            meta: ListMeta {
                total: r.total,
                page: r.page,
                per_page: r.per_page,
                total_pages: r.total_pages,
            },
        })
        .into_response(),
        Ok(None) | Err(_) => ProblemDetail::internal("database error").into_response(),
    }
}

pub async fn get_tags_list(
    State(state): State<Arc<AppState>>,
    Path(source): Path<String>,
) -> impl IntoResponse {
    if !VALID_SOURCES.contains(&source.as_str()) {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }

    let pool = state.ro_pool.clone();
    let result =
        tokio::task::spawn_blocking(move || crate::db::problems::list_tags(&pool, &source))
            .await
            .unwrap_or(None);

    match result {
        Some(tags) => Json(tags).into_response(),
        None => ProblemDetail::internal("database error").into_response(),
    }
}

pub async fn get_problem_detail(
    State(state): State<Arc<AppState>>,
    Path((source, id)): Path<(String, String)>,
) -> impl IntoResponse {
    if !VALID_SOURCES.contains(&source.as_str()) {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }

    let pool = state.ro_pool.clone();
    let result =
        tokio::task::spawn_blocking(move || crate::db::problems::get_problem(&pool, &source, &id))
            .await;

    match result {
        Ok(Some(problem)) => Json(problem).into_response(),
        Ok(None) => ProblemDetail::not_found("problem not found").into_response(),
        Err(_) => ProblemDetail::internal("database error").into_response(),
    }
}

// Token management

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub label: Option<String>,
}

pub async fn list_tokens(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = state.rw_pool.clone();
    let tokens = tokio::task::spawn_blocking(move || crate::db::tokens::list_tokens(&pool))
        .await
        .unwrap_or_default();

    Json(tokens).into_response()
}

pub async fn create_token(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateTokenRequest>,
) -> impl IntoResponse {
    let pool = state.rw_pool.clone();
    let label = body.label;

    let result = tokio::task::spawn_blocking(move || {
        crate::db::tokens::create_token(&pool, label.as_deref())
    })
    .await
    .unwrap_or(None);

    match result {
        Some(token) => (StatusCode::CREATED, Json(token)).into_response(),
        None => ProblemDetail::internal("failed to create token").into_response(),
    }
}

pub async fn revoke_token(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    let pool = state.rw_pool.clone();

    let result =
        tokio::task::spawn_blocking(move || crate::db::tokens::revoke_token(&pool, &token))
            .await
            .unwrap_or(None);

    match result {
        Some(true) => StatusCode::NO_CONTENT.into_response(),
        Some(false) => ProblemDetail::not_found("token not found").into_response(),
        None => ProblemDetail::internal("database error").into_response(),
    }
}

// Crawler

#[derive(Deserialize)]
pub struct TriggerCrawlerRequest {
    pub source: String,
    #[serde(default)]
    pub args: Vec<String>,
}

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
    if let Err(err) =
        crate::utils::persist_job_metadata(&artifact_paths, JobArtifactMetadata::from(&job)).await
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

    Json(serde_json::json!({
        "running": !running_jobs.is_empty(),
        "running_jobs": running_jobs,
        "history": history_vec,
    }))
    .into_response()
}

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

// Login / Logout

#[derive(Deserialize)]
pub struct LoginForm {
    pub secret: String,
}

pub async fn login_submit(
    Extension(admin_secret): Extension<AdminSecret>,
    Extension(sessions): Extension<AdminSessions>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if admin_secret.0.is_empty() || form.secret != admin_secret.0 {
        return super::pages::login_page_with_error("Invalid admin secret").into_response();
    }

    let token: String = {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    };
    let expires_at = chrono::Utc::now().timestamp() + 28800;

    sessions.0.write().await.insert(token.clone(), expires_at);

    let cookie = format!(
        "oj_admin_session={}; Path=/admin; HttpOnly; SameSite=Lax; Max-Age=28800",
        token
    );

    (
        StatusCode::SEE_OTHER,
        [("location", "/admin/"), ("set-cookie", &cookie)],
    )
        .into_response()
}

pub async fn logout(
    Extension(sessions): Extension<AdminSessions>,
    request: axum::extract::Request,
) -> impl IntoResponse {
    if let Some(token) = crate::auth::extract_cookie(request.headers(), "oj_admin_session") {
        sessions.0.write().await.remove(token);
    }

    let cookie = "oj_admin_session=; Path=/admin; HttpOnly; SameSite=Lax; Max-Age=0";

    (
        StatusCode::SEE_OTHER,
        [("location", "/admin/login"), ("set-cookie", cookie)],
    )
        .into_response()
}

// Settings toggle

#[derive(Deserialize)]
pub struct TokenAuthSettingRequest {
    pub enabled: bool,
}

pub async fn get_token_auth_setting(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let enabled = state.token_auth_enabled.load(Ordering::Acquire);
    Json(serde_json::json!({ "enabled": enabled }))
}

pub async fn set_token_auth_setting(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TokenAuthSettingRequest>,
) -> impl IntoResponse {
    let pool = state.rw_pool.clone();
    let value = if body.enabled { "1" } else { "0" };

    let ok = tokio::task::spawn_blocking(move || {
        crate::db::settings::set_setting(&pool, "token_auth_enabled", value)
    })
    .await
    .unwrap_or(false);

    if ok {
        state
            .token_auth_enabled
            .store(body.enabled, Ordering::Release);
        Json(serde_json::json!({ "enabled": body.enabled })).into_response()
    } else {
        ProblemDetail::internal("failed to update setting").into_response()
    }
}

// Embeddings

#[derive(Deserialize)]
pub struct TriggerEmbeddingRequest {
    pub source: String,
    #[serde(default)]
    pub rebuild: bool,
    #[serde(default)]
    pub dry_run: bool,
    pub batch_size: Option<u16>,
    pub filter: Option<String>,
}

pub async fn embedding_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = state.ro_pool.clone();
    let stats =
        tokio::task::spawn_blocking(move || crate::db::embeddings::get_embedding_stats(&pool))
            .await
            .unwrap_or_default();

    Json(stats).into_response()
}

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
    if let Err(err) =
        crate::utils::persist_job_metadata(&artifact_paths, JobArtifactMetadata::from(&job)).await
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
            let progress = read_embedding_progress_json(&job.job_id)
                .await
                .unwrap_or_else(|| serde_json::json!({ "phase": "queued" }));
            Json(serde_json::json!({
                "running": true,
                "current_job": job,
                "progress": progress,
                "history": history_vec,
            }))
            .into_response()
        }
        Some(mut job) => {
            job.stdout = None;
            job.stderr = None;
            let progress = read_embedding_progress_json(&job.job_id)
                .await
                .unwrap_or_else(|| serde_json::json!({ "phase": "unknown" }));
            Json(serde_json::json!({
                "running": false,
                "last_job": job,
                "progress": progress,
                "history": history_vec,
            }))
            .into_response()
        }
        None => Json(serde_json::json!({
            "running": false,
            "last_job": null,
            "history": history_vec,
        }))
        .into_response(),
    }
}

async fn read_embedding_progress_json(job_id: &str) -> Option<serde_json::Value> {
    let paths = match crate::utils::canonical_job_artifact_paths(JobType::Embedding, job_id) {
        Ok(paths) => paths,
        Err(_) => return None,
    };
    match tokio::fs::read_to_string(&paths.progress).await {
        Ok(content) => Some(
            serde_json::from_str(&content)
                .unwrap_or_else(|_| serde_json::json!({ "phase": "unknown" })),
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            match tokio::fs::metadata(&paths.job_dir).await {
                Ok(metadata) if metadata.is_dir() => Some(serde_json::json!({ "phase": "queued" })),
                Ok(_) => None,
                Err(meta_err) if meta_err.kind() == std::io::ErrorKind::NotFound => None,
                Err(_) => Some(serde_json::json!({ "phase": "unknown" })),
            }
        }
        Err(_) => Some(serde_json::json!({ "phase": "unknown" })),
    }
}

async fn read_crawler_progress_json(job_id: &str) -> Option<serde_json::Value> {
    let paths = match crate::utils::canonical_job_artifact_paths(JobType::Crawler, job_id) {
        Ok(paths) => paths,
        Err(_) => return None,
    };
    match tokio::fs::read_to_string(&paths.progress).await {
        Ok(content) => Some(
            serde_json::from_str(&content)
                .unwrap_or_else(|_| serde_json::json!({ "phase": "unknown" })),
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            match tokio::fs::metadata(&paths.job_dir).await {
                Ok(metadata) if metadata.is_dir() => Some(serde_json::json!({ "phase": "queued" })),
                Ok(_) => None,
                Err(meta_err) if meta_err.kind() == std::io::ErrorKind::NotFound => None,
                Err(_) => Some(serde_json::json!({ "phase": "unknown" })),
            }
        }
        Err(_) => Some(serde_json::json!({ "phase": "unknown" })),
    }
}

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

pub async fn crawler_progress(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if uuid::Uuid::parse_str(&job_id).is_err() {
        return ProblemDetail::bad_request("invalid job_id").into_response();
    }

    if let Some(progress) = read_crawler_progress_json(&job_id).await {
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

pub async fn embedding_progress(Path(job_id): Path<String>) -> impl IntoResponse {
    if uuid::Uuid::parse_str(&job_id).is_err() {
        return ProblemDetail::bad_request("invalid job_id").into_response();
    }

    match read_embedding_progress_json(&job_id).await {
        Some(progress) => Json(progress).into_response(),
        None => ProblemDetail::not_found("embedding progress not found").into_response(),
    }
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use std::collections::{HashMap, HashSet, VecDeque};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use axum::body::to_bytes;
    use axum::extract::{Path, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::Json;
    use rand::Rng;
    use tokio::sync::{Notify, RwLock, Semaphore};

    use super::{
        apply_terminal_update, collect_running_job_keys, embedding_launch_allowed,
        embedding_trigger_conflicts, finalize_owned_manual_crawler_job,
        manual_crawler_launch_allowed, manual_trigger_conflicts, persist_crawler_terminal_progress,
        take_manual_crawler_pid, take_owned_manual_crawler_pid, with_owned_manual_crawler_job,
        TriggerCrawlerRequest, TriggerEmbeddingRequest,
    };
    use crate::config::Config;
    use crate::models::{
        daily_fallback_crawler_runtime_key, manual_crawler_runtime_key, ActiveCrawlerPid,
        CrawlerJob, CrawlerPhase, CrawlerProgress, CrawlerStatus, CrawlerTrigger,
        DailyFallbackEntry, EmbeddingJob, JobType,
    };
    use crate::utils::CapturedOutput;
    use crate::AppState;

    fn test_state() -> Arc<AppState> {
        let config = Config::default();
        Arc::new(AppState {
            ro_pool: crate::db::create_ro_pool(":memory:", 1, config.database.busy_timeout_ms),
            rw_pool: crate::db::create_rw_pool(":memory:", 1, config.database.busy_timeout_ms),
            config,
            crawler_jobs: tokio::sync::Mutex::new(HashMap::new()),
            manual_crawler_guard: tokio::sync::Mutex::new(None),
            crawler_history: tokio::sync::Mutex::new(VecDeque::new()),
            embedding_lock: tokio::sync::Mutex::new(None),
            embedding_launch_guard: tokio::sync::Mutex::new(None),
            embedding_history: tokio::sync::Mutex::new(VecDeque::new()),
            active_crawler_pids: tokio::sync::Mutex::new(HashMap::new()),
            active_embedding_pid: tokio::sync::Mutex::new(None),
            daily_fallback: tokio::sync::Mutex::new(HashMap::new()),
            retained_refresh: tokio::sync::Mutex::new(crate::utils::RetainedRefreshState::default()),
            embed_semaphore: Semaphore::new(1),
            token_auth_enabled: Arc::new(AtomicBool::new(true)),
            admin_sessions: Arc::new(RwLock::new(HashMap::new())),
            config_path: None,
        })
    }

    fn crawler_job(job_id: &str, trigger: CrawlerTrigger, status: CrawlerStatus) -> CrawlerJob {
        CrawlerJob {
            job_id: job_id.to_string(),
            source: "leetcode".to_string(),
            args: vec![],
            trigger,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status,
            stdout: None,
            stderr: None,
        }
    }

    #[test]
    fn apply_terminal_update_preserves_cancelled_crawler_and_keeps_tail() {
        let mut job = CrawlerJob {
            job_id: "job-1".to_string(),
            source: "leetcode".to_string(),
            args: vec![],
            trigger: CrawlerTrigger::Admin,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: Some("2026-03-14T00:01:00Z".to_string()),
            status: CrawlerStatus::Cancelled,
            stdout: None,
            stderr: None,
        };

        apply_terminal_update(
            &mut job,
            None,
            Ok(CapturedOutput {
                stdout: b"crawler stdout".to_vec(),
                stderr: b"crawler stderr".to_vec(),
            }),
            "crawler",
        );

        assert_eq!(job.status, CrawlerStatus::Cancelled);
        assert_eq!(job.stdout.as_deref(), Some("crawler stdout"));
        assert_eq!(job.stderr.as_deref(), Some("crawler stderr"));
    }

    #[test]
    fn apply_terminal_update_marks_embedding_timed_out_and_keeps_tail() {
        let mut job = EmbeddingJob {
            job_id: "job-2".to_string(),
            source: "all".to_string(),
            args: vec![],
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };

        apply_terminal_update(
            &mut job,
            Some(CrawlerStatus::TimedOut),
            Ok(CapturedOutput {
                stdout: b"embedding stdout".to_vec(),
                stderr: b"embedding stderr".to_vec(),
            }),
            "embedding",
        );

        assert_eq!(job.status, CrawlerStatus::TimedOut);
        assert_eq!(job.stdout.as_deref(), Some("embedding stdout"));
        assert_eq!(job.stderr.as_deref(), Some("embedding stderr"));
    }

    #[test]
    fn apply_terminal_update_does_not_regress_existing_terminal_status() {
        let mut job = CrawlerJob {
            job_id: "job-3".to_string(),
            source: "leetcode".to_string(),
            args: vec![],
            trigger: CrawlerTrigger::Admin,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: Some("2026-03-14T00:01:00Z".to_string()),
            status: CrawlerStatus::Cancelled,
            stdout: None,
            stderr: None,
        };

        apply_terminal_update(
            &mut job,
            Some(CrawlerStatus::Completed),
            Ok(CapturedOutput {
                stdout: b"late stdout".to_vec(),
                stderr: b"late stderr".to_vec(),
            }),
            "crawler",
        );

        assert_eq!(job.status, CrawlerStatus::Cancelled);
        assert_eq!(job.stdout.as_deref(), Some("late stdout"));
        assert_eq!(job.stderr.as_deref(), Some("late stderr"));
    }

    #[test]
    fn embedding_launch_allowed_only_for_owned_running_job() {
        let running = EmbeddingJob {
            job_id: "embed-1".to_string(),
            source: "all".to_string(),
            args: vec![],
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };
        let cancelled = EmbeddingJob {
            status: CrawlerStatus::Cancelled,
            finished_at: Some("2026-03-14T00:01:00Z".to_string()),
            ..running.clone()
        };

        assert!(!embedding_trigger_conflicts(&None));
        assert!(embedding_trigger_conflicts(&Some("embed-1".to_string())));
        assert!(embedding_launch_allowed(
            &Some("embed-1".to_string()),
            Some(&running),
            "embed-1"
        ));
        assert!(!embedding_launch_allowed(
            &Some("embed-1".to_string()),
            Some(&running),
            "embed-2"
        ));
        assert!(!embedding_launch_allowed(
            &Some("embed-1".to_string()),
            Some(&cancelled),
            "embed-1"
        ));
        assert!(!embedding_launch_allowed(&None, Some(&running), "embed-1"));
        assert!(!embedding_launch_allowed(
            &Some("embed-1".to_string()),
            None,
            "embed-1"
        ));
    }

    #[test]
    fn collect_running_job_keys_includes_all_running_job_types() {
        let manual_job = CrawlerJob {
            job_id: "crawler-running".to_string(),
            source: "leetcode".to_string(),
            args: vec![],
            trigger: CrawlerTrigger::Admin,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };
        let daily_job = CrawlerJob {
            job_id: "daily-running".to_string(),
            source: "leetcode".to_string(),
            args: vec![
                "--daily".to_string(),
                "--domain".to_string(),
                "com".to_string(),
            ],
            trigger: CrawlerTrigger::DailyFallback,
            started_at: "2026-03-14T00:02:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };
        let completed_daily_job = CrawlerJob {
            status: CrawlerStatus::Completed,
            ..daily_job.clone()
        };
        let embedding_job = EmbeddingJob {
            job_id: "embedding-running".to_string(),
            source: "all".to_string(),
            args: vec![],
            started_at: "2026-03-14T00:01:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };
        let crawler_jobs = HashMap::from([
            (manual_crawler_runtime_key().to_string(), manual_job),
            (
                daily_fallback_crawler_runtime_key("com", "2026-03-14"),
                daily_job,
            ),
            (
                daily_fallback_crawler_runtime_key("com", "2026-03-13"),
                completed_daily_job,
            ),
        ]);

        let active = collect_running_job_keys(crawler_jobs.values(), Some(&embedding_job));

        assert_eq!(
            active,
            HashSet::from([
                (JobType::Crawler, "crawler-running".to_string()),
                (JobType::Crawler, "daily-running".to_string()),
                (JobType::Embedding, "embedding-running".to_string()),
            ])
        );
    }

    #[test]
    fn manual_trigger_conflicts_only_when_manual_guard_is_present() {
        assert!(!manual_trigger_conflicts(&None));
        assert!(manual_trigger_conflicts(&Some(
            "manual-running".to_string()
        )));
    }

    #[test]
    fn take_manual_crawler_pid_only_removes_manual_slot() {
        let daily_key = daily_fallback_crawler_runtime_key("com", "2026-03-14");
        let mut pids = HashMap::from([
            (
                manual_crawler_runtime_key().to_string(),
                ActiveCrawlerPid {
                    job_id: "manual-job".to_string(),
                    pid: 101_u32,
                },
            ),
            (
                daily_key.clone(),
                ActiveCrawlerPid {
                    job_id: "daily-job".to_string(),
                    pid: 202_u32,
                },
            ),
        ]);

        assert_eq!(take_manual_crawler_pid(&mut pids), Some(101_u32));
        assert_eq!(
            pids.get(&daily_key),
            Some(&ActiveCrawlerPid {
                job_id: "daily-job".to_string(),
                pid: 202_u32,
            })
        );
        assert_eq!(take_manual_crawler_pid(&mut pids), None);
    }

    #[test]
    fn take_owned_manual_crawler_pid_requires_matching_manual_job() {
        let manual_key = manual_crawler_runtime_key();
        let mut pids = HashMap::from([(
            manual_key.to_string(),
            ActiveCrawlerPid {
                job_id: "new-job".to_string(),
                pid: 303_u32,
            },
        )]);

        assert_eq!(take_owned_manual_crawler_pid(&mut pids, "old-job"), None);
        assert_eq!(
            pids.get(manual_key),
            Some(&ActiveCrawlerPid {
                job_id: "new-job".to_string(),
                pid: 303_u32,
            })
        );
        assert_eq!(
            take_owned_manual_crawler_pid(&mut pids, "new-job"),
            Some(303_u32)
        );
        assert_eq!(pids.get(manual_key), None);
    }

    #[test]
    fn with_owned_manual_crawler_job_ignores_reused_manual_slot() {
        let manual_key = manual_crawler_runtime_key().to_string();
        let mut crawler_jobs = HashMap::from([(
            manual_key.clone(),
            crawler_job("new-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
        )]);

        let updated = with_owned_manual_crawler_job(&mut crawler_jobs, "old-job", |job| {
            job.status = CrawlerStatus::Failed;
            job.finished_at = Some("2026-03-15T00:00:00Z".to_string());
            job.clone()
        });

        assert!(updated.is_none());
        let job = crawler_jobs.get(&manual_key).unwrap();
        assert_eq!(job.job_id, "new-job");
        assert_eq!(job.status, CrawlerStatus::Running);
        assert!(job.finished_at.is_none());
    }

    #[test]
    fn finalize_owned_manual_crawler_job_ignores_reused_manual_slot() {
        let manual_key = manual_crawler_runtime_key().to_string();
        let mut crawler_jobs = HashMap::from([(
            manual_key.clone(),
            crawler_job("new-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
        )]);

        let finished_job = finalize_owned_manual_crawler_job(
            &mut crawler_jobs,
            "old-job",
            Some(CrawlerStatus::Failed),
            Ok(CapturedOutput {
                stdout: b"old stdout".to_vec(),
                stderr: b"old stderr".to_vec(),
            }),
        );

        assert!(finished_job.is_none());
        let job = crawler_jobs.get(&manual_key).unwrap();
        assert_eq!(job.job_id, "new-job");
        assert_eq!(job.status, CrawlerStatus::Running);
        assert!(job.finished_at.is_none());
        assert!(job.stdout.is_none());
        assert!(job.stderr.is_none());
    }

    #[test]
    fn manual_crawler_launch_allowed_only_for_owned_running_job() {
        let running_job = crawler_job(
            "manual-running",
            CrawlerTrigger::Admin,
            CrawlerStatus::Running,
        );
        let cancelled_job = crawler_job(
            "manual-running",
            CrawlerTrigger::Admin,
            CrawlerStatus::Cancelled,
        );

        assert!(manual_crawler_launch_allowed(
            &Some("manual-running".to_string()),
            Some(&running_job),
            "manual-running",
        ));
        assert!(!manual_crawler_launch_allowed(
            &Some("manual-running".to_string()),
            Some(&cancelled_job),
            "manual-running",
        ));
        assert!(!manual_crawler_launch_allowed(
            &Some("other-job".to_string()),
            Some(&running_job),
            "manual-running",
        ));
        assert!(!manual_crawler_launch_allowed(
            &Some("manual-running".to_string()),
            None,
            "manual-running",
        ));
    }

    #[tokio::test]
    async fn trigger_crawler_returns_conflict_when_manual_job_is_reserved() {
        let state = test_state();
        *state.manual_crawler_guard.lock().await = Some("manual-running".to_string());

        let response = super::trigger_crawler(
            State(state),
            Json(TriggerCrawlerRequest {
                source: "leetcode".to_string(),
                args: vec![],
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn cancel_crawler_clears_manual_guard_before_pid_registration() {
        let state = test_state();
        let runtime_key = manual_crawler_runtime_key().to_string();
        *state.manual_crawler_guard.lock().await = Some("manual-job".to_string());
        state.crawler_jobs.lock().await.insert(
            runtime_key,
            crawler_job("manual-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
        );

        let response = super::cancel_crawler(State(state.clone()))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(*state.manual_crawler_guard.lock().await, None);
        let crawler_jobs = state.crawler_jobs.lock().await;
        let job = crawler_jobs.get(manual_crawler_runtime_key()).unwrap();
        assert_eq!(job.status, CrawlerStatus::Cancelled);
        assert!(job.finished_at.is_some());
    }

    #[tokio::test]
    async fn cancel_crawler_waits_for_manual_launch_critical_section() {
        let state = test_state();
        let runtime_key = manual_crawler_runtime_key().to_string();
        *state.manual_crawler_guard.lock().await = Some("manual-job".to_string());
        state.crawler_jobs.lock().await.insert(
            runtime_key,
            crawler_job("manual-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
        );

        let launch_guard = state.manual_crawler_guard.lock().await;
        let cancel_state = state.clone();
        let cancel_task = tokio::spawn(async move {
            super::cancel_crawler(State(cancel_state))
                .await
                .into_response()
                .status()
        });

        tokio::task::yield_now().await;
        assert!(!cancel_task.is_finished());

        drop(launch_guard);

        let status = cancel_task.await.unwrap();
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn cancel_embedding_waits_for_launch_critical_section() {
        let state = test_state();
        *state.embedding_lock.lock().await = Some(EmbeddingJob {
            job_id: "embed-job".to_string(),
            source: "all".to_string(),
            args: vec![],
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        });
        *state.embedding_launch_guard.lock().await = Some("embed-job".to_string());

        let launch_guard = state.embedding_launch_guard.lock().await;
        let cancel_state = state.clone();
        let cancel_task = tokio::spawn(async move {
            super::cancel_embedding(State(cancel_state))
                .await
                .into_response()
                .status()
        });

        tokio::task::yield_now().await;
        assert!(!cancel_task.is_finished());

        drop(launch_guard);

        let status = cancel_task.await.unwrap();
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn cancel_crawler_persists_cancelled_progress_before_pid_registration() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let state = test_state();
        let runtime_key = manual_crawler_runtime_key().to_string();
        let job_id = uuid::Uuid::new_v4().to_string();
        let job = CrawlerJob {
            job_id: job_id.clone(),
            ..crawler_job(&job_id, CrawlerTrigger::Admin, CrawlerStatus::Running)
        };
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
        crate::utils::persist_job_metadata(&paths, crate::models::JobArtifactMetadata::from(&job))
            .await
            .unwrap();

        *state.manual_crawler_guard.lock().await = Some(job_id.clone());
        state.crawler_jobs.lock().await.insert(runtime_key, job);

        let response = super::cancel_crawler(State(state.clone()))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let progress: CrawlerProgress = serde_json::from_str(&content).unwrap();
        assert_eq!(progress.phase, CrawlerPhase::Cancelled);
        assert!(progress.metadata.is_some());

        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[tokio::test]
    async fn cancel_crawler_preserves_cancelled_job_in_history_when_manual_slot_is_reused() {
        let state = test_state();
        let runtime_key = manual_crawler_runtime_key().to_string();
        *state.manual_crawler_guard.lock().await = Some("old-job".to_string());
        state.crawler_jobs.lock().await.insert(
            runtime_key.clone(),
            crawler_job("old-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
        );

        let response = super::cancel_crawler(State(state.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        state.crawler_jobs.lock().await.insert(
            runtime_key.clone(),
            crawler_job("new-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
        );

        let finished_job = {
            let mut crawler_jobs = state.crawler_jobs.lock().await;
            finalize_owned_manual_crawler_job(
                &mut crawler_jobs,
                "old-job",
                None,
                Ok(CapturedOutput {
                    stdout: b"old stdout".to_vec(),
                    stderr: b"old stderr".to_vec(),
                }),
            )
        };

        assert!(finished_job.is_none());

        let crawler_jobs = state.crawler_jobs.lock().await;
        let new_job = crawler_jobs.get(&runtime_key).unwrap();
        assert_eq!(new_job.job_id, "new-job");
        assert_eq!(new_job.status, CrawlerStatus::Running);
        drop(crawler_jobs);

        let history = state.crawler_history.lock().await;
        let cancelled_job = history.iter().find(|job| job.job_id == "old-job");
        assert!(cancelled_job.is_some());
        let cancelled_job = cancelled_job.unwrap();
        assert_eq!(cancelled_job.status, CrawlerStatus::Cancelled);
        assert!(cancelled_job.finished_at.is_some());
    }

    #[cfg(unix)]
    struct JobArtifactsRootBlocker {
        root: std::path::PathBuf,
        backup: Option<std::path::PathBuf>,
        _root_lock: std::sync::MutexGuard<'static, ()>,
    }

    #[cfg(unix)]
    impl JobArtifactsRootBlocker {
        async fn install() -> Self {
            let root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
                .lock()
                .unwrap_or_else(|err| err.into_inner());
            let root = std::env::current_dir()
                .unwrap()
                .join(crate::models::JOB_ARTIFACTS_ROOT);
            let backup = if tokio::fs::metadata(&root).await.is_ok() {
                let backup = std::env::temp_dir().join(format!(
                    "oj-api-rs-job-artifacts-backup-{}-{}",
                    std::process::id(),
                    rand::thread_rng().r#gen::<u64>()
                ));
                tokio::fs::rename(&root, &backup).await.unwrap();
                Some(backup)
            } else {
                None
            };
            if let Some(parent) = root.parent() {
                tokio::fs::create_dir_all(parent).await.unwrap();
            }
            tokio::fs::write(&root, b"blocked").await.unwrap();

            Self {
                root,
                backup,
                _root_lock: root_lock,
            }
        }
    }

    #[cfg(unix)]
    impl Drop for JobArtifactsRootBlocker {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.root);
            if let Some(backup) = self.backup.as_ref() {
                let _ = std::fs::rename(backup, &self.root);
            }
        }
    }

    #[cfg(unix)]
    struct FakeUvGuard {
        root: std::path::PathBuf,
        original_path: Option<std::ffi::OsString>,
        _path_lock: std::sync::MutexGuard<'static, ()>,
    }

    #[cfg(unix)]
    impl FakeUvGuard {
        async fn install(script_body: &str) -> Self {
            let path_lock = crate::utils::TEST_PATH_MUTEX
                .lock()
                .unwrap_or_else(|err| err.into_inner());
            let root = std::env::temp_dir().join(format!(
                "oj-api-rs-fake-uv-{}-{}",
                std::process::id(),
                rand::thread_rng().r#gen::<u64>()
            ));
            tokio::fs::create_dir_all(&root).await.unwrap();
            let uv_path = root.join("uv");
            tokio::fs::write(&uv_path, script_body).await.unwrap();
            let mut perms = tokio::fs::metadata(&uv_path).await.unwrap().permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&uv_path, perms).await.unwrap();

            let original_path = std::env::var_os("PATH");
            let mut new_path = std::ffi::OsString::from(&root);
            if let Some(existing) = original_path.as_ref() {
                new_path.push(":");
                new_path.push(existing);
            }
            std::env::set_var("PATH", &new_path);

            Self {
                root,
                original_path,
                _path_lock: path_lock,
            }
        }
    }

    #[cfg(unix)]
    impl Drop for FakeUvGuard {
        fn drop(&mut self) {
            match self.original_path.as_ref() {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[cfg(unix)]
    async fn wait_for_manual_job_terminal(state: &Arc<AppState>, runtime_key: &str) {
        for _ in 0..100 {
            let finished = {
                let crawler_jobs = state.crawler_jobs.lock().await;
                crawler_jobs
                    .get(runtime_key)
                    .map(|job| job.finished_at.is_some())
                    .unwrap_or(false)
            };
            if finished {
                let (_, progress) = read_crawler_progress_from_state(state, runtime_key).await;
                if matches!(
                    progress.phase,
                    CrawlerPhase::Completed
                        | CrawlerPhase::Failed
                        | CrawlerPhase::Cancelled
                        | CrawlerPhase::TimedOut
                ) {
                    return;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("manual crawler did not reach terminal state in time");
    }

    #[cfg(unix)]
    async fn wait_for_embedding_job_terminal(state: &Arc<AppState>) {
        for _ in 0..100 {
            let done = {
                let lock = state.embedding_lock.lock().await;
                lock.as_ref()
                    .map(|job| job.finished_at.is_some())
                    .unwrap_or(false)
            };
            if done {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("embedding job did not reach terminal state in time");
    }

    #[cfg(unix)]
    async fn read_crawler_progress_from_state(
        state: &Arc<AppState>,
        runtime_key: &str,
    ) -> (String, CrawlerProgress) {
        let job_id = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            crawler_jobs.get(runtime_key).unwrap().job_id.clone()
        };
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
        let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let progress: CrawlerProgress = serde_json::from_str(&content).unwrap();
        (job_id, progress)
    }

    #[cfg(unix)]
    async fn wait_for_manual_job_progress_phase(
        state: &Arc<AppState>,
        runtime_key: &str,
        phase: CrawlerPhase,
    ) -> bool {
        for _ in 0..100 {
            let finished = {
                let crawler_jobs = state.crawler_jobs.lock().await;
                crawler_jobs
                    .get(runtime_key)
                    .and_then(|job| job.finished_at.as_ref())
                    .is_some()
            };
            let (_, progress) = read_crawler_progress_from_state(state, runtime_key).await;
            if progress.phase == phase {
                return true;
            }
            if finished {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        false
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trigger_crawler_injects_job_env_into_manual_subprocess() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let fake_uv = FakeUvGuard::install(
            "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\nprintf '%s\\n' \"$OJ_JOB_ID\" > \"$OJ_JOB_DIR/env-job-id.txt\"\nprintf '%s\\n' \"$OJ_JOB_TYPE\" > \"$OJ_JOB_DIR/env-job-type.txt\"\nprintf '%s\\n' \"$OJ_JOB_DIR\" > \"$OJ_JOB_DIR/env-job-dir.txt\"\nprintf '%s\\n' \"$OJ_PROGRESS_PATH\" > \"$OJ_JOB_DIR/env-progress-path.txt\"\nprintf '%s\\n' \"$OJ_PYTHON_LOG_PATH\" > \"$OJ_JOB_DIR/env-python-log-path.txt\"\nexit 0\n",
        )
        .await;
        let state = test_state();
        let runtime_key = manual_crawler_runtime_key().to_string();

        let response = super::trigger_crawler(
            State(state.clone()),
            Json(TriggerCrawlerRequest {
                source: "leetcode".to_string(),
                args: vec![],
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        wait_for_manual_job_terminal(&state, &runtime_key).await;

        let job_id = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            crawler_jobs.get(&runtime_key).unwrap().job_id.clone()
        };
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();

        let env_job_id = tokio::fs::read_to_string(paths.job_dir.join("env-job-id.txt"))
            .await
            .unwrap();
        let env_job_type = tokio::fs::read_to_string(paths.job_dir.join("env-job-type.txt"))
            .await
            .unwrap();
        let env_job_dir = tokio::fs::read_to_string(paths.job_dir.join("env-job-dir.txt"))
            .await
            .unwrap();
        let env_progress_path =
            tokio::fs::read_to_string(paths.job_dir.join("env-progress-path.txt"))
                .await
                .unwrap();
        let env_python_log_path =
            tokio::fs::read_to_string(paths.job_dir.join("env-python-log-path.txt"))
                .await
                .unwrap();

        assert_eq!(env_job_id.trim(), job_id);
        let expected_job_dir = std::env::current_dir().unwrap().join(&paths.job_dir);
        let expected_progress_path = std::env::current_dir().unwrap().join(&paths.progress);
        let expected_python_log_path = std::env::current_dir().unwrap().join(&paths.python_log);

        assert_eq!(env_job_type.trim(), JobType::Crawler.as_str());
        assert_eq!(env_job_dir.trim(), expected_job_dir.to_string_lossy());
        assert_eq!(
            env_progress_path.trim(),
            expected_progress_path.to_string_lossy()
        );
        assert_eq!(
            env_python_log_path.trim(),
            expected_python_log_path.to_string_lossy()
        );

        drop(fake_uv);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trigger_crawler_fails_before_spawn_when_metadata_persist_fails() {
        let root_blocker = JobArtifactsRootBlocker::install().await;
        let spawned_marker = std::env::temp_dir().join(format!(
            "oj-api-rs-manual-crawler-spawned-{}-{}",
            std::process::id(),
            rand::thread_rng().r#gen::<u64>()
        ));
        let _ = tokio::fs::remove_file(&spawned_marker).await;
        let fake_uv = FakeUvGuard::install(&format!(
            "#!/bin/sh\nprintf 'spawned\\n' > \"{}\"\nexit 0\n",
            spawned_marker.display()
        ))
        .await;
        let state = test_state();
        let runtime_key = manual_crawler_runtime_key().to_string();

        let response = super::trigger_crawler(
            State(state.clone()),
            Json(TriggerCrawlerRequest {
                source: "leetcode".to_string(),
                args: vec![],
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(*state.manual_crawler_guard.lock().await, None);
        assert!(state.active_crawler_pids.lock().await.is_empty());

        let job = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            crawler_jobs.get(&runtime_key).cloned().unwrap()
        };
        assert_eq!(job.status, CrawlerStatus::Failed);
        assert!(job.finished_at.is_some());

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(tokio::fs::metadata(&spawned_marker).await.is_err());
        let history = state.crawler_history.lock().await;
        assert!(history
            .iter()
            .any(|entry| entry.job_id == job.job_id && entry.status == CrawlerStatus::Failed));

        drop(fake_uv);
        let _ = tokio::fs::remove_file(&spawned_marker).await;
        drop(root_blocker);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trigger_crawler_persists_running_progress_phase_for_manual_job() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let fake_uv = FakeUvGuard::install(
            "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\nsleep 0.3\nexit 0\n",
        )
        .await;
        let state = test_state();
        let runtime_key = manual_crawler_runtime_key().to_string();

        let response = super::trigger_crawler(
            State(state.clone()),
            Json(TriggerCrawlerRequest {
                source: "leetcode".to_string(),
                args: vec![],
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        assert!(
            wait_for_manual_job_progress_phase(&state, &runtime_key, CrawlerPhase::Running).await
        );

        wait_for_manual_job_terminal(&state, &runtime_key).await;

        let (_, progress) = read_crawler_progress_from_state(&state, &runtime_key).await;
        assert_eq!(progress.phase, CrawlerPhase::Completed);
        assert!(progress.metadata.is_some());

        drop(fake_uv);
    }

    #[tokio::test]
    async fn persist_crawler_terminal_progress_preserves_existing_terminal_phase() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let job_id = uuid::Uuid::new_v4().to_string();
        let job = CrawlerJob {
            job_id: job_id.clone(),
            source: "leetcode".to_string(),
            args: vec![],
            trigger: CrawlerTrigger::Admin,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: Some("2026-03-14T00:01:00Z".to_string()),
            status: CrawlerStatus::Completed,
            stdout: None,
            stderr: None,
        };
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
        crate::utils::persist_job_metadata(&paths, crate::models::JobArtifactMetadata::from(&job))
            .await
            .unwrap();
        crate::utils::write_crawler_progress(
            &paths,
            &CrawlerProgress {
                phase: CrawlerPhase::Cancelled,
                message: Some("cancelled earlier".to_string()),
                updated_at: Some("2026-03-14T00:00:30Z".to_string()),
                metadata: None,
            },
        )
        .await
        .unwrap();

        persist_crawler_terminal_progress(&paths, &job).await;

        let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let progress: CrawlerProgress = serde_json::from_str(&content).unwrap();
        assert_eq!(progress.phase, CrawlerPhase::Cancelled);
        assert_eq!(progress.message.as_deref(), Some("cancelled earlier"));
        assert!(progress.metadata.is_some());

        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trigger_embedding_injects_job_env_into_subprocess() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let fake_uv = FakeUvGuard::install(
            "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\nprintf '%s\\n' \"$OJ_JOB_ID\" > \"$OJ_JOB_DIR/env-job-id.txt\"\nprintf '%s\\n' \"$OJ_JOB_TYPE\" > \"$OJ_JOB_DIR/env-job-type.txt\"\nprintf '%s\\n' \"$OJ_JOB_DIR\" > \"$OJ_JOB_DIR/env-job-dir.txt\"\nprintf '%s\\n' \"$OJ_PROGRESS_PATH\" > \"$OJ_JOB_DIR/env-progress-path.txt\"\nprintf '%s\\n' \"$OJ_PYTHON_LOG_PATH\" > \"$OJ_JOB_DIR/env-python-log-path.txt\"\nexit 0\n",
        )
        .await;
        let state = test_state();

        let response = super::trigger_embedding(
            State(state.clone()),
            Json(TriggerEmbeddingRequest {
                source: "all".to_string(),
                rebuild: false,
                dry_run: false,
                batch_size: None,
                filter: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        wait_for_embedding_job_terminal(&state).await;

        let job_id = {
            let lock = state.embedding_lock.lock().await;
            lock.as_ref().unwrap().job_id.clone()
        };
        let paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();

        let env_job_id = tokio::fs::read_to_string(paths.job_dir.join("env-job-id.txt"))
            .await
            .unwrap();
        let env_job_type = tokio::fs::read_to_string(paths.job_dir.join("env-job-type.txt"))
            .await
            .unwrap();
        let env_job_dir = tokio::fs::read_to_string(paths.job_dir.join("env-job-dir.txt"))
            .await
            .unwrap();
        let env_progress_path =
            tokio::fs::read_to_string(paths.job_dir.join("env-progress-path.txt"))
                .await
                .unwrap();
        let env_python_log_path =
            tokio::fs::read_to_string(paths.job_dir.join("env-python-log-path.txt"))
                .await
                .unwrap();

        let expected_job_dir = std::env::current_dir().unwrap().join(&paths.job_dir);
        let expected_progress_path = std::env::current_dir().unwrap().join(&paths.progress);
        let expected_python_log_path = std::env::current_dir().unwrap().join(&paths.python_log);

        assert_eq!(env_job_id.trim(), job_id);
        assert_eq!(env_job_type.trim(), JobType::Embedding.as_str());
        assert_eq!(env_job_dir.trim(), expected_job_dir.to_string_lossy());
        assert_eq!(
            env_progress_path.trim(),
            expected_progress_path.to_string_lossy()
        );
        assert_eq!(
            env_python_log_path.trim(),
            expected_python_log_path.to_string_lossy()
        );

        drop(fake_uv);
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[tokio::test]
    async fn embedding_status_reports_queued_when_progress_file_is_missing() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let state = test_state();
        let job_id = uuid::Uuid::new_v4().to_string();
        let paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

        *state.embedding_lock.lock().await = Some(EmbeddingJob {
            job_id: job_id.clone(),
            source: "all".to_string(),
            args: vec![
                "--source".to_string(),
                "all".to_string(),
                "--build".to_string(),
            ],
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        });

        let response = super::embedding_status(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["running"], serde_json::Value::Bool(true));
        assert_eq!(
            payload["progress"]["phase"],
            serde_json::Value::String("queued".to_string())
        );

        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[tokio::test]
    async fn embedding_output_returns_empty_strings_when_stream_files_are_missing() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let state = test_state();
        let job_id = uuid::Uuid::new_v4().to_string();
        let job = EmbeddingJob {
            job_id: job_id.clone(),
            source: "all".to_string(),
            args: vec![
                "--source".to_string(),
                "all".to_string(),
                "--build".to_string(),
            ],
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };
        let paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
        crate::utils::persist_job_metadata(&paths, crate::models::JobArtifactMetadata::from(&job))
            .await
            .unwrap();

        let response = super::embedding_output(State(state), Path(job_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["stdout"], serde_json::Value::String(String::new()));
        assert_eq!(payload["stderr"], serde_json::Value::String(String::new()));
        assert_eq!(
            payload["python_log"],
            serde_json::Value::String(String::new())
        );

        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[tokio::test]
    async fn embedding_output_returns_not_found_after_artifacts_are_removed() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let state = test_state();
        let job_id = uuid::Uuid::new_v4().to_string();
        let paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

        state
            .embedding_history
            .lock()
            .await
            .push_back(EmbeddingJob {
                job_id: job_id.clone(),
                source: "all".to_string(),
                args: vec![
                    "--source".to_string(),
                    "all".to_string(),
                    "--build".to_string(),
                ],
                started_at: "2026-03-14T00:00:00Z".to_string(),
                finished_at: Some("2026-03-14T00:01:00Z".to_string()),
                status: CrawlerStatus::Completed,
                stdout: Some("history stdout".to_string()),
                stderr: Some("history stderr".to_string()),
            });

        let response = super::embedding_output(State(state), Path(job_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trigger_embedding_writes_python_log_and_output_endpoint_includes_all_streams() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let fake_uv = FakeUvGuard::install(
            "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\npython3 - <<'PY'\nimport sys\nfrom utils.logger import get_core_logger\nlogger = get_core_logger()\nlogger.info('python logger ready')\nprint('stdout line')\nprint('stderr line', file=sys.stderr)\nPY\n",
        )
        .await;
        let state = test_state();

        let response = super::trigger_embedding(
            State(state.clone()),
            Json(TriggerEmbeddingRequest {
                source: "all".to_string(),
                rebuild: false,
                dry_run: false,
                batch_size: None,
                filter: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        wait_for_embedding_job_terminal(&state).await;

        let job_id = {
            let lock = state.embedding_lock.lock().await;
            lock.as_ref().unwrap().job_id.clone()
        };
        let paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
        let python_log = tokio::fs::read_to_string(&paths.python_log).await.unwrap();
        assert!(python_log.contains("python logger ready"));
        assert!(!python_log.contains('\u{1b}'));

        let output_response = super::embedding_output(State(state.clone()), Path(job_id))
            .await
            .into_response();
        assert_eq!(output_response.status(), StatusCode::OK);

        let body = to_bytes(output_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload["stdout"],
            serde_json::Value::String("stdout line\n".to_string())
        );
        assert_eq!(
            payload["stderr"],
            serde_json::Value::String("stderr line\n".to_string())
        );
        assert_eq!(payload["python_log"], serde_json::Value::String(python_log));

        drop(fake_uv);
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trigger_embedding_preserves_summary_and_writes_terminal_metadata() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let fake_uv = FakeUvGuard::install(
            "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\npython3 - <<'PY'\nimport json, os\nwith open(os.environ['OJ_PROGRESS_PATH'], 'w', encoding='utf-8') as fh:\n    json.dump({\n        'phase': 'embedding',\n        'rewrite_progress': {'done': 3, 'total': 4, 'skipped': 1},\n        'embed_progress': {'done': 2, 'total': 2},\n        'summary': {'succeeded': 2, 'skipped': {'rewrite_timeout': 1}, 'failed': {}, 'duration_secs': 1.2},\n        'started_at': '2026-03-14T00:00:00Z'\n    }, fh)\nPY\n",
        )
        .await;
        let state = test_state();

        let response = super::trigger_embedding(
            State(state.clone()),
            Json(TriggerEmbeddingRequest {
                source: "all".to_string(),
                rebuild: false,
                dry_run: false,
                batch_size: None,
                filter: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        wait_for_embedding_job_terminal(&state).await;

        let job_id = {
            let lock = state.embedding_lock.lock().await;
            lock.as_ref().unwrap().job_id.clone()
        };
        let paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
        let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let payload: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(
            payload["phase"],
            serde_json::Value::String("completed".to_string())
        );
        assert_eq!(
            payload["summary"]["succeeded"],
            serde_json::Value::Number(2.into())
        );
        assert_eq!(
            payload["metadata"]["job_id"],
            serde_json::Value::String(job_id)
        );
        assert!(payload["metadata"]["finished_at"].is_string());
        assert!(payload["updated_at"].is_string());

        drop(fake_uv);
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trigger_embedding_spawn_failure_persists_failed_progress() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let _path_lock = crate::utils::TEST_PATH_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", "");

        let state = test_state();
        let response = super::trigger_embedding(
            State(state.clone()),
            Json(TriggerEmbeddingRequest {
                source: "all".to_string(),
                rebuild: false,
                dry_run: false,
                batch_size: None,
                filter: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        wait_for_embedding_job_terminal(&state).await;

        match original_path.as_ref() {
            Some(path) => std::env::set_var("PATH", path),
            None => std::env::remove_var("PATH"),
        }

        let job_id = {
            let lock = state.embedding_lock.lock().await;
            let job = lock.as_ref().unwrap();
            assert_eq!(job.status, CrawlerStatus::Failed);
            job.job_id.clone()
        };

        let response = super::embedding_progress(Path(job_id.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload["phase"],
            serde_json::Value::String("failed".to_string())
        );
        assert!(payload["metadata"]["finished_at"].is_string());

        let paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trigger_embedding_fails_before_spawn_when_metadata_persist_fails() {
        let root_blocker = JobArtifactsRootBlocker::install().await;
        let spawned_marker = std::env::temp_dir().join(format!(
            "oj-api-rs-embedding-spawned-{}-{}",
            std::process::id(),
            rand::thread_rng().r#gen::<u64>()
        ));
        let _ = tokio::fs::remove_file(&spawned_marker).await;
        let fake_uv = FakeUvGuard::install(&format!(
            "#!/bin/sh\nprintf 'spawned\\n' > \"{}\"\nexit 0\n",
            spawned_marker.display()
        ))
        .await;
        let state = test_state();

        let response = super::trigger_embedding(
            State(state.clone()),
            Json(TriggerEmbeddingRequest {
                source: "all".to_string(),
                rebuild: false,
                dry_run: false,
                batch_size: None,
                filter: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(*state.embedding_launch_guard.lock().await, None);
        assert!(state.active_embedding_pid.lock().await.is_none());

        let job = {
            let lock = state.embedding_lock.lock().await;
            lock.as_ref().cloned().unwrap()
        };
        assert_eq!(job.status, CrawlerStatus::Failed);
        assert!(job.finished_at.is_some());

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(tokio::fs::metadata(&spawned_marker).await.is_err());
        let history = state.embedding_history.lock().await;
        assert!(history
            .iter()
            .any(|entry| entry.job_id == job.job_id && entry.status == CrawlerStatus::Failed));

        drop(fake_uv);
        let _ = tokio::fs::remove_file(&spawned_marker).await;
        drop(root_blocker);
    }

    #[test]
    fn embedding_finalize_ignores_reused_slot() {
        let original_job = EmbeddingJob {
            job_id: "old-job".to_string(),
            source: "all".to_string(),
            args: vec!["--source".to_string(), "all".to_string()],
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: Some("2026-03-14T00:01:00Z".to_string()),
            status: CrawlerStatus::Cancelled,
            stdout: None,
            stderr: None,
        };
        let mut reused_slot = Some(EmbeddingJob {
            job_id: "new-job".to_string(),
            source: "leetcode".to_string(),
            args: vec!["--source".to_string(), "leetcode".to_string()],
            started_at: "2026-03-14T00:02:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        });

        let finished = super::finalize_owned_embedding_job(
            &mut reused_slot,
            "old-job",
            None,
            Ok(CapturedOutput {
                stdout: b"old stdout".to_vec(),
                stderr: b"old stderr".to_vec(),
            }),
        );

        assert!(finished.is_none());
        let current = reused_slot.as_ref().unwrap();
        assert_eq!(current.job_id, "new-job");
        assert_eq!(current.status, CrawlerStatus::Running);
        assert!(current.finished_at.is_none());
        assert!(current.stdout.is_none());
        assert!(current.stderr.is_none());

        let mut owned_slot = Some(original_job.clone());
        let finished = super::finalize_owned_embedding_job(
            &mut owned_slot,
            "old-job",
            None,
            Ok(CapturedOutput {
                stdout: b"old stdout".to_vec(),
                stderr: b"old stderr".to_vec(),
            }),
        );

        let finished = finished.expect("owned embedding job should finalize");
        assert_eq!(finished.job_id, "old-job");
        assert_eq!(finished.status, CrawlerStatus::Cancelled);
        assert_eq!(finished.stdout.as_deref(), Some("old stdout"));
        assert_eq!(finished.stderr.as_deref(), Some("old stderr"));
    }

    #[tokio::test]
    async fn embedding_progress_reports_unknown_for_malformed_progress_json() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let job_id = uuid::Uuid::new_v4().to_string();
        let paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
        tokio::fs::create_dir_all(&paths.job_dir).await.unwrap();
        tokio::fs::write(&paths.progress, b"{not-json")
            .await
            .unwrap();

        let response = super::embedding_progress(Path(job_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload["phase"],
            serde_json::Value::String("unknown".to_string())
        );

        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[tokio::test]
    async fn crawler_status_projects_manual_job_while_daily_fallback_coexists() {
        let state = test_state();
        let manual_key = manual_crawler_runtime_key().to_string();
        let daily_key = daily_fallback_crawler_runtime_key("com", "2026-03-14");
        let manual_job = crawler_job("manual-job", CrawlerTrigger::Admin, CrawlerStatus::Running);
        let daily_job = CrawlerJob {
            args: vec![
                "--daily".to_string(),
                "--domain".to_string(),
                "com".to_string(),
            ],
            ..crawler_job(
                "daily-job",
                CrawlerTrigger::DailyFallback,
                CrawlerStatus::Running,
            )
        };

        {
            let mut crawler_jobs = state.crawler_jobs.lock().await;
            crawler_jobs.insert(manual_key, manual_job);
            crawler_jobs.insert(daily_key.clone(), daily_job);
        }
        state.daily_fallback.lock().await.insert(
            daily_key,
            DailyFallbackEntry {
                job_id: "daily-job".to_string(),
                status: CrawlerStatus::Running,
                started_at: tokio::time::Instant::now(),
                cooldown_until: None,
                notify: Arc::new(Notify::new()),
                completed: Arc::new(AtomicBool::new(false)),
                stdout: None,
                stderr: None,
            },
        );

        let response = super::crawler_status(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["running"], serde_json::Value::Bool(true));
        assert_eq!(payload["running_jobs"].as_array().map(Vec::len), Some(2));
        assert_eq!(payload["running_jobs"][0]["job_id"], "daily-job");
        assert_eq!(payload["running_jobs"][0]["trigger"], "daily_fallback");
        assert_eq!(payload["running_jobs"][1]["job_id"], "manual-job");
        assert_eq!(payload["running_jobs"][1]["trigger"], "admin");
        assert!(payload["history"].is_array());
    }

    #[tokio::test]
    async fn crawler_progress_returns_queued_for_running_job_without_progress_file() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let state = test_state();
        let runtime_key = manual_crawler_runtime_key().to_string();
        let job_id = uuid::Uuid::new_v4().to_string();
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

        state.crawler_jobs.lock().await.insert(
            runtime_key,
            CrawlerJob {
                job_id: job_id.clone(),
                ..crawler_job(&job_id, CrawlerTrigger::Admin, CrawlerStatus::Running)
            },
        );

        let response = super::crawler_progress(State(state), Path(job_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload["phase"],
            serde_json::Value::String("queued".to_string())
        );
    }

    #[tokio::test]
    async fn crawler_output_returns_not_found_after_artifacts_are_removed() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let state = test_state();
        let job_id = uuid::Uuid::new_v4().to_string();
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

        state.crawler_history.lock().await.push_back(CrawlerJob {
            job_id: job_id.clone(),
            source: "leetcode".to_string(),
            args: vec![],
            trigger: CrawlerTrigger::Admin,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: Some("2026-03-14T00:01:00Z".to_string()),
            status: CrawlerStatus::Completed,
            stdout: Some("history stdout".to_string()),
            stderr: Some("history stderr".to_string()),
        });

        let response = super::crawler_output(State(state), Path(job_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn embedding_progress_returns_not_found_for_missing_non_running_job() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let job_id = uuid::Uuid::new_v4().to_string();
        let paths =
            crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

        let response = super::embedding_progress(Path(job_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }
}
