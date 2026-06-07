use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::api::problems::{build_problem_detail_response, ProblemDetailResponse};
use crate::models::CrawlerSource;
use crate::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectFetchPlan {
    pub(crate) db_source: String,
    pub(crate) db_id: String,
    pub(crate) crawler_source: CrawlerSource,
    pub(crate) problem_arg: String,
    pub(crate) url: String,
}

pub(crate) fn derive_direct_fetch_plan(source: &str, id: &str) -> Option<DirectFetchPlan> {
    let source = source.trim().to_ascii_lowercase();
    let id = id.trim();
    match source.as_str() {
        "codeforces" | "gym" => derive_codeforces_plan(source.as_str(), id),
        "atcoder" => derive_atcoder_plan(id),
        "luogu" => derive_luogu_plan(id),
        _ => None,
    }
}

fn derive_codeforces_plan(source: &str, id: &str) -> Option<DirectFetchPlan> {
    let (contest_id, index) = split_codeforces_id(id)?;
    let is_gym = contest_id.len() >= 6;
    if source == "gym" && !is_gym {
        return None;
    }

    let path_kind = if is_gym { "gym" } else { "contest" };
    let db_id = format!("{contest_id}{index}");
    Some(DirectFetchPlan {
        db_source: "codeforces".to_string(),
        db_id: db_id.clone(),
        crawler_source: CrawlerSource::Codeforces,
        problem_arg: db_id,
        url: format!("https://codeforces.com/{path_kind}/{contest_id}/problem/{index}"),
    })
}

fn split_codeforces_id(id: &str) -> Option<(String, String)> {
    let trimmed = id.trim();
    let digit_len = trimmed
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_len == 0 || digit_len == trimmed.len() {
        return None;
    }
    let (contest_id, index) = trimmed.split_at(digit_len);
    if !index
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic())
    {
        return None;
    }
    if !index.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return None;
    }
    Some((contest_id.to_string(), index.to_ascii_uppercase()))
}

fn derive_atcoder_plan(id: &str) -> Option<DirectFetchPlan> {
    let id = id.trim().to_ascii_lowercase();
    if id.is_empty()
        || !id.contains('_')
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    let contest_id = derive_atcoder_contest_id(&id)?;
    Some(DirectFetchPlan {
        db_source: "atcoder".to_string(),
        db_id: id.clone(),
        crawler_source: CrawlerSource::AtCoder,
        problem_arg: id.clone(),
        url: format!("https://atcoder.jp/contests/{contest_id}/tasks/{id}"),
    })
}

fn derive_atcoder_contest_id(problem_id: &str) -> Option<String> {
    let mut parts = problem_id.split('_');
    let prefix = parts.next()?;
    if prefix.is_empty() {
        return None;
    }
    if let Some(month) = prefix.strip_prefix("past") {
        if month.len() == 6 && month.bytes().all(|byte| byte.is_ascii_digit()) {
            return Some(format!("{prefix}-open"));
        }
    }
    if prefix.starts_with("arc") {
        if let Some(abc_contest) = parts.next() {
            if let Some(abc_number) = abc_contest.strip_prefix("abc") {
                if !abc_number.is_empty() && abc_number.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Some(abc_contest.to_string());
                }
            }
        }
    }
    Some(prefix.to_string())
}

fn derive_luogu_plan(id: &str) -> Option<DirectFetchPlan> {
    let id = id.trim();
    let digits = id.strip_prefix('P').or_else(|| id.strip_prefix('p'))?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let id = format!("P{digits}");
    Some(DirectFetchPlan {
        db_source: "luogu".to_string(),
        db_id: id.clone(),
        crawler_source: CrawlerSource::Luogu,
        problem_arg: id.clone(),
        url: format!("https://www.luogu.com.cn/problem/{id}"),
    })
}

pub(crate) async fn fetch_problem_on_miss(
    state: Arc<AppState>,
    source: &str,
    id: &str,
) -> Option<ProblemDetailResponse> {
    let plan = derive_direct_fetch_plan(source, id)?;
    if !run_single_problem_crawler(&state, &plan).await {
        return None;
    }

    let pool = state.ro_pool.clone();
    let db_source = plan.db_source;
    let db_id = plan.db_id;
    tokio::task::spawn_blocking(move || {
        crate::db::problems::get_problem_record(&pool, &db_source, &db_id)
            .map(|record| build_problem_detail_response(&pool, record))
    })
    .await
    .ok()
    .flatten()
}

fn absolute_path(path: &str) -> PathBuf {
    let path = Path::new(path);
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_relative() {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        } else {
            path.to_path_buf()
        }
    })
}

