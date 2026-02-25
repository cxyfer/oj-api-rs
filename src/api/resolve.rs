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
    let id_for_closure = id.clone();

    let pool = state.ro_pool.clone();
    let (effective_id, problem) = tokio::task::spawn_blocking(move || {
        let eid = if source_str == "leetcode"
            && id_for_closure.contains(|c: char| !c.is_ascii_digit())
        {
            let slug = id_for_closure.to_lowercase();
            crate::db::problems::get_problem_id_by_slug(&pool, "leetcode", &slug).unwrap_or(slug)
        } else {
            id_for_closure
        };
        let problem = crate::db::problems::get_problem(&pool, &source_str, &eid);
        (eid, problem)
    })
    .await
    .unwrap_or((id, None));

    Json(ResolveResponse {
        source: source.to_string(),
        id: effective_id,
        problem,
    })
    .into_response()
}
