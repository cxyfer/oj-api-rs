use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::DbPool;
use crate::models::{DailyChallengeRecord, ProblemRecord};

const COMPACT_DAILY_CHALLENGE_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS daily_challenge (
    date TEXT NOT NULL,
    source TEXT NOT NULL,
    problems TEXT NOT NULL,
    PRIMARY KEY (date, source)
)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DailyChallengeSchema {
    Missing,
    Compact,
    Legacy,
    Unsupported,
}

pub(crate) fn ensure_daily_challenge_table(conn: &mut Connection) -> rusqlite::Result<()> {
    match detect_daily_challenge_schema(conn)? {
        DailyChallengeSchema::Missing => create_compact_daily_challenge_table(conn),
        DailyChallengeSchema::Compact => Ok(()),
        DailyChallengeSchema::Legacy => rebuild_legacy_daily_challenge(conn),
        DailyChallengeSchema::Unsupported => Err(rusqlite::Error::InvalidQuery),
    }
}

fn detect_daily_challenge_schema(conn: &Connection) -> rusqlite::Result<DailyChallengeSchema> {
    let columns = table_columns(conn, "daily_challenge")?;

    if columns.is_empty() {
        return Ok(DailyChallengeSchema::Missing);
    }
    if ["date", "source", "problems"]
        .into_iter()
        .all(|column| columns.contains(column))
    {
        return Ok(DailyChallengeSchema::Compact);
    }
    if ["date", "domain", "id", "slug"]
        .into_iter()
        .all(|column| columns.contains(column))
    {
        return Ok(DailyChallengeSchema::Legacy);
    }
    Ok(DailyChallengeSchema::Unsupported)
}

fn table_columns(conn: &Connection, table: &str) -> rusqlite::Result<HashSet<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<HashSet<_>>>()?;
    Ok(columns)
}

fn create_compact_daily_challenge_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(COMPACT_DAILY_CHALLENGE_SCHEMA)
}

fn rebuild_legacy_daily_challenge(conn: &mut Connection) -> rusqlite::Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    rebuild_legacy_daily_challenge_inner(&tx)?;
    tx.commit()
}

fn legacy_daily_select_sql(columns: &HashSet<String>) -> String {
    const OPTIONAL_COLUMNS: &[&str] = &[
        "title",
        "title_cn",
        "difficulty",
        "ac_rate",
        "rating",
        "contest",
        "problem_index",
        "tags",
        "link",
        "category",
        "paid_only",
        "content",
        "content_cn",
        "similar_questions",
    ];

    let mut selected = vec![
        "date".to_string(),
        "domain".to_string(),
        "id".to_string(),
        "slug".to_string(),
    ];
    selected.extend(OPTIONAL_COLUMNS.iter().map(|column| {
        if columns.contains(*column) {
            (*column).to_string()
        } else {
            format!("NULL AS {column}")
        }
    }));
    format!(
        "SELECT {} FROM daily_challenge_legacy_migration ORDER BY date, domain",
        selected.join(", ")
    )
}

