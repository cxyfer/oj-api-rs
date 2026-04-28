use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::error::ProblemDetail;
use crate::models::{ProblemRecord, ProblemSummary};
use crate::AppState;

#[derive(Serialize, Deserialize)]
pub(crate) struct ProblemDetailResponse {
    pub(crate) id: String,
    pub(crate) source: String,
    pub(crate) slug: String,
    pub(crate) title: Option<String>,
    pub(crate) title_cn: Option<String>,
    pub(crate) difficulty: Option<String>,
    pub(crate) ac_rate: Option<f64>,
    pub(crate) rating: Option<f64>,
    pub(crate) contest: Option<String>,
    pub(crate) problem_index: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) link: Option<String>,
    pub(crate) category: Option<String>,
    pub(crate) paid_only: Option<i32>,
    pub(crate) content: Option<String>,
    pub(crate) content_cn: Option<String>,
    pub(crate) similar_questions: Vec<ProblemSummary>,
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

#[derive(Serialize, Deserialize)]
pub(crate) struct ListMeta {
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ListResponse<T: Serialize> {
    pub data: Vec<T>,
    pub meta: ListMeta,
}

pub(crate) const VALID_SOURCES: &[&str] = &["atcoder", "leetcode", "codeforces", "luogu", "spoj"];

const MAX_BATCH_SIZE: usize = 50;

#[derive(Deserialize)]
pub struct BatchItem {
    pub source: String,
    pub id: String,
}

#[derive(Deserialize)]
pub struct BatchQuery {
    pub detail: Option<bool>,
}

#[derive(Serialize)]
pub(crate) struct BatchNotFoundItem {
    pub source: String,
    pub id: String,
}

#[derive(Serialize)]
pub(crate) struct BatchResponse<T: Serialize> {
    pub results: Vec<T>,
    pub not_found: Vec<BatchNotFoundItem>,
}
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

pub async fn batch_problems(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BatchQuery>,
    Json(items): Json<Vec<BatchItem>>,
) -> impl IntoResponse {
    if items.is_empty() {
        return ProblemDetail::bad_request("request body must not be empty").into_response();
    }
    if items.len() > MAX_BATCH_SIZE {
        return ProblemDetail::bad_request(format!(
            "batch size {} exceeds maximum of {}",
            items.len(),
            MAX_BATCH_SIZE
        ))
        .into_response();
    }
    for item in &items {
        if !VALID_SOURCES.contains(&item.source.as_str()) {
            return ProblemDetail::bad_request(format!("invalid source: {}", item.source))
                .into_response();
        }
    }

    let detail = query.detail.unwrap_or(false);
    let pool = state.ro_pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        if detail {
            let mut results = Vec::with_capacity(items.len());
            let mut not_found = Vec::with_capacity(items.len());
            for item in items {
                match crate::db::problems::get_problem_record_result(&pool, &item.source, &item.id)?
                {
                    Some(record) => results.push(build_problem_detail_response(&pool, record)),
                    None => not_found.push(BatchNotFoundItem {
                        source: item.source,
                        id: item.id,
                    }),
                }
            }
            Ok(Json(BatchResponse { results, not_found }).into_response())
        } else {
            let mut results = Vec::with_capacity(items.len());
            let mut not_found = Vec::with_capacity(items.len());
            for item in items {
                match crate::db::problems::get_problem_record_result(&pool, &item.source, &item.id)?
                {
                    Some(record) => results.push(ProblemSummary::from(record)),
                    None => not_found.push(BatchNotFoundItem {
                        source: item.source,
                        id: item.id,
                    }),
                }
            }
            Ok(Json(BatchResponse { results, not_found }).into_response())
        }
    })
    .await
    .unwrap_or(Err(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
        Some("task panicked".to_string()),
    )));

    match result {
        Ok(resp) => resp,
        Err(_) => ProblemDetail::internal("database error").into_response(),
    }
}
