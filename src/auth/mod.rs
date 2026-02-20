use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Extension;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::error::ProblemDetail;
use crate::db::DbPool;

#[derive(Clone)]
pub struct AuthRwPool(pub Arc<DbPool>);

#[derive(Clone)]
pub struct AdminSecret(pub String);

#[derive(Clone)]
pub struct AdminSessions(pub Arc<RwLock<std::collections::HashMap<String, i64>>>);

#[derive(Clone)]
pub struct TokenAuthEnabled(pub Arc<AtomicBool>);

pub async fn bearer_auth(
    Extension(auth_pool): Extension<AuthRwPool>,
    Extension(token_auth): Extension<TokenAuthEnabled>,
    request: Request,
    next: Next,
) -> Response {
    if !token_auth.0.load(Ordering::Acquire) {
        return next.run(request).await;
    }

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

pub fn extract_cookie<'a>(headers: &'a axum::http::HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all("cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(';'))
        .map(|s| s.trim())
        .find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let val = parts.next()?;
            if key == name { Some(val) } else { None }
        })
}

pub async fn admin_auth(
    Extension(secret): Extension<AdminSecret>,
    Extension(sessions): Extension<AdminSessions>,
    request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();

    // 1. Check x-admin-secret header first (backward compat with cURL)
    if let Some(s) = headers.get("x-admin-secret").and_then(|v| v.to_str().ok()) {
        if !secret.0.is_empty() && s == secret.0 {
            return next.run(request).await;
        }
    }

    // 2. Check oj_admin_session cookie
    if let Some(session_token) = extract_cookie(headers, "oj_admin_session") {
        let map = sessions.0.read().await;
        if let Some(&expires_at) = map.get(session_token) {
            let now = chrono::Utc::now().timestamp();
            if expires_at > now {
                drop(map);
                return next.run(request).await;
            }
            // Expired — lazy cleanup
            drop(map);
            sessions.0.write().await.remove(session_token);
        }
    }

    // 3. Determine response: redirect for page routes, 401 for API routes
    let is_api = request.uri().path().contains("/api/");
    if is_api {
        ProblemDetail::unauthorized("invalid admin secret").into_response()
    } else {
        Redirect::to("/admin/login").into_response()
    }
}
