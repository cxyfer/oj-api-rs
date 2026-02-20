use std::sync::Arc;

use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use serde::Deserialize;

use crate::models::{ApiToken, ProblemSummary};
use crate::AppState;

#[derive(Template)]
#[template(path = "admin/index.html")]
struct IndexTemplate;

pub async fn index() -> impl IntoResponse {
    Html(IndexTemplate.render().unwrap_or_default())
}

#[derive(Deserialize)]
pub struct ProblemsPageQuery {
    pub source: Option<String>,
    pub page: Option<u32>,
}

#[derive(Template)]
#[template(path = "admin/problems.html")]
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
}

pub async fn tokens_page(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pool = state.rw_pool.clone();

    let tokens = tokio::task::spawn_blocking(move || {
        crate::db::tokens::list_tokens(&pool)
    })
    .await
    .unwrap_or_default();

    Html(
        TokensTemplate { tokens }
            .render()
            .unwrap_or_default(),
    )
    .into_response()
}
