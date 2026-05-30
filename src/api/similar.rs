use std::sync::Arc;

use axum::extract::{Json, Path, Query, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::api::error::ProblemDetail;
use crate::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SimilarByProblemQuery {
    pub limit: Option<u32>,
    pub threshold: Option<f32>,
    pub source: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SimilarByTextQuery {
    #[serde(alias = "q")]
    pub query: Option<String>,
    pub limit: Option<u32>,
    pub threshold: Option<f32>,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct SimilarResponse {
    pub(crate) rewritten_query: Option<String>,
    pub(crate) results: Vec<SimilarResult>,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct SimilarResult {
    pub(crate) source: String,
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    pub(crate) difficulty: Option<String>,
    pub(crate) link: Option<String>,
    pub(crate) similarity: f32,
}

#[derive(Debug, Deserialize)]
struct EmbedTextOutput {
    embedding: Vec<f32>,
    rewritten: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct EmbedTextErrorOutput {
    error: EmbedTextError,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct EmbedTextError {
    stage: String,
    kind: String,
    message: String,
}

fn embed_text_stage_detail(stage: &str) -> Option<&'static str> {
    match stage {
        "config" => Some("embedding service configuration failed"),
        "rewrite" => Some("query rewrite service failed"),
        "embedding" => Some("embedding service failed"),
        "output" => Some("invalid embedding response"),
        _ => None,
    }
}

fn parse_embed_text_error(stdout: &str) -> Option<EmbedTextErrorOutput> {
    serde_json::from_str::<EmbedTextErrorOutput>(stdout)
        .ok()
        .filter(|output| embed_text_stage_detail(&output.error.stage).is_some())
}

fn parse_embed_text_output(stdout: &str) -> Result<EmbedTextOutput, ProblemDetail> {
    serde_json::from_str(stdout)
        .map_err(|_| ProblemDetail::bad_gateway("invalid embedding response"))
}

#[utoipa::path(
    get,
    path = "/api/v1/similar/{source}/{id}",
    params(
        ("source" = String, Path, description = "Problem source"),
        ("id" = String, Path, description = "Problem ID"),
        SimilarByProblemQuery,
    ),
    responses(
        (status = 200, description = "Similar problems by embedding", body = SimilarResponse),
        (status = 404, description = "No embedding found", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Similar"
)]
pub async fn similar_by_problem(
    State(state): State<Arc<AppState>>,
    Path((source, id)): Path<(String, String)>,
    Query(query): Query<SimilarByProblemQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(10).min(50);
    let threshold = query.threshold.unwrap_or(0.0);
    let source_filter: Option<Vec<String>> = query
        .source
        .as_ref()
        .map(|s| s.split(',').map(|v| v.trim().to_string()).collect());

    let pool = state.ro_pool.clone();
    let over_fetch = state.config.embedding.over_fetch_factor;

    let source_clone = source.clone();
    let id_clone = id.clone();

    let result = tokio::task::spawn_blocking(move || {
        let embedding = match crate::db::embeddings::get_embedding(&pool, &source_clone, &id_clone)
        {
            Some(e) => e,
            None => {
                return Err(ProblemDetail::not_found(
                    "no embedding found for this problem",
                ));
            }
        };

        let rewritten_query = crate::db::embeddings::get_rewritten_content(&pool, &source, &id);

        let k = (limit * over_fetch).min(200);
        let knn_results = crate::db::embeddings::knn_search(&pool, &embedding, k);

        let mut results: Vec<SimilarResult> = knn_results
            .into_iter()
            .filter(|(s, pid, _)| !(s == &source && pid == &id))
            .map(|(s, pid, distance)| {
                let similarity = 1.0 - distance;
                (s, pid, similarity)
            })
            .filter(|(_, _, sim)| *sim >= threshold)
            .filter(|(s, _, _)| {
                source_filter
                    .as_ref()
                    .is_none_or(|filters| filters.iter().any(|f| f == s))
            })
            .take(limit as usize)
            .map(|(s, pid, similarity)| {
                let problem = crate::db::problems::get_problem(&pool, &s, &pid);
                SimilarResult {
                    source: s,
                    id: pid,
                    title: problem.as_ref().and_then(|p| p.title.clone()),
                    difficulty: problem.as_ref().and_then(|p| p.difficulty.clone()),
                    link: problem.as_ref().and_then(|p| p.link.clone()),
                    similarity,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(SimilarResponse {
            rewritten_query,
            results,
        })
    })
    .await
    .unwrap_or(Err(ProblemDetail::internal("task join error")));

    match result {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/similar",
    request_body = SimilarByTextQuery,
    responses(
        (status = 200, description = "Similar problems by text query", body = SimilarResponse),
        (status = 400, description = "Invalid query", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 502, description = "Embedding service error", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 504, description = "Embedding service timeout", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Similar"
)]
pub async fn similar_by_text(
    State(state): State<Arc<AppState>>,
    Json(query): Json<SimilarByTextQuery>,
) -> impl IntoResponse {
    let processed = query.query.as_deref().map(|q| {
        let trimmed = q.trim();
        if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
            &trimmed[1..trimmed.len() - 1]
        } else {
            trimmed
        }
    });

    let text = match processed {
        Some("") => {
            return ProblemDetail::bad_request("query field is required").into_response();
        }
        Some(q) if q.len() > 2000 => {
            return ProblemDetail::bad_request("query must be at most 2000 characters")
                .into_response();
        }
        Some(q) if q.len() < 3 => {
            return ProblemDetail::bad_request("query must be at least 3 characters")
                .into_response();
        }
        Some(q) => q.to_string(),
        None => {
            return ProblemDetail::bad_request("query field is required").into_response();
        }
    };

    let limit = query.limit.unwrap_or(10).min(50);
    let threshold = query.threshold.unwrap_or(0.0);
    let source_filter: Option<Vec<String>> = query
        .source
        .as_ref()
        .map(|s| s.split(',').map(|v| v.trim().to_string()).collect());

    let embed_timeout = state.config.embedding.timeout_secs;

    // Acquire semaphore permit
    let _permit = match state.embed_semaphore.acquire().await {
        Ok(p) => p,
        Err(_) => {
            return ProblemDetail::internal("semaphore closed").into_response();
        }
    };

    // Spawn Python subprocess
    let mut cmd = tokio::process::Command::new("uv");
    cmd.args(["run", "python3", "embedding_cli.py", "--embed-text", &text]);
    cmd.current_dir("scripts/");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    if let Some(ref cp) = state.config_path {
        cmd.env("CONFIG_PATH", cp);
    }

    let child = match crate::utils::spawn_with_pgid(cmd) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to spawn embedding subprocess: {}", e);
            return ProblemDetail::bad_gateway("embedding service unavailable").into_response();
        }
    };

    let pid = child.id().expect("child should have a pid");
    let mut wait_task = tokio::spawn(async move { child.wait_with_output().await });

    let output = match tokio::time::timeout(
        std::time::Duration::from_secs(embed_timeout),
        &mut wait_task,
    )
    .await
    {
        Ok(Ok(Ok(o))) => o,
        Ok(Ok(Err(e))) => {
            tracing::error!("embedding subprocess error: {}", e);
            return ProblemDetail::bad_gateway("embedding service error").into_response();
        }
        Ok(Err(e)) => {
            tracing::error!("embedding subprocess join error: {}", e);
            return ProblemDetail::bad_gateway("embedding service error").into_response();
        }
        Err(_) => {
            tracing::warn!("embedding query timed out");
            crate::utils::kill_pgid(pid);
            let _ = wait_task.await;
            return ProblemDetail::gateway_timeout("embedding service timed out").into_response();
        }
    };

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(error_output) = parse_embed_text_error(&stdout) {
            tracing::warn!(
                status = %output.status,
                stage = %error_output.error.stage,
                kind = %error_output.error.kind,
                stderr = %stderr,
                "embedding subprocess failed"
            );
            let detail = embed_text_stage_detail(&error_output.error.stage)
                .unwrap_or("embedding service failed");
            return ProblemDetail::bad_gateway(detail).into_response();
        }

        tracing::warn!(
            status = %output.status,
            stderr = %stderr,
            "embedding subprocess failed with unstructured output"
        );
        return ProblemDetail::bad_gateway("embedding service failed").into_response();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let embed_output: EmbedTextOutput = match parse_embed_text_output(&stdout) {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };

    let rewritten_query = embed_output
        .rewritten
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let embedding = embed_output.embedding;

    let pool = state.ro_pool.clone();
    let over_fetch = state.config.embedding.over_fetch_factor;

    let result = tokio::task::spawn_blocking(move || {
        let k = (limit * over_fetch).min(200);
        let knn_results = crate::db::embeddings::knn_search(&pool, &embedding, k);

        let mut results: Vec<SimilarResult> = knn_results
            .into_iter()
            .map(|(s, pid, distance)| {
                let similarity = 1.0 - distance;
                (s, pid, similarity)
            })
            .filter(|(_, _, sim)| *sim >= threshold)
            .filter(|(s, _, _)| {
                source_filter
                    .as_ref()
                    .is_none_or(|filters| filters.iter().any(|f| f == s))
            })
            .take(limit as usize)
            .map(|(s, pid, similarity)| {
                let problem = crate::db::problems::get_problem(&pool, &s, &pid);
                SimilarResult {
                    source: s,
                    id: pid,
                    title: problem.as_ref().and_then(|p| p.title.clone()),
                    difficulty: problem.as_ref().and_then(|p| p.difficulty.clone()),
                    link: problem.as_ref().and_then(|p| p.link.clone()),
                    similarity,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    })
    .await
    .unwrap_or_default();
    Json(SimilarResponse {
        rewritten_query,
        results: result,
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
    use axum::response::IntoResponse;
    use tokio::sync::{RwLock, Semaphore};

    use super::*;
    use crate::config::Config;

    static PATH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn test_db_path() -> String {
        std::env::temp_dir()
            .join(format!(
                "oj-api-rs-similar-tests-{}.sqlite",
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

    fn test_state(config: Config) -> (Arc<AppState>, String) {
        crate::db::register_sqlite_vec();
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

    fn fake_uv_dir(stdout: &str, stderr: &str, exit_code: i32) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("oj-api-rs-fake-uv-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let script = format!(
            "#!/bin/sh\nprintf '%s' '{}'\nprintf '%s' '{}' >&2\nexit {}\n",
            stdout.replace('\\', "\\\\").replace('\'', "'\\''"),
            stderr.replace('\\', "\\\\").replace('\'', "'\\''"),
            exit_code
        );
        let uv_path = dir.join("uv");
        fs::write(&uv_path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&uv_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&uv_path, perms).unwrap();
        }
        dir
    }

    struct EnvPathGuard {
        old_path: Option<String>,
    }

    impl EnvPathGuard {
        fn prepend(dir: &std::path::Path) -> Self {
            let old_path = std::env::var("PATH").ok();
            let new_path = match &old_path {
                Some(path) => format!("{}:{path}", dir.display()),
                None => dir.display().to_string(),
            };
            std::env::set_var("PATH", new_path);
            Self { old_path }
        }
    }

    impl Drop for EnvPathGuard {
        fn drop(&mut self) {
            if let Some(path) = &self.old_path {
                std::env::set_var("PATH", path);
            } else {
                std::env::remove_var("PATH");
            }
        }
    }

    async fn problem_detail(response: axum::response::Response) -> (StatusCode, serde_json::Value) {
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice(&body).unwrap();
        (status, json)
    }

    #[test]
    fn embed_text_error_maps_known_stages_to_sanitized_details() {
        let cases = [
            ("config", "embedding service configuration failed"),
            ("rewrite", "query rewrite service failed"),
            ("embedding", "embedding service failed"),
            ("output", "invalid embedding response"),
        ];

        for (stage, detail) in cases {
            let stdout = format!(
                r#"{{"error":{{"stage":"{stage}","kind":"provider_error","message":"raw secret"}}}}"#
            );
            let parsed = parse_embed_text_error(&stdout).unwrap();
            assert_eq!(embed_text_stage_detail(&parsed.error.stage), Some(detail));
        }
    }

    #[test]
    fn embed_text_error_rejects_unknown_or_invalid_envelopes() {
        assert!(parse_embed_text_error(
            r#"{"error":{"stage":"unknown","kind":"provider_error","message":"raw"}}"#
        )
        .is_none());
        assert!(parse_embed_text_error("not json").is_none());
        assert!(parse_embed_text_error(r#"{"embedding":[0.1],"rewritten":"ok"}"#).is_none());
    }

    #[test]
    fn embed_text_output_preserves_success_shape() {
        let output =
            parse_embed_text_output(r#"{"embedding":[0.1,0.2],"rewritten":"rewritten"}"#).unwrap();

        assert_eq!(output.embedding, vec![0.1, 0.2]);
        assert_eq!(output.rewritten.as_deref(), Some("rewritten"));
    }

    #[tokio::test]
    async fn embed_text_output_invalid_json_returns_invalid_embedding_response() {
        let err = parse_embed_text_output("not json").unwrap_err();
        let (status, json) = problem_detail(err.into_response()).await;

        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(json["detail"], "invalid embedding response");
    }

    #[test]
    fn embed_text_unknown_failure_stage_keeps_generic_fallback() {
        let stdout = r#"{"error":{"stage":"provider","kind":"provider_error","message":"secret"}}"#;

        assert!(parse_embed_text_error(stdout).is_none());
        assert_eq!(
            ProblemDetail::bad_gateway("embedding service failed").detail,
            "embedding service failed"
        );
    }

    async fn call_similar_by_text_with_fake_uv(
        stdout: &str,
        stderr: &str,
        exit_code: i32,
    ) -> (StatusCode, serde_json::Value) {
        let fake_dir = fake_uv_dir(stdout, stderr, exit_code);
        let _path_guard = EnvPathGuard::prepend(&fake_dir);
        let (state, db_path) = test_state(Config::default());
        let response = similar_by_text(
            State(state),
            Json(SimilarByTextQuery {
                query: Some("binary search".to_string()),
                limit: None,
                threshold: None,
                source: None,
            }),
        )
        .await
        .into_response();
        let result = problem_detail(response).await;
        cleanup_db_files(&db_path);
        let _ = fs::remove_dir_all(fake_dir);
        result
    }

    #[tokio::test(flavor = "current_thread")]
    async fn similar_by_text_maps_subprocess_rewrite_stage_to_sanitized_detail() {
        let _guard = PATH_LOCK.lock().await;
        let (status, json) = call_similar_by_text_with_fake_uv(
            r#"{"error":{"stage":"rewrite","kind":"provider_error","message":"secret provider message"}}"#,
            "stack trace with secret provider message",
            1,
        )
        .await;

        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(json["detail"], "query rewrite service failed");
        assert!(!json.to_string().contains("secret provider message"));
        assert!(!json.to_string().contains("stack trace"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn similar_by_text_keeps_generic_fallback_for_unknown_subprocess_stage() {
        let _guard = PATH_LOCK.lock().await;
        let (status, json) = call_similar_by_text_with_fake_uv(
            r#"{"error":{"stage":"provider","kind":"provider_error","message":"secret provider message"}}"#,
            "stack trace with secret provider message",
            1,
        )
        .await;

        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(json["detail"], "embedding service failed");
        assert!(!json.to_string().contains("secret provider message"));
        assert!(!json.to_string().contains("stack trace"));
    }
}
