use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::error::ProblemDetail;
use crate::AppState;

#[derive(Deserialize)]
pub struct SimilarByProblemQuery {
    pub limit: Option<u32>,
    pub threshold: Option<f32>,
    pub source: Option<String>,
}

#[derive(Deserialize)]
pub struct SimilarByTextQuery {
    pub query: Option<String>,
    pub limit: Option<u32>,
    pub threshold: Option<f32>,
    pub source: Option<String>,
}

#[derive(Serialize)]
struct SimilarResult {
    source: String,
    id: String,
    title: Option<String>,
    difficulty: Option<String>,
    link: Option<String>,
    similarity: f32,
}

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
    let over_fetch = state.config.over_fetch_factor;

    let source_clone = source.clone();
    let id_clone = id.clone();

    let result = tokio::task::spawn_blocking(move || {
        let embedding =
            match crate::db::embeddings::get_embedding(&pool, &source_clone, &id_clone) {
                Some(e) => e,
                None => return Err(ProblemDetail::not_found("no embedding found for this problem")),
            };

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
                    .map_or(true, |filters| filters.iter().any(|f| f == s))
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

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    })
    .await
    .unwrap_or(Err(ProblemDetail::internal("task join error")));

    match result {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn similar_by_text(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SimilarByTextQuery>,
) -> impl IntoResponse {
    let text = match &query.query {
        Some(q) if q.len() >= 3 && q.len() <= 2000 => q.clone(),
        Some(q) if q.len() > 2000 => {
            return ProblemDetail::bad_request("query must be at most 2000 characters")
                .into_response();
        }
        Some(_) => {
            return ProblemDetail::bad_request("query must be at least 3 characters")
                .into_response();
        }
        None => {
            return ProblemDetail::bad_request("query parameter is required").into_response();
        }
    };

    let limit = query.limit.unwrap_or(10).min(50);
    let threshold = query.threshold.unwrap_or(0.0);
    let source_filter: Option<Vec<String>> = query
        .source
        .as_ref()
        .map(|s| s.split(',').map(|v| v.trim().to_string()).collect());

    let embed_timeout = state.config.embed_timeout_secs;
    let gemini_key = state.config.gemini_api_key.clone();

    // Acquire semaphore permit
    let _permit = match state.embed_semaphore.acquire().await {
        Ok(p) => p,
        Err(_) => {
            return ProblemDetail::internal("semaphore closed").into_response();
        }
    };

    // Spawn Python subprocess
    let mut cmd = tokio::process::Command::new("python3");
    cmd.args(["embedding_cli.py", "--embed-text", &text]);
    cmd.current_dir("references/leetcode-daily-discord-bot/");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    if let Some(key) = &gemini_key {
        cmd.env("GEMINI_API_KEY", key);
    }

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to spawn embedding subprocess: {}", e);
            return ProblemDetail::bad_gateway("embedding service unavailable").into_response();
        }
    };

    let output = match tokio::time::timeout(
        std::time::Duration::from_secs(embed_timeout),
        child.wait_with_output(),
    )
    .await
    {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            tracing::error!("embedding subprocess error: {}", e);
            return ProblemDetail::bad_gateway("embedding service error").into_response();
        }
        Err(_) => {
            return ProblemDetail::gateway_timeout("embedding service timed out").into_response();
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("embedding subprocess stderr: {}", stderr);
        return ProblemDetail::bad_gateway("embedding service failed").into_response();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let embed_response: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => {
            return ProblemDetail::bad_gateway("invalid embedding response").into_response();
        }
    };

    let embedding: Vec<f32> = match embed_response.get("embedding").and_then(|v| {
        serde_json::from_value::<Vec<f32>>(v.clone()).ok()
    }) {
        Some(e) => e,
        None => {
            return ProblemDetail::bad_gateway("invalid embedding format").into_response();
        }
    };

    let pool = state.ro_pool.clone();
    let over_fetch = state.config.over_fetch_factor;

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
                    .map_or(true, |filters| filters.iter().any(|f| f == s))
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

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        results
    })
    .await
    .unwrap_or_default();

    Json(result).into_response()
}
