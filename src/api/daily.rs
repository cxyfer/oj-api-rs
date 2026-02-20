use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ProblemDetail;
use crate::AppState;

#[derive(Deserialize)]
pub struct DailyQuery {
    pub domain: Option<String>,
    pub date: Option<String>,
}

pub async fn get_daily(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DailyQuery>,
) -> impl IntoResponse {
    let domain = query.domain.as_deref().unwrap_or("com");
    if domain != "com" && domain != "cn" {
        return ProblemDetail::bad_request("domain must be 'com' or 'cn'").into_response();
    }

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let date = query.date.as_deref().unwrap_or(&today);

    // Validate date format
    let date_re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    if !date_re.is_match(date) {
        return ProblemDetail::bad_request("invalid date format, expected YYYY-MM-DD")
            .into_response();
    }

    // Validate actual calendar date
    let parsed = match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return ProblemDetail::bad_request("invalid calendar date").into_response();
        }
    };

    let lower = chrono::NaiveDate::from_ymd_opt(2020, 4, 1).unwrap();
    let upper = chrono::Utc::now().date_naive();

    if parsed < lower {
        return ProblemDetail::bad_request("date must be >= 2020-04-01").into_response();
    }
    if parsed > upper {
        return ProblemDetail::bad_request("date must be <= today").into_response();
    }

    let pool = state.ro_pool.clone();
    let domain_owned = domain.to_string();
    let date_owned = date.to_string();
    let result = tokio::task::spawn_blocking(move || {
        crate::db::daily::get_daily(&pool, &domain_owned, &date_owned)
    })
    .await
    .unwrap_or(None);

    match result {
        Some(d) => Json(d).into_response(),
        None => ProblemDetail::not_found("no daily challenge found for this date").into_response(),
    }
}