async fn run_single_problem_crawler(state: &Arc<AppState>, plan: &DirectFetchPlan) -> bool {
    #[cfg(test)]
    match state.config.database.path.as_str() {
        "__dynamic_fetch_mock_success__" => {
            let problem = crate::models::Problem {
                id: plan.db_id.clone(),
                source: plan.db_source.clone(),
                slug: plan.db_id.clone(),
                title: Some(plan.db_id.clone()),
                title_cn: None,
                difficulty: None,
                ac_rate: None,
                rating: None,
                contest: None,
                problem_index: None,
                tags: Vec::new(),
                link: Some(plan.url.clone()),
                category: Some("Algorithms".to_string()),
                paid_only: Some(0),
                content: Some("mock fetched content".to_string()),
                content_cn: None,
                similar_questions: Vec::new(),
            };
            return crate::db::problems::insert_problem(&state.rw_pool, &problem).is_ok();
        }
        "__dynamic_fetch_mock_fail__" => return false,
        _ => {}
    }

    let args = vec!["--problem".to_string(), plan.problem_arg.clone()];
    let args = match crate::models::validate_args(&plan.crawler_source, &args) {
        Ok(args) => args,
        Err(err) => {
            tracing::warn!("invalid dynamic crawler args: {}", err);
            return false;
        }
    };

    let db_path = absolute_path(&state.config.database.path);
    let mut cmd = tokio::process::Command::new("uv");
    cmd.args(["run", "python3", plan.crawler_source.script_name()]);
    cmd.args(args);
    cmd.arg("--db-path");
    cmd.arg(db_path);
    cmd.current_dir("scripts/");
    cmd.kill_on_drop(true);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());
    if let Some(ref config_path) = state.config_path {
        cmd.env("CONFIG_PATH", config_path);
    }

    let timeout_secs = state
        .config
        .crawler
        .per_source_timeout
        .get(&plan.db_source)
        .copied()
        .unwrap_or(state.config.crawler.timeout_secs);

    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output()).await {
        Ok(Ok(output)) if output.status.success() => true,
        Ok(Ok(output)) => {
            tracing::warn!(
                "dynamic crawler failed for {}:{}: {}",
                plan.db_source,
                plan.db_id,
                String::from_utf8_lossy(&output.stderr)
            );
            false
        }
        Ok(Err(err)) => {
            tracing::warn!("failed to run dynamic crawler: {}", err);
            false
        }
        Err(_) => {
            tracing::warn!(
                "dynamic crawler timed out for {}:{}",
                plan.db_source,
                plan.db_id
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::derive_direct_fetch_plan;

    #[test]
    fn derives_codeforces_contest_problem_url() {
        let plan = derive_direct_fetch_plan("codeforces", "1988A").unwrap();
        assert_eq!(plan.db_source, "codeforces");
        assert_eq!(plan.db_id, "1988A");
        assert_eq!(plan.url, "https://codeforces.com/contest/1988/problem/A");
    }

    #[test]
    fn derives_codeforces_gym_problem_url_from_long_contest_id() {
        let plan = derive_direct_fetch_plan("codeforces", "102951A").unwrap();
        assert_eq!(plan.db_id, "102951A");
        assert_eq!(plan.url, "https://codeforces.com/gym/102951/problem/A");
    }

    #[test]
    fn derives_explicit_gym_problem_url() {
        let plan = derive_direct_fetch_plan("gym", "102951A").unwrap();
        assert_eq!(plan.db_source, "codeforces");
        assert_eq!(plan.url, "https://codeforces.com/gym/102951/problem/A");
    }

    #[test]
    fn derives_atcoder_problem_url() {
        let plan = derive_direct_fetch_plan("atcoder", "abc321_a").unwrap();
        assert_eq!(plan.db_id, "abc321_a");
        assert_eq!(
            plan.url,
            "https://atcoder.jp/contests/abc321/tasks/abc321_a"
        );

        let plan = derive_direct_fetch_plan("atcoder", "abc100_arc100_a").unwrap();
        assert_eq!(plan.db_id, "abc100_arc100_a");
        assert_eq!(
            plan.url,
            "https://atcoder.jp/contests/abc100/tasks/abc100_arc100_a"
        );

        let plan = derive_direct_fetch_plan("atcoder", "past201912_a").unwrap();
        assert_eq!(
            plan.url,
            "https://atcoder.jp/contests/past201912-open/tasks/past201912_a"
        );

        let plan = derive_direct_fetch_plan("atcoder", "arc058_abc042_a").unwrap();
        assert_eq!(
            plan.url,
            "https://atcoder.jp/contests/abc042/tasks/arc058_abc042_a"
        );

        let plan = derive_direct_fetch_plan("atcoder", "arc001_1").unwrap();
        assert_eq!(
            plan.url,
            "https://atcoder.jp/contests/arc001/tasks/arc001_1"
        );
    }

    #[test]
    fn derives_luogu_problem_url() {
        let plan = derive_direct_fetch_plan("luogu", "P1083").unwrap();
        assert_eq!(plan.db_id, "P1083");
        assert_eq!(plan.url, "https://www.luogu.com.cn/problem/P1083");
    }

    #[test]
    fn rejects_malformed_or_unsupported_ids() {
        assert!(derive_direct_fetch_plan("leetcode", "1").is_none());
        assert!(derive_direct_fetch_plan("codeforces", "1988").is_none());
        assert!(derive_direct_fetch_plan("codeforces", "ABC").is_none());
        assert!(derive_direct_fetch_plan("gym", "1988A").is_none());
        assert!(derive_direct_fetch_plan("atcoder", "abc321").is_none());
        assert!(derive_direct_fetch_plan("luogu", "1083").is_none());
    }
}
