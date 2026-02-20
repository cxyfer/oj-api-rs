use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::error::ProblemDetail;
use crate::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub difficulty: Option<String>,
    pub tags: Option<String>,
}

#[derive(Serialize)]
struct ListMeta {
    total: u32,
    page: u32,
    per_page: u32,
    total_pages: u32,
}

#[derive(Serialize)]
struct ListResponse<T: Serialize> {
    data: Vec<T>,
    meta: ListMeta,
}

const VALID_SOURCES: &[&str] = &["atcoder", "leetcode", "codeforces"];

pub async fn get_problem(
    State(state): State<Arc<AppState>>,
    Path((source, id)): Path<(String, String)>,
) -> impl IntoResponse {
    if !VALID_SOURCES.contains(&source.as_str()) {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }

    let pool = state.ro_pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        crate::db::problems::get_problem(&pool, &source, &id)
    })
    .await
    .unwrap_or(None);

    match result {
        Some(p) => Json(p).into_response(),
        None => ProblemDetail::not_found("problem not found").into_response(),
    }
}

pub async fn list_problems(
    State(state): State<Arc<AppState>>,
    Path(source): Path<String>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    if !VALID_SOURCES.contains(&source.as_str()) {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }

    let pool = state.ro_pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let tags: Option<Vec<&str>> = query
            .tags
            .as_ref()
            .map(|t| t.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect());

        let params = crate::db::problems::ListParams {
            source: &source,
            page: query.page.unwrap_or(1),
            per_page: query.per_page.unwrap_or(20),
            difficulty: query.difficulty.as_deref(),
            tags,
        };
        crate::db::problems::list_problems(&pool, &params)
    })
    .await
    .unwrap_or(None);

    match result {
        Some(r) => Json(ListResponse {
            data: r.data,
            meta: ListMeta {
                total: r.total,
                page: r.page,
                per_page: r.per_page,
                total_pages: r.total_pages,
            },
        })
        .into_response(),
        None => ProblemDetail::internal("database error").into_response(),
    }
}
