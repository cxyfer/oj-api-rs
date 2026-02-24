use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
struct StatusResponse {
    version: &'static str,
    platforms: Vec<crate::db::problems::PlatformStats>,
}

pub async fn get_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = state.ro_pool.clone();
    let platforms = tokio::task::spawn_blocking(move || {
        crate::db::problems::platform_stats(&pool)
    })
    .await
    .unwrap_or_default();

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION"),
        platforms,
    })
}
