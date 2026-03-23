use rusqlite::params;

use super::DbPool;
use crate::models::{parse_similar_question_slugs, parse_string_array, DailyChallengeRecord};
pub fn get_daily_record(pool: &DbPool, domain: &str, date: &str) -> Option<DailyChallengeRecord> {
    let conn = pool.get().ok()?;
    conn.query_row(
        "SELECT * FROM daily_challenge WHERE domain = ?1 AND date = ?2",
        params![domain, date],
        |row| {
            let tags_raw: Option<String> = row.get("tags")?;
            let similar_raw: Option<String> = row.get("similar_questions")?;
            let id_val: rusqlite::types::Value = row.get("id")?;
            let id_str = match id_val {
                rusqlite::types::Value::Integer(i) => i.to_string(),
                rusqlite::types::Value::Text(s) => s,
                _ => String::new(),
            };
            Ok(DailyChallengeRecord {
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
                tags: tags_raw
                    .as_deref()
                    .map(parse_string_array)
                    .unwrap_or_default(),
                link: row.get("link")?,
                category: row.get("category")?,
                paid_only: row.get("paid_only")?,
                content: row.get("content")?,
                content_cn: row.get("content_cn")?,
                similar_questions: similar_raw
                    .as_deref()
                    .map(parse_similar_question_slugs)
                    .unwrap_or_default(),
            })
        },
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::params;

    use super::get_daily_record;
    use crate::db::{create_ro_pool, create_rw_pool, ensure_data_tables, DbPool};

    fn test_db_path() -> String {
        std::env::temp_dir()
            .join(format!(
                "oj-api-rs-daily-tests-{}.sqlite",
                uuid::Uuid::new_v4()
            ))
            .to_string_lossy()
            .into_owned()
    }

    fn setup_pools() -> (DbPool, DbPool, String) {
        crate::db::register_sqlite_vec();
        let path = test_db_path();
        let rw_pool = create_rw_pool(&path, 1, 1000);
        ensure_data_tables(&rw_pool);
        let ro_pool = create_ro_pool(&path, 1, 1000);
        (rw_pool, ro_pool, path)
    }

    #[test]
    fn get_daily_record_parses_legacy_similar_questions() {
        let (rw_pool, ro_pool, path) = setup_pools();
        let legacy_raw = r#"[{"titleSlug":"two-sum"},{"title_slug":"3sum"}]"#;

        {
            let conn = rw_pool.get().unwrap();
            conn.execute(
                "INSERT INTO daily_challenge (date, domain, id, slug, title, tags, similar_questions) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params!["2026-03-20", "com", 1, "sample", "Sample", "[]", legacy_raw],
            )
            .unwrap();
        }

        let record = get_daily_record(&ro_pool, "com", "2026-03-20").unwrap();
        assert_eq!(record.similar_questions, vec!["two-sum", "3sum"]);

        let _ = fs::remove_file(path);
    }
}
