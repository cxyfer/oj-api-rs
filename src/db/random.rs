use super::DbPool;
use crate::db::problems::row_to_problem_record;
use crate::models::ProblemRecord;

pub fn random_problems(
    pool: &DbPool,
    source: Option<&str>,
    difficulty: Option<&str>,
    tags: Option<Vec<&str>>,
    tag_mode: &str,
    rating_min: Option<f64>,
    rating_max: Option<f64>,
    count: u32,
) -> Option<Vec<ProblemRecord>> {
    let conn = pool.get().ok()?;
    let count = count.clamp(1, 20);

    let mut where_clauses: Vec<String> = Vec::new();
    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1u32;

    // Source filter (optional — unlike list_problems where source is a path param)
    if let Some(src) = source {
        where_clauses.push(format!("source = ?{}", idx));
        sql_params.push(Box::new(src.to_string()));
        idx += 1;
    }

    // Difficulty filter
    if let Some(diff) = difficulty {
        let diff_lower = diff.to_lowercase();
        match diff_lower.as_str() {
            "easy" | "medium" | "hard" => {
                let conditions = build_difficulty_conditions(
                    source,
                    diff_lower.as_str(),
                    &mut sql_params,
                    &mut idx,
                );
                if !conditions.is_empty() {
                    where_clauses.push(format!("({})", conditions.join(" OR ")));
                }
            }
            _ => {
                // Native difficulty value: case-insensitive exact match
                where_clauses.push(format!("LOWER(difficulty) = LOWER(?{})", idx));
                sql_params.push(Box::new(diff.to_string()));
                idx += 1;
            }
        }
    }

    // Tag filter (same pattern as list_problems)
    if let Some(ref tags) = tags {
        let joiner = if tag_mode == "all" { " AND " } else { " OR " };
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

    // Rating range filter (same pattern as list_problems)
    if let Some(min) = rating_min {
        where_clauses.push(format!("rating >= ?{}", idx));
        sql_params.push(Box::new(min));
        idx += 1;
    }
    if let Some(max) = rating_max {
        where_clauses.push(format!("rating <= ?{}", idx));
        sql_params.push(Box::new(max));
        idx += 1;
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!(
        "SELECT * FROM problems{} ORDER BY RANDOM() LIMIT ?{}",
        where_sql, idx
    );
    sql_params.push(Box::new(count));

    let mut stmt = conn.prepare(&sql).ok()?;
    let rows = stmt
        .query_map(
            rusqlite::params_from_iter(sql_params.iter().map(|p| p.as_ref())),
            row_to_problem_record,
        )
        .ok()?;

    Some(rows.filter_map(|r| r.ok()).collect())
}

/// Build per-source difficulty conditions for standardized values (easy/medium/hard).
/// When `source` is specified, only that source's condition is included and source
/// prefix is omitted (source already filtered). When `source` is None, conditions
/// include source checks and form an OR chain across all platforms.
fn build_difficulty_conditions(
    source: Option<&str>,
    difficulty: &str,
    sql_params: &mut Vec<Box<dyn rusqlite::types::ToSql>>,
    idx: &mut u32,
) -> Vec<String> {
    let source_already_filtered = source.is_some();
    let mut conditions: Vec<String> = Vec::new();

    // LeetCode
    if source.is_none_or(|s| s == "leetcode") {
        let value = match difficulty {
            "easy" => "Easy",
            "medium" => "Medium",
            "hard" => "Hard",
            _ => return conditions,
        };
        conditions.push(if source_already_filtered {
            format!("LOWER(difficulty) = LOWER(?{})", *idx)
        } else {
            format!("(source = 'leetcode' AND LOWER(difficulty) = LOWER(?{}))", *idx)
        });
        sql_params.push(Box::new(value.to_string()));
        *idx += 1;
    }

    // Luogu + SPOJ — share the same native difficulty values
    for src in &["luogu", "spoj"] {
        if source.is_none_or(|s| s == *src) {
            let values: &[&str] = match difficulty {
                "easy" => &["暂无评定", "入门", "普及−"],
                "medium" => &["普及/提高−", "普及+/提高"],
                "hard" => &["提高+/省选−", "省选/NOI−", "NOI/NOI+/CTSC"],
                _ => return conditions,
            };
            let mut placeholders: Vec<String> = Vec::new();
            for v in values {
                placeholders.push(format!("?{}", *idx));
                sql_params.push(Box::new(v.to_string()));
                *idx += 1;
            }
            conditions.push(if source_already_filtered {
                format!("difficulty IN ({})", placeholders.join(", "))
            } else {
                format!(
                    "(source = '{}' AND difficulty IN ({}))",
                    src,
                    placeholders.join(", ")
                )
            });
        }
    }

    // Codeforces + AtCoder — use rating ranges (difficulty column is typically NULL)
    for src in &["codeforces", "atcoder"] {
        if source.is_none_or(|s| s == *src) {
            let rating_cond = build_rating_condition(difficulty, sql_params, idx);
            conditions.push(if source_already_filtered {
                rating_cond
            } else {
                format!("(source = '{}' AND {})", src, rating_cond)
            });
        }
    }

    conditions
}

fn build_rating_condition(
    difficulty: &str,
    sql_params: &mut Vec<Box<dyn rusqlite::types::ToSql>>,
    idx: &mut u32,
) -> String {
    let (min, max) = match difficulty {
        "easy" => (None, Some(1200.0)),
        "medium" => (Some(1200.0), Some(1800.0)),
        "hard" => (Some(1800.0), None),
        _ => return String::new(),
    };
    let mut parts: Vec<String> = Vec::new();
    if let Some(min_val) = min {
        parts.push(format!("rating > ?{}", *idx));
        sql_params.push(Box::new(min_val));
        *idx += 1;
    }
    if let Some(max_val) = max {
        parts.push(format!("rating <= ?{}", *idx));
        sql_params.push(Box::new(max_val));
        *idx += 1;
    }
    parts.join(" AND ")
}
