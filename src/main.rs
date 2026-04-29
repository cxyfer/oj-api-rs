use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use axum::routing::get;
use axum::{Extension, Json, Router};
use tokio::signal;
use tokio::sync::{RwLock, Semaphore};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};


mod admin;
mod api;
mod auth;
mod config;
mod db;
mod detect;
mod health;
mod home;
mod mcp;
mod models;
mod utils;

pub struct AppState {
    pub ro_pool: db::DbPool,
    pub rw_pool: db::DbPool,
    pub config: config::Config,
    pub crawler_jobs: tokio::sync::Mutex<HashMap<String, models::CrawlerJob>>,
    pub manual_crawler_guard: tokio::sync::Mutex<Option<String>>,
    pub crawler_history: tokio::sync::Mutex<VecDeque<models::CrawlerJob>>,
    pub embedding_lock: tokio::sync::Mutex<Option<models::EmbeddingJob>>,
    pub embedding_launch_guard: tokio::sync::Mutex<Option<String>>,
    pub embedding_history: tokio::sync::Mutex<VecDeque<models::EmbeddingJob>>,
    pub active_crawler_pids: tokio::sync::Mutex<HashMap<String, models::ActiveCrawlerPid>>,
    pub active_embedding_pid: tokio::sync::Mutex<Option<u32>>,
    pub daily_fallback: tokio::sync::Mutex<HashMap<String, models::DailyFallbackEntry>>,
    pub retained_refresh: tokio::sync::Mutex<utils::RetainedRefreshState>,
    pub embed_semaphore: Semaphore,
    pub token_auth_enabled: Arc<AtomicBool>,
    pub admin_sessions: Arc<RwLock<HashMap<String, i64>>>,
    pub config_path: Option<String>,
}

#[tokio::main]
async fn main() {
    // 1. Load config
    let config = config::Config::load();

    // 2. Set RUST_LOG from config (only if not already set)
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", &config.logging.rust_log);
    }

    // 3. Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // 4. Ensure data directory exists
    db::ensure_data_dir(&config.database.path);

    // 5. Register sqlite-vec
    db::register_sqlite_vec();

    // 6. Build pools
    let ro_pool = db::create_ro_pool(
        &config.database.path,
        config.database.pool_max_size,
        config.database.busy_timeout_ms,
    );
    let rw_pool = db::create_rw_pool(
        &config.database.path,
        2, // admin operations are infrequent
        config.database.busy_timeout_ms,
    );

    // 7. Ensure tables exist
    db::ensure_data_tables(&rw_pool);
    db::ensure_api_tokens_table(&rw_pool);
    db::ensure_app_settings_table(&rw_pool);

    // 8. Read initial settings
    let auth_enabled = db::settings::get_token_auth_enabled(&rw_pool);

    // 9. Startup self-check
    health::startup_self_check(&ro_pool);

    // 10. Build shared auth state
    let admin_sessions = Arc::new(RwLock::new(HashMap::<String, i64>::new()));
    let token_auth_flag = Arc::new(AtomicBool::new(auth_enabled));
    let mut retained_jobs = crate::utils::RetainedJobState::default();
    if let Err(err) = crate::utils::reconcile_retained_job_state(
        crate::models::JOB_ARTIFACTS_ROOT,
        &std::collections::HashSet::new(),
        &mut retained_jobs.crawler_history,
        &mut retained_jobs.embedding_history,
    )
    .await
    {
        tracing::warn!(
            "failed to reconstruct retained job history at startup: {}",
            err
        );
    }
    let retained_refresh = tokio::sync::Mutex::new(crate::utils::RetainedRefreshState {
        last_summary_sync: Some(tokio::time::Instant::now()),
        last_cleanup: Some(tokio::time::Instant::now()),
    });

    // 11. Build AppState
    let config_path_for_children = Some(config.config_path.to_string_lossy().into_owned());
    let state = Arc::new(AppState {
        ro_pool: ro_pool.clone(),
        rw_pool,
        config: config.clone(),
        crawler_jobs: tokio::sync::Mutex::new(HashMap::new()),
        manual_crawler_guard: tokio::sync::Mutex::new(None),
        crawler_history: tokio::sync::Mutex::new(retained_jobs.crawler_history),
        embedding_lock: tokio::sync::Mutex::new(None),
        embedding_launch_guard: tokio::sync::Mutex::new(None),
        embedding_history: tokio::sync::Mutex::new(retained_jobs.embedding_history),
        active_crawler_pids: tokio::sync::Mutex::new(HashMap::new()),
        active_embedding_pid: tokio::sync::Mutex::new(None),
        daily_fallback: tokio::sync::Mutex::new(HashMap::new()),
        retained_refresh,
        embed_semaphore: Semaphore::new(config.embedding.concurrency as usize),
        token_auth_enabled: token_auth_flag.clone(),
        admin_sessions: admin_sessions.clone(),
        config_path: config_path_for_children,
    });

    // 12. Build OpenAPI spec
    let openapi = api::openapi::ApiDoc::openapi();
    let openapi_json = serde_json::to_value(&openapi).expect("failed to serialize OpenAPI spec");

    // 13. Assemble routers
    let health_cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let shutdown_state = state.clone();
    let app = Router::new()
        // OpenAPI spec — public, no auth
        .route(
            "/openapi.json",
            get({
                let spec = openapi_json.clone();
                move || async move { Json(spec) }
            }),
        )
        // Scalar UI — public, no auth
        .merge(Scalar::with_url("/docs", openapi_json))
        // Public docs pages — no auth
        .merge(home::public_router())
        // Health check — no auth
        .route("/health", get(health::health_check))
        .layer(health_cors)
        // Public API — bearer auth + CORS
        .merge(api::public_router())
        // Admin — admin secret auth, no CORS
        .merge(admin::admin_router())
        // MCP — streamable HTTP over the same server
        .merge(mcp::router(state.clone(), &config.mcp))
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        // Extensions for auth middleware
        .layer(Extension(auth::AuthRwPool(Arc::new(db::create_rw_pool(
            &config.database.path,
            2,
            config.database.busy_timeout_ms,
        )))))
        .layer(Extension(auth::AdminSecret(
            config.server.admin_secret.clone(),
        )))
        .layer(Extension(auth::AdminSessions(admin_sessions)))
        .layer(Extension(auth::TokenAuthEnabled(token_auth_flag)))
        .with_state(state);

    // 13. Start server
    let listener = tokio::net::TcpListener::bind(&config.server.listen_addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!(
                "FATAL: failed to bind to {}: {}",
                config.server.listen_addr, e
            );
            std::process::exit(1);
        });

    tracing::info!("listening on {}", config.server.listen_addr);

    let shutdown_timeout = config.server.graceful_shutdown_secs;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_state, shutdown_timeout))
        .await
        .unwrap_or_else(|e| {
            eprintln!("server error: {}", e);
            std::process::exit(1);
        });
}

