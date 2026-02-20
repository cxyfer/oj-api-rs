use std::env;
use std::net::SocketAddr;

#[derive(Clone)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub database_path: String,
    pub admin_secret: String,
    pub gemini_api_key: Option<String>,
    pub db_pool_max_size: u32,
    pub busy_timeout_ms: u64,
    pub embed_timeout_secs: u64,
    pub crawler_timeout_secs: u64,
    pub over_fetch_factor: u32,
    pub graceful_shutdown_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        let admin_secret = env::var("ADMIN_SECRET").unwrap_or_else(|_| {
            eprintln!("FATAL: ADMIN_SECRET environment variable is required");
            std::process::exit(1);
        });

        Self {
            listen_addr: env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3000".into())
                .parse()
                .expect("invalid LISTEN_ADDR"),
            database_path: env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "data/data.db".into()),
            admin_secret,
            gemini_api_key: env::var("GEMINI_API_KEY").ok(),
            db_pool_max_size: env::var("DB_POOL_MAX_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8),
            busy_timeout_ms: env::var("BUSY_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            embed_timeout_secs: env::var("EMBED_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            crawler_timeout_secs: env::var("CRAWLER_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            over_fetch_factor: env::var("OVER_FETCH_FACTOR")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4),
            graceful_shutdown_secs: env::var("GRACEFUL_SHUTDOWN_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        }
    }
}