fn rebuild_legacy_daily_challenge_inner(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "DROP TABLE IF EXISTS daily_challenge_legacy_migration;
         ALTER TABLE daily_challenge RENAME TO daily_challenge_legacy_migration;",
    )?;
    create_compact_daily_challenge_table(conn)?;

    let legacy_columns = table_columns(conn, "daily_challenge_legacy_migration")?;
    let select_sql = legacy_daily_select_sql(&legacy_columns);
    let legacy_rows = {
        let mut stmt = conn.prepare(&select_sql)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LegacyDailyRow {
                    date: row.get("date")?,
                    domain: row.get("domain")?,
                    id: row.get("id")?,
                    slug: row.get("slug")?,
                    title: row.get("title")?,
                    title_cn: row.get("title_cn")?,
                    difficulty: row.get("difficulty")?,
                    ac_rate: row.get("ac_rate")?,
                    rating: row.get("rating")?,
                    contest: row.get("contest")?,
                    problem_index: row.get("problem_index")?,
                    tags: row.get("tags")?,
                    link: row.get("link")?,
                    category: row.get("category")?,
                    paid_only: row.get("paid_only")?,
                    content: row.get("content")?,
                    content_cn: row.get("content_cn")?,
                    similar_questions: row.get("similar_questions")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    for row in legacy_rows {
        let Some(problem_id) = legacy_problem_id(conn, &row)? else {
            tracing::warn!(
                "skipping legacy daily_challenge row for date '{}' domain '{}' because it has no convertible problem id",
                row.date,
                row.domain
            );
            continue;
        };
        seed_legacy_problem_if_missing(conn, &problem_id, &row)?;
        let refs = serde_json::to_string(&[format!("leetcode:{problem_id}")])
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
        conn.execute(
            "INSERT OR REPLACE INTO daily_challenge (date, source, problems) VALUES (?1, ?2, ?3)",
            params![
                row.date,
                canonical_daily_source_from_legacy_domain(&row.domain),
                refs
            ],
        )?;
    }

    conn.execute_batch("DROP TABLE daily_challenge_legacy_migration")
}

#[derive(Debug)]
struct LegacyDailyRow {
    date: String,
    domain: String,
    id: Option<rusqlite::types::Value>,
    slug: Option<String>,
    title: Option<String>,
    title_cn: Option<String>,
    difficulty: Option<String>,
    ac_rate: Option<f64>,
    rating: Option<f64>,
    contest: Option<String>,
    problem_index: Option<String>,
    tags: Option<String>,
    link: Option<String>,
    category: Option<String>,
    paid_only: Option<i32>,
    content: Option<String>,
    content_cn: Option<String>,
    similar_questions: Option<String>,
}

fn seed_legacy_problem_if_missing(
    conn: &Connection,
    problem_id: &str,
    row: &LegacyDailyRow,
) -> rusqlite::Result<()> {
    let Some(slug) = row.slug.as_deref().filter(|slug| !slug.trim().is_empty()) else {
        return Ok(());
    };
    conn.execute(
        "INSERT OR IGNORE INTO problems (
            id, source, slug, title, title_cn, difficulty, ac_rate, rating,
            contest, problem_index, tags, link, category, paid_only, content,
            content_cn, similar_questions
         ) VALUES (
            ?1, 'leetcode', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            COALESCE(?10, '[]'), ?11, ?12, ?13, ?14, ?15, COALESCE(?16, '[]')
         )",
        params![
            problem_id,
            slug,
            row.title.as_deref(),
            row.title_cn.as_deref(),
            row.difficulty.as_deref(),
            row.ac_rate,
            row.rating,
            row.contest.as_deref(),
            row.problem_index.as_deref(),
            row.tags.as_deref(),
            row.link.as_deref(),
            row.category.as_deref(),
            row.paid_only,
            row.content.as_deref(),
            row.content_cn.as_deref(),
            row.similar_questions.as_deref(),
        ],
    )?;
    Ok(())
}

fn legacy_problem_id(conn: &Connection, row: &LegacyDailyRow) -> rusqlite::Result<Option<String>> {
    if let Some(id) = row.id.as_ref().and_then(sql_value_to_non_empty_string) {
        return Ok(Some(id));
    }

    let Some(slug) = row.slug.as_deref().filter(|slug| !slug.trim().is_empty()) else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT id FROM problems WHERE source = 'leetcode' AND slug = ?1 LIMIT 1",
        params![slug],
        |row| row.get::<_, String>(0),
    )
    .optional()
}

