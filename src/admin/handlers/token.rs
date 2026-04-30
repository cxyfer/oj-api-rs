use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ProblemDetail;
use crate::models::ApiToken;
use crate::AppState;

// Token management

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateTokenRequest {
    pub label: Option<String>,
}

#[utoipa::path(
    get,
    path = "/admin/api/tokens",
    responses(
        (status = 200, description = "Token list", body = Vec<ApiToken>),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn list_tokens(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = state.rw_pool.clone();
    let tokens = tokio::task::spawn_blocking(move || crate::db::tokens::list_tokens(&pool))
        .await
        .unwrap_or_default();

    Json(tokens).into_response()
}

#[utoipa::path(
    post,
    path = "/admin/api/tokens",
    request_body = CreateTokenRequest,
    responses(
        (status = 201, description = "Token created", body = ApiToken),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn create_token(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateTokenRequest>,
) -> impl IntoResponse {
    let pool = state.rw_pool.clone();
    let label = body.label;

    let result = tokio::task::spawn_blocking(move || {
        crate::db::tokens::create_token(&pool, label.as_deref())
    })
    .await
    .unwrap_or(None);

    match result {
        Some(token) => (StatusCode::CREATED, Json(token)).into_response(),
        None => ProblemDetail::internal("failed to create token").into_response(),
    }
}

#[utoipa::path(
    delete,
    path = "/admin/api/tokens/{token}",
    params(
        ("token" = String, Path, description = "API token to revoke"),
    ),
    responses(
        (status = 204, description = "Token revoked"),
        (status = 404, description = "Token not found", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("admin_secret" = []), ("admin_session" = [])),
    tag = "Admin"
)]
pub async fn revoke_token(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    let pool = state.rw_pool.clone();

    let result =
        tokio::task::spawn_blocking(move || crate::db::tokens::revoke_token(&pool, &token))
            .await
            .unwrap_or(None);

    match result {
        Some(true) => StatusCode::NO_CONTENT.into_response(),
        Some(false) => ProblemDetail::not_found("token not found").into_response(),
        None => ProblemDetail::internal("database error").into_response(),
    }
}
