use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::error::ProblemDetail;
use crate::models::{ProblemRecord, ProblemSummary};
use crate::AppState;

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Deserialize, utoipa::IntoParams)]
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

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ListMeta {
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ListResponse<T: Serialize + utoipa::ToSchema> {
    pub data: Vec<T>,
    pub meta: ListMeta,
}

pub(crate) const VALID_SOURCES: &[&str] = &["atcoder", "leetcode", "codeforces", "luogu", "spoj"];
const PROBLEM_DETAIL_SOURCE_ALIASES: &[&str] = &["gym"];

const MAX_BATCH_SIZE: usize = 50;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BatchItem {
    pub source: String,
    pub id: String,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct BatchQuery {
    pub detail: Option<bool>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct BatchNotFoundItem {
    pub source: String,
    pub id: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct BatchResponse<T: Serialize + utoipa::ToSchema> {
    pub results: Vec<T>,
    pub not_found: Vec<BatchNotFoundItem>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct RandomResponse {
    pub results: Vec<ProblemDetailResponse>,
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

#[utoipa::path(
    get,
    path = "/api/v1/problems/{source}/{id}",
    params(
        ("source" = String, Path, description = "Problem source (leetcode, atcoder, codeforces, luogu, spoj)"),
        ("id" = String, Path, description = "Problem ID"),
    ),
    responses(
        (status = 200, description = "Problem detail", body = ProblemDetailResponse),
        (status = 400, description = "Invalid source", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 404, description = "Problem not found", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Problems"
)]
pub async fn get_problem(
    State(state): State<Arc<AppState>>,
    Path((source, id)): Path<(String, String)>,
) -> Response {
    get_problem_by_source_and_id(state, source, id).await
}

#[utoipa::path(
    get,
    path = "/api/v1/problems/atcoder/{contest}/{problem}",
    params(
        ("contest" = String, Path, description = "AtCoder contest ID"),
        ("problem" = String, Path, description = "AtCoder problem ID"),
    ),
    responses(
        (status = 200, description = "Problem detail", body = ProblemDetailResponse),
        (status = 404, description = "Problem not found", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Problems"
)]
pub async fn get_atcoder_problem_with_contest(
    State(state): State<Arc<AppState>>,
    Path((contest, problem)): Path<(String, String)>,
) -> Response {
    get_problem_by_source_and_id(state, "atcoder".to_string(), format!("{contest}/{problem}")).await
}

#[utoipa::path(
    get,
    path = "/api/v1/problems/atcoder/{contest}/tasks/{problem}",
    params(
        ("contest" = String, Path, description = "AtCoder contest ID"),
        ("problem" = String, Path, description = "AtCoder problem ID"),
    ),
    responses(
        (status = 200, description = "Problem detail", body = ProblemDetailResponse),
        (status = 404, description = "Problem not found", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Problems"
)]
pub async fn get_atcoder_problem_with_tasks_path(
    State(state): State<Arc<AppState>>,
    Path((contest, problem)): Path<(String, String)>,
) -> Response {
    get_problem_by_source_and_id(
        state,
        "atcoder".to_string(),
        format!("{contest}/tasks/{problem}"),
    )
    .await
}

async fn get_problem_by_source_and_id(
    state: Arc<AppState>,
    source: String,
    id: String,
) -> Response {
    if !VALID_SOURCES.contains(&source.as_str())
        && !PROBLEM_DETAIL_SOURCE_ALIASES.contains(&source.as_str())
    {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }

    let direct_plan = crate::dynamic_problem::derive_direct_fetch_plan(&source, &id);
    if source == "gym" && direct_plan.is_none() {
        return ProblemDetail::not_found("problem not found").into_response();
    }

    let (db_source, id_for_db) = direct_plan
        .as_ref()
        .map(|plan| (plan.db_source.clone(), plan.db_id.clone()))
        .unwrap_or_else(|| {
            if source == "gym" {
                ("codeforces".to_string(), id.clone())
            } else {
                (source.clone(), id.clone())
            }
        });

    let pool = state.ro_pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let record = crate::db::problems::get_problem_record(&pool, &db_source, &id_for_db)?;
        Some(build_problem_detail_response(&pool, record))
    })
    .await
    .unwrap_or(None);

    match result {
        Some(problem) => Json(problem).into_response(),
        None => match crate::dynamic_problem::fetch_problem_on_miss(state, &source, &id).await {
            Some(problem) => Json(problem).into_response(),
            None => ProblemDetail::not_found("problem not found").into_response(),
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/problems/{source}",
    params(
        ("source" = String, Path, description = "Problem source"),
        ListQuery,
    ),
    responses(
        (status = 200, description = "Paginated problem list", body = ListResponse<ProblemSummary>),
        (status = 400, description = "Invalid parameters", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Database error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Problems"
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/problems/tags/{source}",
    params(
        ("source" = String, Path, description = "Problem source"),
    ),
    responses(
        (status = 200, description = "Tag list", body = Vec<String>),
        (status = 400, description = "Invalid source", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Database error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Tags"
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/problems/difficulties/{source}",
    params(
        ("source" = String, Path, description = "Problem source"),
    ),
    responses(
        (status = 200, description = "Difficulty list", body = Vec<String>),
        (status = 400, description = "Invalid source", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Database error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Difficulties"
)]
pub async fn list_difficulties(
    State(state): State<Arc<AppState>>,
    Path(source): Path<String>,
) -> impl IntoResponse {
    if !VALID_SOURCES.contains(&source.as_str()) {
        return ProblemDetail::bad_request(format!("invalid source: {}", source)).into_response();
    }

    let pool = state.ro_pool.clone();
    let result =
        tokio::task::spawn_blocking(move || crate::db::problems::list_difficulties(&pool, &source))
            .await
            .unwrap_or(None);

    match result {
        Some(difficulties) => Json(difficulties).into_response(),
        None => ProblemDetail::internal("database error").into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/problems/batch",
    params(
        BatchQuery,
    ),
    request_body = Vec<BatchItem>,
    responses(
        (status = 200, description = "Batch results with found and not-found items", body = BatchResponse<ProblemSummary>),
        (status = 400, description = "Invalid request", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Database error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Problems"
)]
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

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::Extension;
    use tokio::sync::{RwLock, Semaphore};
    use tower::ServiceExt;

    use super::*;
    use crate::config::Config;
    use crate::models::Problem;

    fn test_db_path() -> String {
        std::env::temp_dir()
            .join(format!(
                "oj-api-rs-batch-tests-{}.sqlite",
                uuid::Uuid::new_v4()
            ))
            .to_string_lossy()
            .into_owned()
    }

    fn cleanup_db_files(path: &str) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(format!("{path}-wal"));
        let _ = fs::remove_file(format!("{path}-shm"));
    }

    fn test_state() -> (Arc<AppState>, String) {
        test_state_with_database_path(None)
    }

    fn test_state_with_database_path(database_path: Option<&str>) -> (Arc<AppState>, String) {
        crate::db::register_sqlite_vec();
        let mut config = Config::default();
        if let Some(database_path) = database_path {
            config.database.path = database_path.to_string();
        }
        let path = test_db_path();
        let rw_pool = crate::db::create_rw_pool(&path, 1, config.database.busy_timeout_ms);
        crate::db::ensure_data_tables(&rw_pool);
        let ro_pool = crate::db::create_ro_pool(&path, 1, config.database.busy_timeout_ms);

        let state = Arc::new(AppState {
            ro_pool,
            rw_pool,
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
        });

        (state, path)
    }

    fn insert_problem(state: &Arc<AppState>, problem: Problem) {
        crate::db::problems::insert_problem(&state.rw_pool, &problem).unwrap();
    }

    fn sample_problem(id: &str, slug: &str, source: &str) -> Problem {
        Problem {
            id: id.to_string(),
            source: source.to_string(),
            slug: slug.to_string(),
            title: Some(slug.to_string()),
            title_cn: None,
            difficulty: Some("Easy".to_string()),
            ac_rate: Some(50.0),
            rating: None,
            contest: None,
            problem_index: None,
            tags: vec!["array".to_string()],
            link: Some(format!("https://leetcode.com/problems/{slug}/")),
            category: Some("Algorithms".to_string()),
            paid_only: Some(0),
            content: Some("content".to_string()),
            content_cn: None,
            similar_questions: vec!["3sum".to_string()],
        }
    }

    async fn call_batch(
        state: &Arc<AppState>,
        items: Vec<BatchItem>,
        detail: Option<bool>,
    ) -> (StatusCode, serde_json::Value) {
        let query = BatchQuery { detail };
        let response = super::batch_problems(State(state.clone()), Query(query), Json(items))
            .await
            .into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        (status, json)
    }

    async fn call_list_difficulties(
        state: &Arc<AppState>,
        source: &str,
    ) -> (StatusCode, serde_json::Value) {
        let response = super::list_difficulties(State(state.clone()), Path(source.to_string()))
            .await
            .into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        (status, json)
    }

    async fn call_public_route(state: &Arc<AppState>, path: &str) -> (StatusCode, Vec<u8>) {
        let app = crate::api::public_router()
            .layer(Extension(crate::auth::AuthRwPool(Arc::new(
                state.rw_pool.clone(),
            ))))
            .layer(Extension(crate::auth::TokenAuthEnabled(Arc::new(
                AtomicBool::new(false),
            ))))
            .with_state(state.clone());
        let response = app
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, body.to_vec())
    }

    #[tokio::test]
    async fn get_problem_returns_database_hit_without_dynamic_fetch() {
        let (state, path) = test_state_with_database_path(Some("__dynamic_fetch_mock_fail__"));
        insert_problem(&state, sample_problem("1988A", "1988A", "codeforces"));

        let response = super::get_problem(
            State(state.clone()),
            Path(("codeforces".to_string(), "1988A".to_string())),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], "1988A");
        assert_eq!(json["content"], "content");

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn get_problem_rejects_regular_contest_hit_through_gym_alias() {
        let (state, path) = test_state_with_database_path(Some("__dynamic_fetch_mock_fail__"));
        insert_problem(&state, sample_problem("1988A", "1988A", "codeforces"));

        let response = super::get_problem(
            State(state.clone()),
            Path(("gym".to_string(), "1988A".to_string())),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn get_problem_fetches_supported_database_miss() {
        let (state, path) = test_state_with_database_path(Some("__dynamic_fetch_mock_success__"));

        let response = super::get_problem(
            State(state.clone()),
            Path(("luogu".to_string(), "P1083".to_string())),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], "P1083");
        assert_eq!(json["content"], "mock fetched content");
        assert_eq!(json["link"], "https://www.luogu.com.cn/problem/P1083");

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn get_problem_keeps_404_when_dynamic_fetch_fails() {
        let (state, path) = test_state_with_database_path(Some("__dynamic_fetch_mock_fail__"));

        let response = super::get_problem(
            State(state.clone()),
            Path(("codeforces".to_string(), "1988A".to_string())),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn get_atcoder_explicit_path_uses_normalized_database_id() {
        let (state, path) = test_state_with_database_path(Some("__dynamic_fetch_mock_fail__"));
        insert_problem(
            &state,
            sample_problem("aaabbb_aaabbb_ccc", "aaabbb_aaabbb_ccc", "atcoder"),
        );

        for request_path in [
            "/api/v1/problems/atcoder/abc042/aaabbb_aaabbb_ccc",
            "/api/v1/problems/atcoder/abc042/tasks/aaabbb_aaabbb_ccc",
        ] {
            let (status, body) = call_public_route(&state, request_path).await;
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(status, StatusCode::OK);
            assert_eq!(json["id"], "aaabbb_aaabbb_ccc");
            assert_eq!(json["source"], "atcoder");
            assert_eq!(json["content"], "content");
        }

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn get_atcoder_explicit_path_fetches_with_normalized_database_id() {
        let (state, path) = test_state_with_database_path(Some("__dynamic_fetch_mock_success__"));

        let (status, body) =
            call_public_route(&state, "/api/v1/problems/atcoder/abc042/aaabbb_aaabbb_ccc").await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["id"], "aaabbb_aaabbb_ccc");
        assert_eq!(
            json["link"],
            "https://atcoder.jp/contests/abc042/tasks/aaabbb_aaabbb_ccc"
        );

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn public_router_nested_difficulties_returns_json_array() {
        let (state, path) = test_state();
        let mut hard = sample_problem("3", "hard", "leetcode");
        hard.difficulty = Some("Hard".to_string());
        let mut easy = sample_problem("1", "easy", "leetcode");
        easy.difficulty = Some("Easy".to_string());
        insert_problem(&state, hard);
        insert_problem(&state, easy);

        let (status, body) =
            call_public_route(&state, "/api/v1/problems/difficulties/leetcode").await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!(["Easy", "Hard"]));
        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn public_router_nested_tags_returns_json_array() {
        let (state, path) = test_state();
        let mut graph = sample_problem("2", "graph", "leetcode");
        graph.tags = vec!["graph".to_string(), "Array".to_string()];
        insert_problem(&state, sample_problem("1", "array", "leetcode"));
        insert_problem(&state, graph);

        let (status, body) = call_public_route(&state, "/api/v1/problems/tags/leetcode").await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!(["array", "graph"]));
        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn public_router_old_metadata_paths_are_not_registered() {
        let (state, path) = test_state();

        for old_path in ["/api/v1/difficulties/leetcode", "/api/v1/tags/leetcode"] {
            let (status, _) = call_public_route(&state, old_path).await;
            assert_eq!(status, StatusCode::NOT_FOUND);
        }

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn list_difficulties_rejects_invalid_source() {
        let (state, path) = test_state();
        let (status, json) = call_list_difficulties(&state, "invalid_source").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json["detail"].as_str().unwrap().contains("invalid source"));
        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn list_difficulties_returns_json_array() {
        let (state, path) = test_state();
        let mut hard = sample_problem("3", "hard", "leetcode");
        hard.difficulty = Some("Hard".to_string());
        let mut easy = sample_problem("1", "easy", "leetcode");
        easy.difficulty = Some("Easy".to_string());
        insert_problem(&state, hard);
        insert_problem(&state, easy);

        let (status, json) = call_list_difficulties(&state, "leetcode").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!(["Easy", "Hard"]));
        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn list_difficulties_returns_empty_array_without_values() {
        let (state, path) = test_state();
        let mut problem = sample_problem("1", "one", "atcoder");
        problem.difficulty = None;
        insert_problem(&state, problem);

        let (status, json) = call_list_difficulties(&state, "atcoder").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!([]));
        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn batch_empty_body_returns_400() {
        let (state, path) = test_state();
        let (status, json) = call_batch(&state, vec![], None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json["detail"].as_str().unwrap().contains("empty"));
        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn batch_oversized_returns_400() {
        let (state, path) = test_state();
        let items: Vec<BatchItem> = (0..51)
            .map(|i| BatchItem {
                source: "leetcode".to_string(),
                id: i.to_string(),
            })
            .collect();
        let (status, json) = call_batch(&state, items, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json["detail"].as_str().unwrap().contains("exceeds maximum"));
        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn batch_invalid_source_returns_400() {
        let (state, path) = test_state();
        let items = vec![BatchItem {
            source: "invalid_source".to_string(),
            id: "1".to_string(),
        }];
        let (status, json) = call_batch(&state, items, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json["detail"].as_str().unwrap().contains("invalid source"));
        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn batch_summary_mode_returns_results_and_not_found() {
        let (state, path) = test_state();
        insert_problem(&state, sample_problem("1", "two-sum", "leetcode"));
        insert_problem(&state, sample_problem("15", "3sum", "leetcode"));

        let items = vec![
            BatchItem {
                source: "leetcode".to_string(),
                id: "1".to_string(),
            },
            BatchItem {
                source: "leetcode".to_string(),
                id: "999".to_string(),
            },
            BatchItem {
                source: "leetcode".to_string(),
                id: "15".to_string(),
            },
        ];
        let (status, json) = call_batch(&state, items, Some(false)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["results"].as_array().unwrap().len(), 2);
        assert_eq!(json["not_found"].as_array().unwrap().len(), 1);
        assert_eq!(json["not_found"][0]["id"], "999");
        // summary mode should not include content field
        assert!(json["results"][0]["content"].is_null());
        assert_eq!(json["results"][0]["slug"], "two-sum");

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn batch_detail_mode_returns_full_content() {
        let (state, path) = test_state();
        insert_problem(&state, sample_problem("1", "two-sum", "leetcode"));
        insert_problem(&state, sample_problem("15", "3sum", "leetcode"));

        let items = vec![
            BatchItem {
                source: "leetcode".to_string(),
                id: "1".to_string(),
            },
            BatchItem {
                source: "leetcode".to_string(),
                id: "15".to_string(),
            },
        ];
        let (status, json) = call_batch(&state, items, Some(true)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["results"].as_array().unwrap().len(), 2);
        assert!(json["not_found"].as_array().unwrap().is_empty());
        // detail mode should include content and similar_questions
        assert_eq!(json["results"][0]["content"], "content");
        assert!(json["results"][0]["similar_questions"].is_array());

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn batch_all_not_found_returns_empty_results() {
        let (state, path) = test_state();
        let items = vec![
            BatchItem {
                source: "leetcode".to_string(),
                id: "999".to_string(),
            },
            BatchItem {
                source: "codeforces".to_string(),
                id: "999Z".to_string(),
            },
        ];
        let (status, json) = call_batch(&state, items, None).await;

        assert_eq!(status, StatusCode::OK);
        assert!(json["results"].as_array().unwrap().is_empty());
        assert_eq!(json["not_found"].as_array().unwrap().len(), 2);

        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn batch_default_mode_is_summary() {
        let (state, path) = test_state();
        insert_problem(&state, sample_problem("1", "two-sum", "leetcode"));

        let items = vec![BatchItem {
            source: "leetcode".to_string(),
            id: "1".to_string(),
        }];
        // no detail param → should behave as summary
        let (status, json) = call_batch(&state, items, None).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["results"].as_array().unwrap().len(), 1);
        assert!(json["results"][0]["content"].is_null());

        cleanup_db_files(&path);
    }
}
