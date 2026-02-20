use std::sync::Arc;

use tokio::sync::Semaphore;

mod admin;
mod api;
mod auth;
mod config;
mod db;
mod detect;
mod models;

pub struct AppState {
    pub ro_pool: db::DbPool,
    pub rw_pool: db::DbPool,
    pub config: config::Config,
    pub crawler_lock: tokio::sync::Mutex<Option<models::CrawlerJob>>,
    pub embed_semaphore: Semaphore,
}

fn main() {
    println!("oj-api-rs scaffold");
}
