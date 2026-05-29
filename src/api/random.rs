use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ProblemDetail;
use crate::api::problems::{build_problem_detail_response, ProblemDetailResponse, RandomResponse, VALID_SOURCES};
use crate::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct RandomQuery {
    /// Filter by problem source (leetcode, atcoder, codeforces, luogu, spoj)
    pub source: Option<String>,
    /// Filter by difficulty — either standardized (easy/medium/hard) or platform-native value
    pub difficulty: Option<String>,
    /// Comma-separated tag names
    pub tags: Option<String>,
    /// Tag matching mode: "any" (default) or "all"
    pub tag_mode: Option<String>,
    /// Minimum rating (inclusive)
    pub rating_min: Option<f64>,
    /// Maximum rating (inclusive)
    pub rating_max: Option<f64>,
    /// Number of problems to return (default 1, max 20)
    pub count: Option<u32>,
}

const VALID_TAG_MODES: &[&str] = &["any", "all"];

fn validate_random_query(query: &RandomQuery) -> Result<(), String> {
    if let Some(ref s) = query.source {
        if !VALID_SOURCES.contains(&s.as_str()) {
            return Err(format!("invalid source: {}", s));
        }
    }
    if let Some(c) = query.count {
        if !(1..=20).contains(&c) {
            return Err("count must be between 1 and 20".to_string());
        }
    }
    if let (Some(min), Some(max)) = (query.rating_min, query.rating_max) {
        if min > max {
            return Err("rating_min must be <= rating_max".to_string());
        }
    }
    if let Some(ref s) = query.tag_mode {
        if !VALID_TAG_MODES.contains(&s.as_str()) {
            return Err(format!("invalid tag_mode: {}", s));
        }
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/random",
    params(
        ("source" = Option<String>, Query, description = "Filter by problem source"),
        ("difficulty" = Option<String>, Query, description = "Standardized (easy/medium/hard) or platform-native difficulty"),
        ("tags" = Option<String>, Query, description = "Comma-separated tag names"),
        ("tag_mode" = Option<String>, Query, description = "Tag match mode: any or all (default any)"),
        ("rating_min" = Option<f64>, Query, description = "Minimum rating (inclusive)"),
        ("rating_max" = Option<f64>, Query, description = "Maximum rating (inclusive)"),
        ("count" = Option<u32>, Query, description = "Number of problems to return (default 1, max 20)"),
    ),
    responses(
        (status = 200, description = "Random problems", body = RandomResponse),
        (status = 400, description = "Invalid parameters", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 401, description = "Unauthorized", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Problems"
)]
pub async fn random_problems(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RandomQuery>,
) -> impl IntoResponse {
    if let Err(e) = validate_random_query(&query) {
        return ProblemDetail::bad_request(e).into_response();
    }

    let pool = state.ro_pool.clone();
    let count = query.count.unwrap_or(1);

    let result = tokio::task::spawn_blocking(move || {
        let tags: Option<Vec<&str>> = query.tags.as_ref().map(|t| {
            t.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        });

        let records = crate::db::random::random_problems(
            &pool,
            query.source.as_deref(),
            query.difficulty.as_deref(),
            tags,
            query.tag_mode.as_deref().unwrap_or("any"),
            query.rating_min,
            query.rating_max,
            count,
        )?;

        let results: Vec<ProblemDetailResponse> = records
            .into_iter()
            .map(|record| build_problem_detail_response(&pool, record))
            .collect();

        Some(results)
    })
    .await
    .unwrap_or(None);

    match result {
        Some(results) => Json(RandomResponse { results }).into_response(),
        None => ProblemDetail::internal("database error").into_response(),
    }
}
