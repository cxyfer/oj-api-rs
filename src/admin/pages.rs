use std::sync::atomic::Ordering;
use std::sync::Arc;

use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use serde::Deserialize;

use crate::models::{ApiToken, CrawlerJob, ProblemSummary};
use crate::AppState;

#[derive(Template)]
#[template(path = "admin/login.html")]
struct LoginTemplate {
    error: String,
}

pub async fn login_page() -> impl IntoResponse {
    Html(
        LoginTemplate {
            error: String::new(),
        }
        .render()
        .unwrap_or_default(),
    )
}

pub fn login_page_with_error(error: &str) -> Html<String> {
    Html(
        LoginTemplate {
            error: error.to_string(),
        }
        .render()
        .unwrap_or_default(),
    )
}

#[derive(Template)]
#[template(path = "admin/index.html")]
struct IndexTemplate {
    total_problems: u32,
    active_tokens: u32,
    token_auth_enabled: bool,
}

pub async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = state.ro_pool.clone();
    let total_problems = tokio::task::spawn_blocking(move || {
        let conn = pool.get().ok()?;
        conn.query_row("SELECT COUNT(*) FROM problems", [], |row| row.get::<_, u32>(0))
            .ok()
    })
    .await
    .unwrap_or(None)
    .unwrap_or(0);

    let pool = state.rw_pool.clone();
    let active_tokens = tokio::task::spawn_blocking(move || {
        let conn = pool.get().ok()?;
        conn.query_row(
            "SELECT COUNT(*) FROM api_tokens WHERE is_active = 1",
            [],
            |row| row.get::<_, u32>(0),
        )
        .ok()
    })
    .await
    .unwrap_or(None)
    .unwrap_or(0);

    let token_auth_enabled = state.token_auth_enabled.load(Ordering::Acquire);

    Html(
        IndexTemplate {
            total_problems,
            active_tokens,
            token_auth_enabled,
        }
        .render()
        .unwrap_or_default(),
    )
    .into_response()
}

#[derive(Deserialize)]
pub struct ProblemsPageQuery {
    pub source: Option<String>,
    pub page: Option<u32>,
}

#[derive(Template)]
#[template(path = "admin/problems.html")]
#[allow(dead_code)]
struct ProblemsTemplate {
    source: String,
    problems: Vec<ProblemSummary>,
    page: u32,
    total: u32,
    total_pages: u32,
}

pub async fn problems_page(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProblemsPageQuery>,
) -> impl IntoResponse {
    let source = query.source.unwrap_or_else(|| "leetcode".into());
    let page = query.page.unwrap_or(1);
    let pool = state.ro_pool.clone();
    let source_clone = source.clone();

    let result = tokio::task::spawn_blocking(move || {
        let params = crate::db::problems::ListParams {
            source: &source_clone,
            page,
            per_page: 50,
            difficulty: None,
            tags: None,
            search: None,
            sort_by: None,
            sort_order: None,
            tag_mode: "any",
            rating_min: None,
            rating_max: None,
        };
        crate::db::problems::list_problems(&pool, &params)
    })
    .await
    .unwrap_or(None);

    match result {
        Some(r) => Html(
            ProblemsTemplate {
                source,
                problems: r.data,
                page: r.page,
                total: r.total,
                total_pages: r.total_pages,
            }
            .render()
            .unwrap_or_default(),
        )
        .into_response(),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(Template)]
#[template(path = "admin/tokens.html")]
struct TokensTemplate {
    tokens: Vec<ApiToken>,
    token_auth_enabled: bool,
}

pub async fn tokens_page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = state.rw_pool.clone();

    let tokens = tokio::task::spawn_blocking(move || crate::db::tokens::list_tokens(&pool))
        .await
        .unwrap_or_default();

    let token_auth_enabled = state.token_auth_enabled.load(Ordering::Acquire);

    Html(
        TokensTemplate {
            tokens,
            token_auth_enabled,
        }
        .render()
        .unwrap_or_default(),
    )
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/crawlers.html")]
struct CrawlersTemplate {
    is_running: bool,
    current_job: Option<CrawlerJob>,
    history: Vec<CrawlerJob>,
}

pub async fn crawlers_page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let lock = state.crawler_lock.lock().await;
    let is_running = lock
        .as_ref()
        .map(|j| j.status == crate::models::CrawlerStatus::Running)
        .unwrap_or(false);
    let current_job = if is_running { lock.clone() } else { None };
    drop(lock);

    let history_lock = state.crawler_history.lock().await;
    let history: Vec<CrawlerJob> = history_lock.iter().rev().cloned().collect();
    drop(history_lock);

    Html(
        CrawlersTemplate {
            is_running,
            current_job,
            history,
        }
        .render()
        .unwrap_or_default(),
    )
    .into_response()
}
