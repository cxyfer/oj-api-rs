use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::problems::{build_problem_detail_response, ProblemDetailResponse};
use crate::AppState;

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ResolveResponse {
    pub(crate) source: String,
    pub(crate) id: String,
    pub(crate) problem: Option<ProblemDetailResponse>,
}

#[utoipa::path(
    get,
    path = "/api/v1/resolve/{query}",
    params(
        ("query" = String, Path, description = "Problem identifier or URL to resolve (accepts numeric IDs, slugs, or URLs; captures slashes)"),
    ),
    responses(
        (status = 200, description = "Resolved problem", body = ResolveResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "Resolve"
)]
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
        let problem = crate::db::problems::get_problem_record(&pool, &source_str, &eid)
            .map(|record| build_problem_detail_response(&pool, record));
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

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use tokio::sync::{RwLock, Semaphore};

    use super::*;
    use crate::config::Config;
    use crate::models::Problem;

    fn test_db_path() -> String {
        std::env::temp_dir()
            .join(format!(
                "oj-api-rs-resolve-tests-{}.sqlite",
                uuid::Uuid::new_v4()
            ))
            .to_string_lossy()
            .into_owned()
    }

    fn cleanup_db_files(path: &str) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(format!("{path}-wal"));
        let _ = fs::remove_file(format!("{path}-shm"));
    }

    fn test_state() -> (Arc<AppState>, String) {
        crate::db::register_sqlite_vec();
        let config = Config::default();
        let path = test_db_path();
        let rw_pool = crate::db::create_rw_pool(&path, 1, config.database.busy_timeout_ms);
        crate::db::ensure_data_tables(&rw_pool);
        let ro_pool = crate::db::create_ro_pool(&path, 1, config.database.busy_timeout_ms);

        let state = Arc::new(AppState {
            ro_pool,
            rw_pool,
            config,
            crawler_jobs: tokio::sync::Mutex::new(HashMap::new()),
            manual_crawler_guard: tokio::sync::Mutex::new(None),
            crawler_history: tokio::sync::Mutex::new(VecDeque::new()),
            embedding_lock: tokio::sync::Mutex::new(None),
            embedding_launch_guard: tokio::sync::Mutex::new(None),
            embedding_history: tokio::sync::Mutex::new(VecDeque::new()),
            active_crawler_pids: tokio::sync::Mutex::new(HashMap::new()),
            active_embedding_pid: tokio::sync::Mutex::new(None),
            daily_fallback: tokio::sync::Mutex::new(HashMap::new()),
            retained_refresh: tokio::sync::Mutex::new(crate::utils::RetainedRefreshState::default()),
            embed_semaphore: Semaphore::new(1),
            token_auth_enabled: Arc::new(AtomicBool::new(true)),
            admin_sessions: Arc::new(RwLock::new(HashMap::new())),
            config_path: None,
        });

        (state, path)
    }

    fn insert_problem(state: &Arc<AppState>, problem: Problem) {
        crate::db::problems::insert_problem(&state.rw_pool, &problem).unwrap();
    }

    fn sample_problem(id: &str, slug: &str, similar_questions: Vec<String>) -> Problem {
        Problem {
            id: id.to_string(),
            source: "leetcode".to_string(),
            slug: slug.to_string(),
            title: Some(slug.to_string()),
            title_cn: None,
            difficulty: Some("Easy".to_string()),
            ac_rate: Some(50.0),
            rating: None,
            contest: None,
            problem_index: None,
            tags: vec!["array".to_string()],
            link: Some(format!("https://leetcode.com/problems/{slug}/")),
            category: Some("Algorithms".to_string()),
            paid_only: Some(0),
            content: Some("content".to_string()),
            content_cn: None,
            similar_questions,
        }
    }

    #[tokio::test]
    async fn resolve_returns_hydrated_similar_questions_for_numeric_id() {
        let (state, path) = test_state();
        insert_problem(
            &state,
            sample_problem("1", "two-sum", vec!["3sum".to_string()]),
        );
        insert_problem(&state, sample_problem("15", "3sum", Vec::new()));

        let response = super::resolve(State(state.clone()), Path("1".to_string()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["id"], "1");
        assert!(json["problem"]["similar_questions"].is_array());
        assert!(json["problem"]["similar_questions"][0].is_object());
        assert_eq!(json["problem"]["similar_questions"][0]["slug"], "3sum");

        drop(state);
        cleanup_db_files(&path);
    }

    #[tokio::test]
    async fn resolve_slug_lookup_keeps_hydrated_similar_questions() {
        let (state, path) = test_state();
        insert_problem(
            &state,
            sample_problem("1", "two-sum", vec!["3sum".to_string()]),
        );
        insert_problem(&state, sample_problem("15", "3sum", Vec::new()));

        let response = super::resolve(State(state.clone()), Path("two-sum".to_string()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["id"], "1");
        assert_eq!(json["problem"]["slug"], "two-sum");
        assert!(json["problem"]["similar_questions"][0].is_object());
        assert_eq!(json["problem"]["similar_questions"][0]["slug"], "3sum");

        drop(state);
        cleanup_db_files(&path);
    }
}
