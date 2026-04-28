use std::collections::{HashMap, HashSet};

use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};

use super::DbPool;
use crate::models::{
    parse_similar_question_slugs, parse_string_array, Problem, ProblemRecord, ProblemSummary,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformStats {
    pub source: String,
    pub total: u32,
    pub missing_content: u32,
    pub not_embedded: u32,
}

pub fn platform_stats(pool: &DbPool) -> Vec<PlatformStats> {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut stmt = match conn.prepare(
        "SELECT p.source, COUNT(*) AS total, \
         SUM(CASE WHEN p.content IS NULL OR p.content = '' THEN 1 ELSE 0 END) AS missing_content, \
         SUM(CASE WHEN pe.problem_id IS NULL THEN 1 ELSE 0 END) AS not_embedded \
         FROM problems p \
         LEFT JOIN problem_embeddings pe ON pe.source = p.source AND pe.problem_id = p.id \
         GROUP BY p.source ORDER BY p.source",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |row| {
        Ok(PlatformStats {
            source: row.get(0)?,
            total: row.get(1)?,
            missing_content: row.get(2)?,
            not_embedded: row.get(3)?,
        })
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn row_to_problem_record(row: &Row<'_>) -> rusqlite::Result<ProblemRecord> {
    let tags_raw: Option<String> = row.get("tags")?;
    let similar_raw: Option<String> = row.get("similar_questions")?;
    Ok(ProblemRecord {
        id: row.get("id")?,
        source: row.get("source")?,
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
}

fn row_to_summary(row: &Row<'_>) -> rusqlite::Result<ProblemSummary> {
    let tags_raw: Option<String> = row.get("tags")?;
    Ok(ProblemSummary {
        id: row.get("id")?,
        source: row.get("source")?,
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
    })
}

pub fn get_problem_id_by_slug(pool: &DbPool, source: &str, slug: &str) -> Option<String> {
    let conn = pool.get().ok()?;
    conn.query_row(
        "SELECT id FROM problems WHERE source = ?1 AND slug = ?2 LIMIT 1",
        params![source, slug],
        |row| row.get(0),
    )
    .ok()
}

pub fn get_problem_record(pool: &DbPool, source: &str, id: &str) -> Option<ProblemRecord> {
    get_problem_record_result(pool, source, id).ok().flatten()
}

pub fn get_problem_record_result(
    pool: &DbPool,
    source: &str,
    id: &str,
) -> rusqlite::Result<Option<ProblemRecord>> {
    let conn = pool.get().map_err(|e| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some(e.to_string()),
        )
    })?;
    match conn.query_row(
        "SELECT * FROM problems WHERE source = ?1 AND id = ?2",
        params![source, id],
        row_to_problem_record,
    ) {
        Ok(record) => Ok(Some(record)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_problem(pool: &DbPool, source: &str, id: &str) -> Option<Problem> {
    get_problem_record(pool, source, id).map(Into::into)
}

pub fn resolve_similar_question_summaries(
    pool: &DbPool,
    source: &str,
    slugs: &[String],
) -> Vec<ProblemSummary> {
    if slugs.is_empty() {
        return Vec::new();
    }

    let conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };

    let slugs_json = match serde_json::to_string(slugs) {
        Ok(json) => json,
        Err(_) => return Vec::new(),
    };

    let mut stmt = match conn.prepare(
        "SELECT id, source, slug, title, title_cn, difficulty, ac_rate, rating, \
         contest, problem_index, tags, link \
         FROM problems \
         WHERE source = ?1 \
           AND slug IN (SELECT value FROM json_each(?2))",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };
    let rows = match stmt.query_map(params![source, slugs_json], row_to_summary) {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };

    let resolved_by_slug: HashMap<String, ProblemSummary> = rows
        .filter_map(|row| row.ok())
        .map(|summary| (summary.slug.clone(), summary))
        .collect();

    let mut missing_logged = HashSet::new();
    let mut resolved = Vec::with_capacity(slugs.len());
    for slug in slugs {
        if let Some(summary) = resolved_by_slug.get(slug) {
            resolved.push(summary.clone());
        } else if missing_logged.insert(slug.clone()) {
            tracing::debug!(
                "failed to resolve similar question slug '{}' for source '{}'",
                slug,
                source
            );
        }
    }

    resolved
}

pub struct ListParams<'a> {
    pub source: &'a str,
    pub page: u32,
    pub per_page: u32,
    pub difficulty: Option<&'a str>,
    pub tags: Option<Vec<&'a str>>,
    pub search: Option<&'a str>,
    pub sort_by: Option<&'a str>,
    pub sort_order: Option<&'a str>,
    pub tag_mode: &'a str,
    pub rating_min: Option<f64>,
    pub rating_max: Option<f64>,
}

pub struct ListResult {
    pub data: Vec<ProblemSummary>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

pub fn list_problems(pool: &DbPool, params: &ListParams<'_>) -> Option<ListResult> {
    let conn = pool.get().ok()?;
    let per_page = params.per_page.clamp(1, 100);
    let page = params.page.max(1);
    let offset = (page - 1).saturating_mul(per_page);

    let mut where_clauses = vec!["source = ?1".to_string()];
    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> =
        vec![Box::new(params.source.to_string())];
    let mut idx = 2u32;

    if let Some(diff) = params.difficulty {
        where_clauses.push(format!("LOWER(difficulty) = LOWER(?{})", idx));
        sql_params.push(Box::new(diff.to_string()));
        idx += 1;
    }

    if let Some(search) = params.search {
        let trimmed = search.trim();
        if !trimmed.is_empty() {
            let escaped = trimmed
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            let like_val = format!("%{}%", escaped);
            where_clauses.push(format!(
                "(id LIKE ?{i} ESCAPE '\\' OR COALESCE(title,'') LIKE ?{j} ESCAPE '\\' OR COALESCE(title_cn,'') LIKE ?{k} ESCAPE '\\')",
                i = idx, j = idx + 1, k = idx + 2
            ));
            sql_params.push(Box::new(like_val.clone()));
            sql_params.push(Box::new(like_val.clone()));
            sql_params.push(Box::new(like_val));
            idx += 3;
        }
    }

    if let Some(ref tags) = params.tags {
        let joiner = if params.tag_mode == "all" {
            " AND "
        } else {
            " OR "
        };
        let tag_conditions: Vec<String> = tags
            .iter()
            .map(|tag| {
                let cond = format!(
                    "EXISTS (SELECT 1 FROM json_each(CASE WHEN tags IS NOT NULL AND tags != '' AND json_valid(tags) THEN tags ELSE '[]' END) WHERE LOWER(value) = LOWER(?{}))",
                    idx
                );
                sql_params.push(Box::new(tag.to_string()));
                idx += 1;
                cond
            })
            .collect();
        if !tag_conditions.is_empty() {
            where_clauses.push(format!("({})", tag_conditions.join(joiner)));
        }
    }

    if let Some(min) = params.rating_min {
        where_clauses.push(format!("rating >= ?{}", idx));
        sql_params.push(Box::new(min));
        idx += 1;
    }
    if let Some(max) = params.rating_max {
        where_clauses.push(format!("rating <= ?{}", idx));
        sql_params.push(Box::new(max));
        idx += 1;
    }

    let where_sql = where_clauses.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM problems WHERE {}", where_sql);
    let total: u32 = conn
        .query_row(
            &count_sql,
            rusqlite::params_from_iter(sql_params.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        )
        .ok()?;

    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(per_page)
    };

    let order_col = match params.sort_by {
        Some("difficulty") => {
            "CASE WHEN LOWER(difficulty)='easy' THEN 1 WHEN LOWER(difficulty)='medium' THEN 2 WHEN LOWER(difficulty)='hard' THEN 3 ELSE 4 END"
        }
        Some("rating") => "rating",
        Some("ac_rate") => "ac_rate",
        Some("id") => "natural_sort_key(id)",
        _ => "natural_sort_key(id)",
    };
    let order_dir = match params.sort_by {
        Some(_) => match params.sort_order {
            Some("desc") => "DESC",
            _ => "ASC",
        },
        None => "ASC",
    };

    let select_sql = format!(
        "SELECT id, source, slug, title, title_cn, difficulty, ac_rate, rating, \
         contest, problem_index, tags, link \
         FROM problems WHERE {} ORDER BY {} {}, id {} LIMIT ?{} OFFSET ?{}",
        where_sql,
        order_col,
        order_dir,
        order_dir,
        idx,
        idx + 1
    );
    sql_params.push(Box::new(per_page));
    sql_params.push(Box::new(offset));

    let mut stmt = conn.prepare(&select_sql).ok()?;
    let rows = stmt
        .query_map(
            rusqlite::params_from_iter(sql_params.iter().map(|p| p.as_ref())),
            row_to_summary,
        )
        .ok()?;

    let data: Vec<ProblemSummary> = rows.filter_map(|r| r.ok()).collect();

    Some(ListResult {
        data,
        total,
        page,
        per_page,
        total_pages,
    })
}

pub fn insert_problem(pool: &DbPool, p: &Problem) -> rusqlite::Result<()> {
    let conn = pool.get().map_err(|e| {
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
    })?;
    let tags_json = serde_json::to_string(&p.tags).unwrap_or_default();
    let similar_json = serde_json::to_string(&p.similar_questions).unwrap_or_default();
    conn.execute(
        "INSERT INTO problems (id, source, slug, title, title_cn, difficulty, ac_rate, rating, \
         contest, problem_index, tags, link, category, paid_only, content, content_cn, similar_questions) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            p.id,
            p.source,
            p.slug,
            p.title,
            p.title_cn,
            p.difficulty,
            p.ac_rate,
            p.rating,
            p.contest,
            p.problem_index,
            tags_json,
            p.link,
            p.category,
            p.paid_only,
            p.content,
            p.content_cn,
            similar_json
        ],
    )?;
    Ok(())
}

pub fn update_problem(
    pool: &DbPool,
    source: &str,
    id: &str,
    p: &Problem,
) -> rusqlite::Result<usize> {
    let conn = pool.get().map_err(|e| {
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
    })?;
    let tags_json = serde_json::to_string(&p.tags).unwrap_or_default();
    let similar_json = serde_json::to_string(&p.similar_questions).unwrap_or_default();
    conn.execute(
        "UPDATE problems SET slug=?1, title=?2, title_cn=?3, difficulty=?4, ac_rate=?5, \
         rating=?6, contest=?7, problem_index=?8, tags=?9, link=?10, category=?11, \
         paid_only=?12, content=?13, content_cn=?14, similar_questions=?15 \
         WHERE source=?16 AND id=?17",
        params![
            p.slug,
            p.title,
            p.title_cn,
            p.difficulty,
            p.ac_rate,
            p.rating,
            p.contest,
            p.problem_index,
            tags_json,
            p.link,
            p.category,
            p.paid_only,
            p.content,
            p.content_cn,
            similar_json,
            source,
            id
        ],
    )
}

pub fn delete_problem(pool: &DbPool, source: &str, id: &str) -> rusqlite::Result<bool> {
    let conn = pool.get().map_err(|e| {
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), Some(e.to_string()))
    })?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM vec_embeddings WHERE source = ?1 AND problem_id = ?2",
        params![source, id],
    )?;
    tx.execute(
        "DELETE FROM problem_embeddings WHERE source = ?1 AND problem_id = ?2",
        params![source, id],
    )?;
    let affected = tx.execute(
        "DELETE FROM problems WHERE source = ?1 AND id = ?2",
        params![source, id],
    )?;
    tx.commit()?;
    Ok(affected > 0)
}

pub fn list_tags(pool: &DbPool, source: &str) -> Option<Vec<String>> {
    let conn = pool.get().ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT LOWER(TRIM(je.value)) AS tag \
             FROM problems p, json_each(\
                 CASE WHEN p.tags IS NOT NULL AND p.tags != '' AND json_valid(p.tags) \
                      THEN p.tags ELSE '[]' END\
             ) je \
             WHERE p.source = ?1 AND TRIM(je.value) != '' \
             ORDER BY tag ASC",
        )
        .ok()?;
    let rows = stmt
        .query_map(params![source], |row| row.get::<_, String>(0))
        .ok()?;
    Some(rows.filter_map(|r| r.ok()).collect())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::params;

    use super::{get_problem_record, resolve_similar_question_summaries};
    use crate::db::{create_ro_pool, create_rw_pool, ensure_data_tables, DbPool};
    use crate::models::{parse_similar_question_slugs, Problem};

    fn test_db_path() -> String {
        std::env::temp_dir()
            .join(format!(
                "oj-api-rs-db-tests-{}.sqlite",
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
    fn parse_similar_questions_accepts_object_array() {
        let raw = r#"[{"titleSlug":"two-sum"},{"title_slug":"3sum"},{"slug":"4sum"}]"#;
        assert_eq!(
            parse_similar_question_slugs(raw),
            vec!["two-sum", "3sum", "4sum"]
        );
    }

    #[test]
    fn get_problem_record_parses_legacy_similar_questions() {
        let (rw_pool, ro_pool, path) = setup_pools();
        let legacy_raw = r#"[{"titleSlug":"two-sum"},{"titleSlug":"valid-parentheses"}]"#;

        {
            let conn = rw_pool.get().unwrap();
            conn.execute(
                "INSERT INTO problems (id, source, slug, title, tags, similar_questions) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params!["1", "leetcode", "sample", "Sample", "[]", legacy_raw],
            )
            .unwrap();
        }

        let record = get_problem_record(&ro_pool, "leetcode", "1").unwrap();
        assert_eq!(
            record.similar_questions,
            vec!["two-sum", "valid-parentheses"]
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn resolve_similar_question_summaries_preserves_order_and_duplicates() {
        let (rw_pool, ro_pool, path) = setup_pools();

        for (id, slug, title) in [
            ("1", "alpha", "Alpha"),
            ("2", "beta", "Beta"),
            ("3", "gamma", "Gamma"),
        ] {
            super::insert_problem(
                &rw_pool,
                &Problem {
                    id: id.to_string(),
                    source: "leetcode".to_string(),
                    slug: slug.to_string(),
                    title: Some(title.to_string()),
                    title_cn: None,
                    difficulty: None,
                    ac_rate: None,
                    rating: None,
                    contest: None,
                    problem_index: None,
                    tags: Vec::new(),
                    link: None,
                    category: None,
                    paid_only: None,
                    content: None,
                    content_cn: None,
                    similar_questions: Vec::new(),
                },
            )
            .unwrap();
        }

        let resolved = resolve_similar_question_summaries(
            &ro_pool,
            "leetcode",
            &[
                "beta".to_string(),
                "missing".to_string(),
                "alpha".to_string(),
                "beta".to_string(),
            ],
        );

        let resolved_slugs: Vec<&str> = resolved
            .iter()
            .map(|summary| summary.slug.as_str())
            .collect();
        assert_eq!(resolved_slugs, vec!["beta", "alpha", "beta"]);

        let _ = fs::remove_file(path);
    }
}
