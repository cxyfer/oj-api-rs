use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ProblemDetail;
use crate::api::problems::{
    validate_list_query, ListMeta, ListQuery, ListResponse, ProblemDetailResponse, VALID_SOURCES,
};
use crate::models::{Problem, ProblemSummary};
use crate::AppState;

// Problem CRUD

#[derive(Deserialize, utoipa::ToSchema)]
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

#[utoipa::path(
    post,
    path = "/admin/api/problems",
    request_body = CreateProblemRequest,
    responses(
        (status = 201, description = "Problem created"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
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

#[utoipa::path(
    put,
    path = "/admin/api/problems/{source}/{id}",
    params(
        ("source" = String, Path, description = "Problem source"),
        ("id" = String, Path, description = "Problem ID"),
    ),
    request_body = CreateProblemRequest,
    responses(
        (status = 200, description = "Problem updated"),
        (status = 404, description = "Problem not found", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
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

#[utoipa::path(
    delete,
    path = "/admin/api/problems/{source}/{id}",
    params(
        ("source" = String, Path, description = "Problem source"),
        ("id" = String, Path, description = "Problem ID"),
    ),
    responses(
        (status = 204, description = "Problem deleted"),
        (status = 404, description = "Problem not found", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
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

#[utoipa::path(
    get,
    path = "/admin/api/problems/{source}",
    params(
        ("source" = String, Path, description = "Problem source"),
        ListQuery,
    ),
    responses(
        (status = 200, description = "Paginated problem list", body = ListResponse<ProblemSummary>),
        (status = 400, description = "Invalid parameters", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
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

#[utoipa::path(
    get,
    path = "/admin/api/tags/{source}",
    params(
        ("source" = String, Path, description = "Problem source"),
    ),
    responses(
        (status = 200, description = "Tag list", body = Vec<String>),
        (status = 400, description = "Invalid source", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
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

#[utoipa::path(
    get,
    path = "/admin/api/problems/{source}/{id}",
    params(
        ("source" = String, Path, description = "Problem source"),
        ("id" = String, Path, description = "Problem ID"),
    ),
    responses(
        (status = 200, description = "Problem detail", body = ProblemDetailResponse),
        (status = 400, description = "Invalid source", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 404, description = "Problem not found", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn get_problem_detail(
    State(state): State<Arc<AppState>>,
    Path((source, id)): Path<(String, String)>,
) -> impl IntoResponse {
    if !VALID_SOURCES.contains(&source.as_str()) {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }

    let pool = state.ro_pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let record = crate::db::problems::get_problem_record(&pool, &source, &id)?;
        Some(crate::api::problems::build_problem_detail_response(
            &pool, record,
        ))
    })
    .await;

    match result {
        Ok(Some(problem)) => Json(problem).into_response(),
        Ok(None) => ProblemDetail::not_found("problem not found").into_response(),
        Err(_) => ProblemDetail::internal("database error").into_response(),
    }
}
