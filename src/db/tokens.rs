use rand::Rng;
use rusqlite::params;

use super::DbPool;
use crate::models::ApiToken;

pub fn validate_token(pool: &DbPool, token: &str) -> bool {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM api_tokens WHERE token = ?1 AND is_active = 1",
            params![token],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if exists {
        let now = chrono::Utc::now().timestamp();
        let _ = conn.execute(
            "UPDATE api_tokens SET last_used_at = ?1 WHERE token = ?2",
            params![now, token],
        );
    }
    exists
}

pub fn list_tokens(pool: &DbPool) -> Vec<ApiToken> {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut stmt = match conn.prepare(
        "SELECT token, label, created_at, last_used_at, is_active FROM api_tokens ORDER BY created_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = match stmt.query_map([], |row| {
        Ok(ApiToken {
            token: row.get(0)?,
            label: row.get(1)?,
            created_at: row.get(2)?,
            last_used_at: row.get(3)?,
            is_active: row.get(4)?,
        })
    }) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    rows.filter_map(|r| r.ok()).collect()
}

pub fn create_token(pool: &DbPool, label: Option<&str>) -> Option<ApiToken> {
    let conn = pool.get().ok()?;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    let token: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "INSERT INTO api_tokens (token, label, created_at, is_active) VALUES (?1, ?2, ?3, 1)",
        params![token, label, now],
    )
    .ok()?;

    Some(ApiToken {
        token,
        label: label.map(String::from),
        created_at: now,
        last_used_at: None,
        is_active: 1,
    })
}

pub fn revoke_token(pool: &DbPool, token: &str) -> Option<bool> {
    let conn = pool.get().ok()?;
    let affected = conn
        .execute(
            "UPDATE api_tokens SET is_active = 0 WHERE token = ?1 AND is_active = 1",
            params![token],
        )
        .ok()?;
    Some(affected > 0)
}
