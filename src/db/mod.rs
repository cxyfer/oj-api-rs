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
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite3_vec_init as *const (),
        )));
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
