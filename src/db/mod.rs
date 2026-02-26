use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::ffi::sqlite3_auto_extension;
use sqlite_vec::sqlite3_vec_init;

pub mod daily;
pub mod embeddings;
pub mod problems;
pub mod settings;
pub mod tokens;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn register_sqlite_vec() {
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(
                *mut rusqlite::ffi::sqlite3,
                *mut *mut i8,
                *const rusqlite::ffi::sqlite3_api_routines,
            ) -> i32,
        >(sqlite3_vec_init as *const ())));
    }
}

pub fn create_ro_pool(path: &str, max_size: u32, busy_timeout_ms: u64) -> DbPool {
    let manager = SqliteConnectionManager::file(path).with_init(move |conn| {
        conn.execute_batch(&format!(
            "PRAGMA journal_mode=WAL;\
             PRAGMA busy_timeout={};\
             PRAGMA query_only=ON;",
            busy_timeout_ms
        ))?;
        Ok(())
    });
    Pool::builder()
        .max_size(max_size)
        .build(manager)
        .expect("failed to create read-only pool")
}

pub fn create_rw_pool(path: &str, max_size: u32, busy_timeout_ms: u64) -> DbPool {
    let manager = SqliteConnectionManager::file(path).with_init(move |conn| {
        conn.execute_batch(&format!(
            "PRAGMA journal_mode=WAL;\
             PRAGMA busy_timeout={};",
            busy_timeout_ms
        ))?;
        Ok(())
    });
    Pool::builder()
        .max_size(max_size)
        .build(manager)
        .expect("failed to create read-write pool")
}

pub fn ensure_data_dir(path: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!(
                "FATAL: failed to create database directory {:?}: {}",
                parent, e
            );
            std::process::exit(1);
        });
    }
}

pub fn ensure_data_tables(pool: &DbPool) {
    let conn = pool.get().expect("failed to get connection");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS problems (
            id TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'leetcode',
            slug TEXT NOT NULL,
            title TEXT,
            title_cn TEXT,
            difficulty TEXT,
            ac_rate REAL,
            rating REAL,
            contest TEXT,
            problem_index TEXT,
            tags TEXT,
            link TEXT,
            category TEXT,
            paid_only INTEGER,
            content TEXT,
            content_cn TEXT,
            similar_questions TEXT,
            PRIMARY KEY (source, id)
        );
        CREATE TABLE IF NOT EXISTS daily_challenge (
            date TEXT NOT NULL,
            domain TEXT NOT NULL,
            id INTEGER,
            slug TEXT NOT NULL,
            title TEXT,
            title_cn TEXT,
            difficulty TEXT,
            ac_rate REAL,
            rating REAL,
            contest TEXT,
            problem_index TEXT,
            tags TEXT,
            link TEXT,
            category TEXT,
            paid_only INTEGER,
            content TEXT,
            content_cn TEXT,
            similar_questions TEXT,
            PRIMARY KEY (date, domain)
        );
        CREATE TABLE IF NOT EXISTS problem_embeddings (
            source TEXT NOT NULL,
            problem_id TEXT NOT NULL,
            rewritten_content TEXT,
            model TEXT NOT NULL,
            dim INTEGER NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (source, problem_id)
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS vec_embeddings USING vec0(
            source TEXT,
            problem_id TEXT,
            embedding float[768]
        );
        CREATE INDEX IF NOT EXISTS idx_problems_source_slug ON problems(source, slug);",
    )
    .expect("failed to create data tables");
}

pub fn ensure_app_settings_table(pool: &DbPool) {
    let conn = pool.get().expect("failed to get connection");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        INSERT OR IGNORE INTO app_settings (key, value) VALUES ('token_auth_enabled', '1');",
    )
    .expect("failed to create app_settings table");
}

pub fn ensure_api_tokens_table(pool: &DbPool) {
    let conn = pool.get().expect("failed to get connection");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS api_tokens (
            token TEXT PRIMARY KEY,
            label TEXT,
            created_at INTEGER NOT NULL,
            last_used_at INTEGER,
            is_active INTEGER NOT NULL DEFAULT 1
        );",
    )
    .expect("failed to create api_tokens table");
}
