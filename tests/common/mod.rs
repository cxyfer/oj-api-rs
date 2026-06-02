use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Once};

use axum::{Extension, Router};
use tokio::sync::{RwLock, Semaphore};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use oj_api_rs::{admin, api, auth, config, db, health, home, mcp, AppState};

static REGISTER_VEC: Once = Once::new();

/// Guard that cleans up temp DB files (including WAL/SHM) on drop.
pub struct TestGuard {
    db_path: PathBuf,
}

impl TestGuard {
    pub fn db_path(&self) -> &std::path::Path {
        &self.db_path
    }
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.db_path);
        let _ = std::fs::remove_file(format!("{}-wal", self.db_path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", self.db_path.display()));
    }
}

/// Build a test app with a temporary SQLite database file.
/// Returns a `(Router, TestGuard)` — hold the guard to ensure cleanup on drop.
pub fn build_test_app() -> (Router, TestGuard) {
    REGISTER_VEC.call_once(|| {
        db::register_sqlite_vec();
    });

    let mut config = config::Config::default();
    config.server.admin_secret = "test-secret".to_string();

    // Use a UUID-based temp file so parallel tests don't conflict
    let db_path = std::env::temp_dir().join(format!("oj-api-test-{}.db", Uuid::new_v4()));
    let db_path_str = db_path.to_string_lossy().into_owned();

    let ro_pool = db::create_ro_pool(&db_path_str, 1, config.database.busy_timeout_ms);
    let rw_pool = db::create_rw_pool(&db_path_str, 2, config.database.busy_timeout_ms);

    // Ensure tables exist on the rw pool
    db::ensure_data_tables(&rw_pool);
    db::ensure_api_tokens_table(&rw_pool);
    db::ensure_app_settings_table(&rw_pool);

    let admin_sessions = Arc::new(RwLock::new(HashMap::<String, i64>::new()));
    let token_auth_flag = Arc::new(AtomicBool::new(false)); // disabled for easier testing

    let state = Arc::new(AppState {
        ro_pool,
        rw_pool: rw_pool.clone(),
        config: config.clone(),
        crawler_jobs: tokio::sync::Mutex::new(HashMap::new()),
        manual_crawler_guard: tokio::sync::Mutex::new(None),
        crawler_history: tokio::sync::Mutex::new(VecDeque::new()),
        embedding_lock: tokio::sync::Mutex::new(None),
        embedding_launch_guard: tokio::sync::Mutex::new(None),
        embedding_history: tokio::sync::Mutex::new(VecDeque::new()),
        active_crawler_pids: tokio::sync::Mutex::new(HashMap::new()),
        active_embedding_pid: tokio::sync::Mutex::new(None),
        daily_fallback: tokio::sync::Mutex::new(HashMap::new()),
        retained_refresh: tokio::sync::Mutex::new(oj_api_rs::utils::RetainedRefreshState::default()),
        embed_semaphore: Semaphore::new(1),
        token_auth_enabled: token_auth_flag.clone(),
        admin_sessions: admin_sessions.clone(),
        config_path: None,
    });

    let health_cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(home::public_router())
        .route("/health", axum::routing::get(health::health_check))
        .layer(health_cors)
        .merge(api::public_router())
        .merge(admin::admin_router())
        .merge(mcp::router(state.clone(), &config.mcp))
        .layer(Extension(auth::AuthRwPool(Arc::new(rw_pool))))
        .layer(Extension(auth::AdminSecret(
            config.server.admin_secret.clone(),
        )))
        .layer(Extension(auth::AdminSessions(admin_sessions)))
        .layer(Extension(auth::TokenAuthEnabled(token_auth_flag)))
        .with_state(state.clone());

    let guard = TestGuard { db_path };

    (app, guard)
}