fn sql_value_to_non_empty_string(value: &rusqlite::types::Value) -> Option<String> {
    let value = match value {
        rusqlite::types::Value::Integer(value) => value.to_string(),
        rusqlite::types::Value::Real(value) => value.to_string(),
        rusqlite::types::Value::Text(value) => value.clone(),
        _ => return None,
    };
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn canonical_daily_source_from_legacy_domain(domain: &str) -> String {
    match domain {
        "com" => "leetcode.com".to_string(),
        "cn" => "leetcode.cn".to_string(),
        other => other.to_string(),
    }
}

pub fn get_daily_record(pool: &DbPool, source: &str, date: &str) -> Option<DailyChallengeRecord> {
    let conn = pool.get().ok()?;
    let problems_raw: String = conn
        .query_row(
            "SELECT problems FROM daily_challenge WHERE source = ?1 AND date = ?2",
            params![source, date],
            |row| row.get(0),
        )
        .ok()?;
    let refs = parse_problem_refs(&problems_raw)?;
    let problems = resolve_problem_refs(&conn, &refs);
    if problems.is_empty() {
        return None;
    }
    Some(DailyChallengeRecord {
        date: date.to_string(),
        source: source.to_string(),
        problems,
    })
}

fn parse_problem_refs(raw: &str) -> Option<Vec<(String, String)>> {
    let refs = serde_json::from_str::<Vec<String>>(raw).ok()?;
    let mut parsed = Vec::with_capacity(refs.len());
    for reference in refs {
        let Some((source, id)) = reference.split_once(':') else {
            tracing::warn!("skipping malformed daily problem ref '{}'", reference);
            continue;
        };
        let source = source.trim();
        let id = id.trim();
        if source.is_empty() || id.is_empty() {
            tracing::warn!("skipping malformed daily problem ref '{}'", reference);
            continue;
        }
        parsed.push((source.to_string(), id.to_string()));
    }
    Some(parsed)
}

fn resolve_problem_refs(conn: &Connection, refs: &[(String, String)]) -> Vec<ProblemRecord> {
    let mut problems = Vec::with_capacity(refs.len());
    for (source, id) in refs {
        match conn.query_row(
            "SELECT * FROM problems WHERE source = ?1 AND id = ?2",
            params![source, id],
            crate::db::problems::row_to_problem_record,
        ) {
            Ok(problem) => problems.push(problem),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                tracing::debug!("failed to resolve daily problem ref '{}:{}'", source, id)
            }
            Err(err) => tracing::warn!(
                "failed to resolve daily problem ref '{}:{}': {}",
                source,
                id,
                err
            ),
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::params;

    use super::{ensure_daily_challenge_table, get_daily_record};
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

    fn create_minimal_problems_table(conn: &rusqlite::Connection) {
        conn.execute_batch(
            "CREATE TABLE problems (
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
            )",
        )
        .unwrap();
    }

    fn insert_problem(conn: &rusqlite::Connection, id: &str, source: &str, slug: &str) {
        conn.execute(
            "INSERT INTO problems (id, source, slug, title, tags, similar_questions) VALUES (?1, ?2, ?3, ?4, '[]', '[]')",
            params![id, source, slug, format!("Problem {id}")],
        )
        .unwrap();
    }

    #[test]
    fn ensure_daily_challenge_table_creates_compact_table() {
        crate::db::register_sqlite_vec();
        let path = test_db_path();
        let pool = create_rw_pool(&path, 1, 1000);
        let mut conn = pool.get().unwrap();
        create_minimal_problems_table(&conn);

        ensure_daily_challenge_table(&mut conn).unwrap();

        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(daily_challenge)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(columns, vec!["date", "source", "problems"]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn ensure_daily_challenge_table_keeps_compact_rows() {
        let (rw_pool, _ro_pool, path) = setup_pools();
        let mut conn = rw_pool.get().unwrap();
        conn.execute(
            "INSERT INTO daily_challenge (date, source, problems) VALUES (?1, ?2, ?3)",
            params!["2026-01-01", "leetcode.com", "[\"leetcode:1\"]"],
        )
        .unwrap();

        ensure_daily_challenge_table(&mut conn).unwrap();

        let problems: String = conn
            .query_row(
                "SELECT problems FROM daily_challenge WHERE date = '2026-01-01' AND source = 'leetcode.com'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(problems, "[\"leetcode:1\"]");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn ensure_daily_challenge_table_migrates_legacy_rows() {
        crate::db::register_sqlite_vec();
        let path = test_db_path();
        let pool = create_rw_pool(&path, 1, 1000);
        let mut conn = pool.get().unwrap();
        create_minimal_problems_table(&conn);
        conn.execute_batch(
            "CREATE TABLE daily_challenge (
                date TEXT NOT NULL,
                domain TEXT NOT NULL,
                id INTEGER,
                slug TEXT NOT NULL,
                PRIMARY KEY (date, domain)
            )",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO daily_challenge (date, domain, id, slug) VALUES (?1, ?2, ?3, ?4)",
            params!["2026-01-01", "com", 1234, "daily-com"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO daily_challenge (date, domain, id, slug) VALUES (?1, ?2, ?3, ?4)",
            params!["2026-01-02", "cn", 1, "daily-cn"],
        )
        .unwrap();

        ensure_daily_challenge_table(&mut conn).unwrap();

        let rows: Vec<(String, String, String)> = conn
            .prepare("SELECT date, source, problems FROM daily_challenge ORDER BY date")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                (
                    "2026-01-01".to_string(),
                    "leetcode.com".to_string(),
                    "[\"leetcode:1234\"]".to_string()
                ),
                (
                    "2026-01-02".to_string(),
                    "leetcode.cn".to_string(),
                    "[\"leetcode:1\"]".to_string()
                ),
            ]
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn ensure_daily_challenge_table_seeds_legacy_snapshot_when_problem_row_is_missing() {
        crate::db::register_sqlite_vec();
        let path = test_db_path();
        let pool = create_rw_pool(&path, 1, 1000);
        let mut conn = pool.get().unwrap();
        create_minimal_problems_table(&conn);
        conn.execute_batch(
            "CREATE TABLE daily_challenge (
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
            )",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO daily_challenge (
                date, domain, id, slug, title, title_cn, difficulty, ac_rate,
                rating, contest, problem_index, tags, link, category, paid_only,
                content, content_cn, similar_questions
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                "2026-01-01",
                "com",
                1234,
                "legacy-only",
                "Legacy Only",
                "舊快取",
                "Easy",
                50.0,
                1200.0,
                "weekly-contest-1",
                "A",
                "[\"Array\"]",
                "https://leetcode.com/problems/legacy-only/",
                "Algorithms",
                0,
                "legacy content",
                "舊內容",
                "[\"two-sum\"]",
            ],
        )
        .unwrap();

        ensure_daily_challenge_table(&mut conn).unwrap();
        drop(conn);
        let ro_pool = create_ro_pool(&path, 1, 1000);

        let record = get_daily_record(&ro_pool, "leetcode.com", "2026-01-01").unwrap();
        let problem = &record.problems[0];
        assert_eq!(problem.id, "1234");
        assert_eq!(problem.slug, "legacy-only");
        assert_eq!(problem.title.as_deref(), Some("Legacy Only"));
        assert_eq!(problem.content.as_deref(), Some("legacy content"));
        assert_eq!(problem.tags, vec!["Array"]);
        assert_eq!(problem.similar_questions, vec!["two-sum"]);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn ensure_daily_challenge_table_migrates_slug_fallback_and_skips_unconvertible_rows() {
        crate::db::register_sqlite_vec();
        let path = test_db_path();
        let pool = create_rw_pool(&path, 1, 1000);
        let mut conn = pool.get().unwrap();
        create_minimal_problems_table(&conn);
        insert_problem(&conn, "42", "leetcode", "two-sum");
        conn.execute_batch(
            "CREATE TABLE daily_challenge (
                date TEXT NOT NULL,
                domain TEXT NOT NULL,
                id INTEGER,
                slug TEXT NOT NULL,
                PRIMARY KEY (date, domain)
            )",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO daily_challenge (date, domain, id, slug) VALUES (?1, ?2, NULL, ?3)",
            params!["2026-01-01", "com", "two-sum"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO daily_challenge (date, domain, id, slug) VALUES (?1, ?2, NULL, ?3)",
            params!["2026-01-02", "cn", "missing"],
        )
        .unwrap();

        ensure_daily_challenge_table(&mut conn).unwrap();

        let rows: Vec<(String, String, String)> = conn
            .prepare("SELECT date, source, problems FROM daily_challenge ORDER BY date")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![(
                "2026-01-01".to_string(),
                "leetcode.com".to_string(),
                "[\"leetcode:42\"]".to_string()
            )]
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn get_daily_record_resolves_refs_in_order() {
        let (rw_pool, ro_pool, path) = setup_pools();
        {
            let conn = rw_pool.get().unwrap();
            insert_problem(&conn, "1234", "leetcode", "daily-a");
            insert_problem(&conn, "1", "leetcode", "daily-b");
            insert_problem(&conn, "abc321_a", "atcoder", "abc321-a");
            conn.execute(
                "INSERT INTO daily_challenge (date, source, problems) VALUES (?1, ?2, ?3)",
                params![
                    "2026-03-20",
                    "leetcode.com",
                    "[\"leetcode:1234\",\"leetcode:1\",\"atcoder:abc321_a\"]"
                ],
            )
            .unwrap();
        }

        let record = get_daily_record(&ro_pool, "leetcode.com", "2026-03-20").unwrap();
        let ids: Vec<_> = record
            .problems
            .iter()
            .map(|problem| problem.id.as_str())
            .collect();
        assert_eq!(ids, vec!["1234", "1", "abc321_a"]);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn get_daily_record_treats_unusable_rows_as_missing() {
        let (rw_pool, ro_pool, path) = setup_pools();
        {
            let conn = rw_pool.get().unwrap();
            conn.execute(
                "INSERT INTO daily_challenge (date, source, problems) VALUES (?1, ?2, ?3)",
                params!["2026-03-20", "leetcode.com", "not-json"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO daily_challenge (date, source, problems) VALUES (?1, ?2, ?3)",
                params![
                    "2026-03-21",
                    "leetcode.com",
                    "[\"bad-ref\",\"leetcode:404\"]"
                ],
            )
            .unwrap();
        }

        assert!(get_daily_record(&ro_pool, "leetcode.com", "2026-03-20").is_none());
        assert!(get_daily_record(&ro_pool, "leetcode.com", "2026-03-21").is_none());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn get_daily_record_splits_refs_on_first_colon() {
        let (rw_pool, ro_pool, path) = setup_pools();
        {
            let conn = rw_pool.get().unwrap();
            insert_problem(&conn, "abc:123", "custom", "colon-id");
            conn.execute(
                "INSERT INTO daily_challenge (date, source, problems) VALUES (?1, ?2, ?3)",
                params!["2026-03-20", "custom.daily", "[\"custom:abc:123\"]"],
            )
            .unwrap();
        }

        let record = get_daily_record(&ro_pool, "custom.daily", "2026-03-20").unwrap();
        assert_eq!(record.problems[0].source, "custom");
        assert_eq!(record.problems[0].id, "abc:123");

        let _ = fs::remove_file(path);
    }
}
