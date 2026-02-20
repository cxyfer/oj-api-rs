use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

use crate::models::Problem;
use crate::AppState;

#[derive(Serialize)]
struct ResolveResponse {
    source: String,
    id: String,
    problem: Option<Problem>,
}

pub async fn resolve(
    State(state): State<Arc<AppState>>,
    Path(query): Path<String>,
) -> impl IntoResponse {
    let decoded = urlencoding::decode(&query)
        .map(|s| s.into_owned())
        .unwrap_or(query);

    let (source, id) = crate::detect::detect_source(&decoded);
    let source_str = source.to_string();
    let id_clone = id.clone();

    let pool = state.ro_pool.clone();
    let problem = tokio::task::spawn_blocking(move || {
        crate::db::problems::get_problem(&pool, &source_str, &id_clone)
    })
    .await
    .unwrap_or(None);

    Json(ResolveResponse {
        source: source.to_string(),
        id,
        problem,
    })
    .into_response()
}
