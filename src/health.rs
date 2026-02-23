use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::AppState;

pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pool = state.ro_pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(_) => {
                return json!({
                    "status": "unhealthy",
                    "db": false,
                    "sqlite_vec": false,
                    "vec_dimension": null
                });
            }
        };

        let vec_version: Option<String> = conn
            .query_row("SELECT vec_version()", [], |row| row.get(0))
            .ok();

        let vec_dim: Option<i64> = conn
            .query_row(
                "SELECT vec_length(embedding) FROM vec_embeddings LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        let db_ok = true;
        let vec_ok = vec_version.is_some();
        let dim_ok = vec_dim == Some(768);

        json!({
            "status": if db_ok && vec_ok && dim_ok { "ok" } else { "unhealthy" },
            "db": db_ok,
            "sqlite_vec": vec_ok,
            "vec_dimension": vec_dim,
            "version": vec_version,
        })
    })
    .await;

    match result {
        Ok(v) => {
            let status = if v["status"] == "ok" {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };
            (status, Json(v)).into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "unhealthy",
                "db": false,
                "sqlite_vec": false,
                "vec_dimension": null
            })),
        )
            .into_response(),
    }
}

pub fn startup_self_check(pool: &crate::db::DbPool) {
    let conn = pool.get().unwrap_or_else(|e| {
        eprintln!("FATAL: failed to connect to database: {}", e);
        std::process::exit(1);
    });

    let vec_version: String = conn
        .query_row("SELECT vec_version()", [], |row| row.get(0))
        .unwrap_or_else(|e| {
            eprintln!("FATAL: sqlite-vec not loaded: {}", e);
            std::process::exit(1);
        });

    tracing::info!("sqlite-vec version: {}", vec_version);

    match conn.query_row(
        "SELECT vec_length(embedding) FROM vec_embeddings LIMIT 1",
        [],
        |row| row.get::<_, i64>(0),
    ) {
        Ok(dim) if dim == 768 => {
            tracing::info!("vec dimension check passed: {}", dim);
        }
        Ok(dim) => {
            tracing::warn!("vec dimension mismatch: expected 768, got {}", dim);
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            tracing::warn!("vec_embeddings is empty, skipping dimension check");
        }
        Err(e) => {
            tracing::warn!("vec_embeddings not queryable, skipping dimension check: {}", e);
        }
    }
}
