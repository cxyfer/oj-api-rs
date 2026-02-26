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
    CrawlerJob, CrawlerSource, CrawlerStatus, CrawlerTrigger, EmbeddingJob, Problem,
};
use crate::AppState;

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

    let mut lock = state.crawler_lock.lock().await;

    if let Some(ref job) = *lock {
        if job.status == CrawlerStatus::Running {
            return ProblemDetail::conflict("a crawler is already running").into_response();
        }
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now().to_rfc3339();

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

    *lock = Some(job.clone());
    drop(lock);

    let script = source.script_name();
    let state_clone = state.clone();
    let timeout_secs = state
        .config
        .crawler
        .per_source_timeout
        .get(&body.source)
        .copied()
        .unwrap_or(state.config.crawler.timeout_secs);
    let job_id_clone = job_id.clone();

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

        let child = match crate::utils::spawn_with_pgid(cmd) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to spawn crawler: {}", e);
                let mut lock = state_clone.crawler_lock.lock().await;
                if let Some(ref mut job) = *lock {
                    job.status = CrawlerStatus::Failed;
                    job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                    let mut history = state_clone.crawler_history.lock().await;
                    if history.len() >= 50 {
                        history.pop_front();
                    }
                    history.push_back(job.clone());
                }
                return;
            }
        };

        let pid = child.id().expect("child should have a pid");
        *state_clone.active_crawler_pid.lock().await = Some(pid);

        let mut wait_task = tokio::spawn(async move { child.wait_with_output().await });
        let timed = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            &mut wait_task,
        )
        .await;

        let mut lock = state_clone.crawler_lock.lock().await;
        // Clear pid under job lock to close the cancel race window
        *state_clone.active_crawler_pid.lock().await = None;

        if let Some(ref mut job) = *lock {
            // Only update if not already set to Cancelled
            if job.status == CrawlerStatus::Running {
                job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                match timed {
                    Ok(Ok(Ok(output))) => {
                        // Write log files
                        if let Err(e) = tokio::fs::create_dir_all("scripts/logs").await {
                            tracing::warn!("failed to create scripts/logs: {}", e);
                        }
                        if !output.stdout.is_empty() {
                            if let Err(e) = tokio::fs::write(
                                format!("scripts/logs/{}.stdout.log", job_id_clone),
                                &output.stdout,
                            )
                            .await
                            {
                                tracing::warn!("failed to write stdout log: {}", e);
                            }
                        }
                        if !output.stderr.is_empty() {
                            if let Err(e) = tokio::fs::write(
                                format!("scripts/logs/{}.stderr.log", job_id_clone),
                                &output.stderr,
                            )
                            .await
                            {
                                tracing::warn!("failed to write stderr log: {}", e);
                            }
                        }

                        if output.status.success() {
                            job.status = CrawlerStatus::Completed;
                        } else {
                            job.status = CrawlerStatus::Failed;
                        }
                        job.set_output(output.stdout, output.stderr);
                    }
                    Ok(Ok(Err(e))) => {
                        tracing::error!("crawler error: {}", e);
                        job.status = CrawlerStatus::Failed;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("crawler join error: {}", e);
                        job.status = CrawlerStatus::Failed;
                    }
                    Err(_) => {
                        tracing::warn!("crawler job {} timed out", job_id_clone);
                        crate::utils::kill_pgid(pid);
                        let _ = wait_task.await;
                        job.status = CrawlerStatus::TimedOut;
                    }
                }
            }
            let mut history = state_clone.crawler_history.lock().await;
            if history.len() >= 50 {
                history.pop_front();
            }
            history.push_back(job.clone());
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "job_id": job_id })),
    )
        .into_response()
}

pub async fn cancel_crawler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut lock = state.crawler_lock.lock().await;
    if let Some(ref mut job) = *lock {
        if job.status == CrawlerStatus::Running {
            let mut pid_lock = state.active_crawler_pid.lock().await;
            if let Some(pid) = pid_lock.take() {
                crate::utils::kill_pgid(pid);
            }
            job.status = CrawlerStatus::Cancelled;
            job.finished_at = Some(chrono::Utc::now().to_rfc3339());

            return StatusCode::OK.into_response();
        }
    }

    ProblemDetail::conflict("no running crawler to cancel").into_response()
}

