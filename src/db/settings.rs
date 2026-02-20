use rusqlite::params;

use super::DbPool;

pub fn get_setting(pool: &DbPool, key: &str) -> Option<String> {
    let conn = pool.get().ok()?;
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .ok()
}

pub fn set_setting(pool: &DbPool, key: &str, value: &str) -> bool {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return false,
    };
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )
    .is_ok()
}

pub fn get_token_auth_enabled(pool: &DbPool) -> bool {
    get_setting(pool, "token_auth_enabled")
        .map(|v| v == "1")
        .unwrap_or(true)
}
