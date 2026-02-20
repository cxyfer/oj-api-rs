use rusqlite::params;
use zerocopy::AsBytes;

use super::DbPool;

pub fn get_embedding(pool: &DbPool, source: &str, id: &str) -> Option<Vec<f32>> {
    let conn = pool.get().ok()?;
    let raw: Vec<u8> = conn
        .query_row(
            "SELECT embedding FROM vec_embeddings WHERE source = ?1 AND problem_id = ?2",
            params![source, id],
            |row| row.get(0),
        )
        .ok()?;

    // Try binary LE f32 parse first
    if raw.len() % 4 == 0 {
        let floats: Vec<f32> = raw
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();
        if !floats.is_empty() {
            return Some(floats);
        }
    }

    // Fallback: try JSON string parse
    let text = String::from_utf8(raw).ok()?;
    serde_json::from_str::<Vec<f32>>(&text).ok()
}

pub fn knn_search(
    pool: &DbPool,
    embedding: &[f32],
    k: u32,
) -> Vec<(String, String, f32)> {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let bytes = embedding.as_bytes();
    let mut stmt = match conn.prepare(
        "SELECT source, problem_id, distance \
         FROM vec_embeddings \
         WHERE embedding MATCH ?1 AND k = ?2",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = match stmt.query_map(params![bytes, k], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f32>(2)?,
        ))
    }) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    rows.filter_map(|r| r.ok()).collect()
}