pub async fn crawler_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let lock = state.crawler_lock.lock().await;
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

    match &*lock {
        Some(job) if job.status == CrawlerStatus::Running => {
            let mut current = job.clone();
            current.stdout = None;
            current.stderr = None;
            Json(serde_json::json!({
                "running": true,
                "current_job": current,
                "history": history_vec,
            }))
            .into_response()
        }
        Some(job) => {
            let mut last = job.clone();
            last.stdout = None;
            last.stderr = None;
            Json(serde_json::json!({
                "running": false,
                "last_job": last,
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

pub async fn crawler_output(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if uuid::Uuid::parse_str(&job_id).is_err() {
        return ProblemDetail::bad_request("invalid job_id").into_response();
    }
    // Check in-memory history first
    let found_in_memory = {
        let history = state.crawler_history.lock().await;
        history.iter().find(|j| j.job_id == job_id).map(|job| {
            serde_json::json!({
                "stdout": job.stdout,
                "stderr": job.stderr,
            })
        })
    };

    if let Some(output) = found_in_memory {
        return Json(output).into_response();
    }

    // Fallback to files
    let stdout_path = format!("scripts/logs/{}.stdout.log", job_id);
    let stderr_path = format!("scripts/logs/{}.stderr.log", job_id);

    let stdout = tokio::fs::read_to_string(&stdout_path).await.ok();
    let stderr = tokio::fs::read_to_string(&stderr_path).await.ok();

    if stdout.is_none() && stderr.is_none() {
        return ProblemDetail::not_found("job output not found").into_response();
    }

    Json(serde_json::json!({
        "stdout": stdout,
        "stderr": stderr,
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

    let mut lock = state.embedding_lock.lock().await;
    if let Some(ref job) = *lock {
        if job.status == CrawlerStatus::Running {
            return ProblemDetail::conflict("an embedding job is already running").into_response();
        }
    }

    let job_id = uuid::Uuid::new_v4().to_string();
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
    drop(lock);

    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    let timeout_secs = state.config.embedding.batch_timeout_secs;

    tokio::spawn(async move {
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

        let child = match crate::utils::spawn_with_pgid(cmd) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to spawn embedding job: {}", e);
                let mut lock = state_clone.embedding_lock.lock().await;
                if let Some(ref mut job) = *lock {
                    job.status = CrawlerStatus::Failed;
                    job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                    let mut history = state_clone.embedding_history.lock().await;
                    if history.len() >= 50 {
                        history.pop_front();
                    }
                    history.push_back(job.clone());
                }
                return;
            }
        };

        let pid = child.id().expect("child should have a pid");
        *state_clone.active_embedding_pid.lock().await = Some(pid);

        let mut wait_task = tokio::spawn(async move { child.wait_with_output().await });
        let timed = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            &mut wait_task,
        )
        .await;

        let mut lock = state_clone.embedding_lock.lock().await;
        // Clear pid under job lock to close the cancel race window
        *state_clone.active_embedding_pid.lock().await = None;

        if let Some(ref mut job) = *lock {
            // Only update if not already set to Cancelled
            if job.status == CrawlerStatus::Running {
                job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                match timed {
                    Ok(Ok(Ok(output))) => {
                        if let Err(e) = tokio::fs::create_dir_all("scripts/logs").await {
                            tracing::warn!("failed to create scripts/logs: {}", e);
                        }
                        if !output.stdout.is_empty() {
                            if let Err(e) = tokio::fs::write(
                                format!("scripts/logs/{}.stdout.log", job_id_clone),
                                &output.stdout,
                            )
                            .await
                            {
                                tracing::warn!("failed to write stdout log: {}", e);
                            }
                        }
                        if !output.stderr.is_empty() {
                            if let Err(e) = tokio::fs::write(
                                format!("scripts/logs/{}.stderr.log", job_id_clone),
                                &output.stderr,
                            )
                            .await
                            {
                                tracing::warn!("failed to write stderr log: {}", e);
                            }
                        }

                        if output.status.success() {
                            job.status = CrawlerStatus::Completed;
                        } else {
                            job.status = CrawlerStatus::Failed;
                        }
                        job.set_output(output.stdout, output.stderr);
                    }
                    Ok(Ok(Err(e))) => {
                        tracing::error!("embedding job error: {}", e);
                        job.status = CrawlerStatus::Failed;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("embedding job join error: {}", e);
                        job.status = CrawlerStatus::Failed;
                    }
                    Err(_) => {
                        tracing::warn!("embedding job {} timed out", job_id_clone);
                        crate::utils::kill_pgid(pid);
                        let _ = wait_task.await;
                        job.status = CrawlerStatus::TimedOut;
                    }
                }
            }

            // Ensure progress JSON reflects final status (Python may not have written it)
            let final_phase = if job.status == CrawlerStatus::Completed {
                "completed"
            } else if job.status == CrawlerStatus::Cancelled {
                "cancelled"
            } else if job.status == CrawlerStatus::TimedOut {
                "timed_out"
            } else {
                "failed"
            };
            let progress_path = format!("scripts/logs/{}.progress.json", job_id_clone);
            let final_progress =
                if let Ok(content) = tokio::fs::read_to_string(&progress_path).await {
                    let mut prog: serde_json::Value =
                        serde_json::from_str(&content).unwrap_or_default();
                    prog["phase"] = serde_json::json!(final_phase);
                    prog
                } else {
                    serde_json::json!({"phase": final_phase})
                };
            if let Ok(json_str) = serde_json::to_string(&final_progress) {
                let _ = tokio::fs::write(&progress_path, json_str).await;
            }

            let mut history = state_clone.embedding_history.lock().await;
            if history.len() >= 50 {
                history.pop_front();
            }
            history.push_back(job.clone());
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "job_id": job_id })),
    )
        .into_response()
}

pub async fn cancel_embedding(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut lock = state.embedding_lock.lock().await;
    if let Some(ref mut job) = *lock {
        if job.status == CrawlerStatus::Running {
            let mut pid_lock = state.active_embedding_pid.lock().await;
            if let Some(pid) = pid_lock.take() {
                crate::utils::kill_pgid(pid);
            }
            job.status = CrawlerStatus::Cancelled;
            job.finished_at = Some(chrono::Utc::now().to_rfc3339());

            return StatusCode::OK.into_response();
        }
    }

    ProblemDetail::conflict("no running embedding job to cancel").into_response()
}

pub async fn embedding_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let lock = state.embedding_lock.lock().await;
    let history = state.embedding_history.lock().await;
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

    match &*lock {
        Some(job) if job.status == CrawlerStatus::Running => {
            let mut current = job.clone();
            current.stdout = None;
            current.stderr = None;
            let progress = read_progress_json(&job.job_id).await;
            Json(serde_json::json!({
                "running": true,
                "current_job": current,
                "progress": progress,
                "history": history_vec,
            }))
            .into_response()
        }
        Some(job) => {
            let mut last = job.clone();
            last.stdout = None;
            last.stderr = None;
            let progress = read_progress_json(&job.job_id).await;
            Json(serde_json::json!({
                "running": false,
                "last_job": last,
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

async fn read_progress_json(job_id: &str) -> serde_json::Value {
    let path = format!("scripts/logs/{}.progress.json", job_id);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content)
            .unwrap_or_else(|_| serde_json::json!({ "phase": "unknown" })),
        Err(_) => serde_json::json!({ "phase": "unknown" }),
    }
}

pub async fn embedding_output(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if uuid::Uuid::parse_str(&job_id).is_err() {
        return ProblemDetail::bad_request("invalid job_id").into_response();
    }

    let found_in_memory = {
        let history = state.embedding_history.lock().await;
        history.iter().find(|j| j.job_id == job_id).map(|job| {
            serde_json::json!({
                "stdout": job.stdout,
                "stderr": job.stderr,
            })
        })
    };

    if let Some(output) = found_in_memory {
        return Json(output).into_response();
    }

    let stdout_path = format!("scripts/logs/{}.stdout.log", job_id);
    let stderr_path = format!("scripts/logs/{}.stderr.log", job_id);

    let stdout = tokio::fs::read_to_string(&stdout_path).await.ok();
    let stderr = tokio::fs::read_to_string(&stderr_path).await.ok();

    if stdout.is_none() && stderr.is_none() {
        return ProblemDetail::not_found("job output not found").into_response();
    }

    Json(serde_json::json!({
        "stdout": stdout,
        "stderr": stderr,
    }))
    .into_response()
}

pub async fn embedding_progress(Path(job_id): Path<String>) -> impl IntoResponse {
    if uuid::Uuid::parse_str(&job_id).is_err() {
        return ProblemDetail::bad_request("invalid job_id").into_response();
    }

    Json(read_progress_json(&job_id).await).into_response()
}
