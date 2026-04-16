use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::error::ProblemDetail;
use crate::models::{ProblemRecord, ProblemSummary};
use crate::AppState;

#[derive(Serialize)]
pub(crate) struct ProblemDetailResponse {
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
    pub tags: Vec<String>,
    pub link: Option<String>,
    pub category: Option<String>,
    pub paid_only: Option<i32>,
    pub content: Option<String>,
    pub content_cn: Option<String>,
    pub similar_questions: Vec<ProblemSummary>,
}

pub(crate) fn build_problem_detail_response(
    pool: &crate::db::DbPool,
    record: ProblemRecord,
) -> ProblemDetailResponse {
    let similar_questions = crate::db::problems::resolve_similar_question_summaries(
        pool,
        &record.source,
        &record.similar_questions,
    );

    ProblemDetailResponse {
        id: record.id,
        source: record.source,
        slug: record.slug,
        title: record.title,
        title_cn: record.title_cn,
        difficulty: record.difficulty,
        ac_rate: record.ac_rate,
        rating: record.rating,
        contest: record.contest,
        problem_index: record.problem_index,
        tags: record.tags,
        link: record.link,
        category: record.category,
        paid_only: record.paid_only,
        content: record.content,
        content_cn: record.content_cn,
        similar_questions,
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub difficulty: Option<String>,
    pub tags: Option<String>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub tag_mode: Option<String>,
    pub rating_min: Option<f64>,
    pub rating_max: Option<f64>,
}

#[derive(Serialize)]
pub(crate) struct ListMeta {
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

#[derive(Serialize)]
pub(crate) struct ListResponse<T: Serialize> {
    pub data: Vec<T>,
    pub meta: ListMeta,
}

pub(crate) const VALID_SOURCES: &[&str] = &["atcoder", "leetcode", "codeforces", "luogu", "spoj"];
pub(crate) const VALID_SORT_BY: &[&str] = &["id", "difficulty", "rating", "ac_rate"];
pub(crate) const VALID_SORT_ORDER: &[&str] = &["asc", "desc"];
pub(crate) const VALID_TAG_MODES: &[&str] = &["any", "all"];

pub(crate) fn validate_list_query(query: &ListQuery) -> Result<(), String> {
    if let Some(ref s) = query.sort_by {
        if !VALID_SORT_BY.contains(&s.as_str()) {
            return Err(format!("invalid sort_by: {}", s));
        }
    }
    if let Some(ref s) = query.sort_order {
        if !VALID_SORT_ORDER.contains(&s.as_str()) {
            return Err(format!("invalid sort_order: {}", s));
        }
    }
    if let Some(ref s) = query.tag_mode {
        if !VALID_TAG_MODES.contains(&s.as_str()) {
            return Err(format!("invalid tag_mode: {}", s));
        }
    }
    if let (Some(min), Some(max)) = (query.rating_min, query.rating_max) {
        if min > max {
            return Err("rating_min must be <= rating_max".to_string());
        }
    }
    Ok(())
}

pub async fn get_problem(
    State(state): State<Arc<AppState>>,
    Path((source, id)): Path<(String, String)>,
) -> impl IntoResponse {
    if !VALID_SOURCES.contains(&source.as_str()) {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }

    let pool = state.ro_pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let record = crate::db::problems::get_problem_record(&pool, &source, &id)?;
        Some(build_problem_detail_response(&pool, record))
    })
    .await
    .unwrap_or(None);

    match result {
        Some(problem) => Json(problem).into_response(),
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

pub async fn list_tags(
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