async fn shutdown_signal(state: Arc<AppState>, timeout_secs: u64) {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    cleanup_active_jobs(&state).await;

    tracing::info!(
        "shutdown signal received, waiting up to {}s for in-flight requests",
        timeout_secs
    );
}

async fn cleanup_active_jobs(state: &Arc<AppState>) {
    cleanup_active_crawler_jobs(&state.active_crawler_pids).await;
    cleanup_active_job(&state.active_embedding_pid, "embedding").await;
}

async fn cleanup_active_crawler_jobs(
    pid_lock: &tokio::sync::Mutex<HashMap<String, models::ActiveCrawlerPid>>,
) {
    let pids = {
        let mut lock = pid_lock.lock().await;
        std::mem::take(&mut *lock)
    };

    if pids.is_empty() {
        tracing::debug!("shutdown cleanup found no active crawler process");
        return;
    }

    for (runtime_key, active_pid) in pids {
        let killed = crate::utils::kill_pgid(active_pid.pid);
        if killed {
            tracing::info!(
                "shutdown cleanup killed active crawler process group for {} (job {}, pid {})",
                runtime_key,
                active_pid.job_id,
                active_pid.pid
            );
        } else {
            tracing::warn!(
                "shutdown cleanup failed to kill active crawler process group for {} (job {}, pid {})",
                runtime_key,
                active_pid.job_id,
                active_pid.pid
            );
        }
    }
}

async fn cleanup_active_job(pid_lock: &tokio::sync::Mutex<Option<u32>>, job_type: &str) {
    let pid = {
        let mut lock = pid_lock.lock().await;
        lock.take()
    };

    match pid {
        Some(pid) => {
            let killed = crate::utils::kill_pgid(pid);
            if killed {
                tracing::info!(
                    "shutdown cleanup killed active {} process group (pid {})",
                    job_type,
                    pid
                );
            } else {
                tracing::warn!(
                    "shutdown cleanup failed to kill active {} process group (pid {})",
                    job_type,
                    pid
                );
            }
        }
        None => {
            tracing::debug!("shutdown cleanup found no active {} process", job_type);
        }
    }
}
