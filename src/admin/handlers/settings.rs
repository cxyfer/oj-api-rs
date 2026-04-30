use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ProblemDetail;
use crate::AppState;

// Settings toggle

#[derive(Deserialize, utoipa::ToSchema)]
pub struct TokenAuthSettingRequest {
    pub enabled: bool,
}

#[utoipa::path(
    get,
    path = "/admin/api/settings/token-auth",
    responses(
        (status = 200, description = "Token auth setting", body = serde_json::Value),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn get_token_auth_setting(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let enabled = state.token_auth_enabled.load(Ordering::Acquire);
    Json(serde_json::json!({ "enabled": enabled }))
}

#[utoipa::path(
    put,
    path = "/admin/api/settings/token-auth",
    request_body = TokenAuthSettingRequest,
    responses(
        (status = 200, description = "Setting updated", body = serde_json::Value),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn set_token_auth_setting(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TokenAuthSettingRequest>,
) -> impl IntoResponse {
    let pool = state.rw_pool.clone();
    let value = if body.enabled { "1" } else { "0" };

    let ok = tokio::task::spawn_blocking(move || {
        crate::db::settings::set_setting(&pool, "token_auth_enabled", value)
    })
    .await
    .unwrap_or(false);

    if ok {
        state
            .token_auth_enabled
            .store(body.enabled, Ordering::Release);
        Json(serde_json::json!({ "enabled": body.enabled })).into_response()
    } else {
        ProblemDetail::internal("failed to update setting").into_response()
    }
}
