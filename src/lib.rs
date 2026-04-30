use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::{RwLock, Semaphore};

pub mod admin;
pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod detect;
pub mod health;
pub mod home;
pub mod mcp;
pub mod models;
pub mod utils;

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
