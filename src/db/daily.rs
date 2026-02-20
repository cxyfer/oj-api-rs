use rusqlite::params;

use super::DbPool;
use crate::models::DailyChallenge;

fn parse_json_array(raw: Option<String>) -> Vec<String> {
    match raw {
        Some(s) if !s.is_empty() => {
            serde_json::from_str::<Vec<String>>(&s).unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

pub fn get_daily(pool: &DbPool, domain: &str, date: &str) -> Option<DailyChallenge> {
    let conn = pool.get().ok()?;
    conn.query_row(
        "SELECT * FROM daily_challenge WHERE domain = ?1 AND date = ?2",
        params![domain, date],
        |row| {
            let tags_raw: Option<String> = row.get("tags")?;
            let similar_raw: Option<String> = row.get("similar_questions")?;
            // daily_challenge.id is INTEGER in existing schema, read as i64 then convert
            let id_val: rusqlite::types::Value = row.get("id")?;
            let id_str = match id_val {
                rusqlite::types::Value::Integer(i) => i.to_string(),
                rusqlite::types::Value::Text(s) => s,
                _ => String::new(),
            };
            Ok(DailyChallenge {
                date: row.get("date")?,
                domain: row.get("domain")?,
                id: id_str,
                slug: row.get("slug")?,
                title: row.get("title")?,
                title_cn: row.get("title_cn")?,
                difficulty: row.get("difficulty")?,
                ac_rate: row.get("ac_rate")?,
                rating: row.get("rating")?,
                contest: row.get("contest")?,
                problem_index: row.get("problem_index")?,
                tags: parse_json_array(tags_raw),
                link: row.get("link")?,
                category: row.get("category")?,
                paid_only: row.get("paid_only")?,
                content: row.get("content")?,
                content_cn: row.get("content_cn")?,
                similar_questions: parse_json_array(similar_raw),
            })
        },
    )
    .ok()
}
