use std::sync::Arc;

use axum::routing::get;
use axum::{Extension, Router};
use tokio::signal;
use tokio::sync::Semaphore;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

mod admin;
mod api;
mod auth;
mod config;
mod db;
mod detect;
mod health;
mod models;

pub struct AppState {
    pub ro_pool: db::DbPool,
    pub rw_pool: db::DbPool,
    pub config: config::Config,
    pub crawler_lock: tokio::sync::Mutex<Option<models::CrawlerJob>>,
    pub embed_semaphore: Semaphore,
}

#[tokio::main]
async fn main() {
    // 1. Load config
    let config = config::Config::from_env();

    // 2. Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // 3. Register sqlite-vec
    db::register_sqlite_vec();

    // 4. Build pools
    let ro_pool = db::create_ro_pool(
        &config.database_path,
        config.db_pool_max_size,
        config.busy_timeout_ms,
    );
    let rw_pool = db::create_rw_pool(
        &config.database_path,
        2, // admin operations are infrequent
        config.busy_timeout_ms,
    );

    // 5. Ensure api_tokens table exists
    db::ensure_api_tokens_table(&rw_pool);

    // 6. Startup self-check
    health::startup_self_check(&ro_pool);

    // 7. Build AppState
    let state = Arc::new(AppState {
        ro_pool: ro_pool.clone(),
        rw_pool,
        config: config.clone(),
        crawler_lock: tokio::sync::Mutex::new(None),
        embed_semaphore: Semaphore::new(4),
    });

    // 8. Assemble routers
    let health_cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Health check — no auth
        .route("/health", get(health::health_check))
        .layer(health_cors)
        // Public API — bearer auth + CORS
        .merge(api::public_router())
        // Admin — admin secret auth, no CORS
        .merge(admin::admin_router())
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        // Extensions for auth middleware
        .layer(Extension(auth::AuthRwPool(Arc::new(
            db::create_rw_pool(&config.database_path, 2, config.busy_timeout_ms),
        ))))
        .layer(Extension(auth::AdminSecret(config.admin_secret.clone())))
        .with_state(state);

    // 9. Start server
    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("FATAL: failed to bind to {}: {}", config.listen_addr, e);
            std::process::exit(1);
        });

    tracing::info!("listening on {}", config.listen_addr);

    let shutdown_timeout = config.graceful_shutdown_secs;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_timeout))
        .await
        .unwrap_or_else(|e| {
            eprintln!("server error: {}", e);
            std::process::exit(1);
        });
}

async fn shutdown_signal(timeout_secs: u64) {
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

    tracing::info!(
        "shutdown signal received, waiting up to {}s for in-flight requests",
        timeout_secs
    );
}
