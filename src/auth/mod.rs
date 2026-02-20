use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use std::sync::Arc;

use crate::api::error::ProblemDetail;
use crate::db::DbPool;

#[derive(Clone)]
pub struct AuthPool(pub Arc<DbPool>);

#[derive(Clone)]
pub struct AdminSecret(pub String);

pub async fn bearer_auth(
    Extension(auth_pool): Extension<AuthPool>,
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

    let pool = auth_pool.0.clone();
    let valid = tokio::task::spawn_blocking(move || {
        crate::db::tokens::validate_token(&pool, &token)
    })
    .await
    .unwrap_or(false);

    if !valid {
        return ProblemDetail::unauthorized("missing or invalid token").into_response();
    }

    next.run(request).await
}

pub async fn admin_auth(
    Extension(secret): Extension<AdminSecret>,
    request: Request,
    next: Next,
) -> Response {
    let secret_header = request
        .headers()
        .get("x-admin-secret")
        .and_then(|v| v.to_str().ok());

    match secret_header {
        Some(s) if s == secret.0 => next.run(request).await,
        _ => ProblemDetail::unauthorized("invalid admin secret").into_response(),
    }
}
