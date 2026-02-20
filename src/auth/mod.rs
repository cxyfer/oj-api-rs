use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

use crate::api::error::ProblemDetail;
use crate::db::DbPool;

pub async fn bearer_auth(
    pool: Arc<DbPool>,
    request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => {
            let t = h[7..].trim();
            if t.is_empty() {
                return ProblemDetail::unauthorized("missing or invalid token").into_response();
            }
            t.to_string()
        }
        _ => {
            return ProblemDetail::unauthorized("missing or invalid token").into_response();
        }
    };

    let pool_clone = pool.clone();
    let valid = tokio::task::spawn_blocking(move || {
        crate::db::tokens::validate_token(&pool_clone, &token)
    })
    .await
    .unwrap_or(false);

    if !valid {
        return ProblemDetail::unauthorized("missing or invalid token").into_response();
    }

    next.run(request).await
}

pub async fn admin_auth(
    admin_secret: Arc<String>,
    request: Request,
    next: Next,
) -> Response {
    let secret_header = request
        .headers()
        .get("x-admin-secret")
        .and_then(|v| v.to_str().ok());

    match secret_header {
        Some(s) if s == admin_secret.as_str() => next.run(request).await,
        _ => ProblemDetail::unauthorized("invalid admin secret").into_response(),
    }
}
