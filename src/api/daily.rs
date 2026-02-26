use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ProblemDetail;
use crate::models::LeetCodeDomain;
use crate::AppState;

#[derive(Deserialize)]
pub struct DailyQuery {
    pub domain: Option<String>,
    pub source: Option<String>,
    pub date: Option<String>,
}

fn resolve_domain(
    domain: Option<&str>,
    source: Option<&str>,
) -> Result<LeetCodeDomain, ProblemDetail> {
    let from_source = match source {
        Some("leetcode.com") => Some(LeetCodeDomain::Com),
        Some("leetcode.cn") => Some(LeetCodeDomain::Cn),
        Some(s) => {
            return Err(ProblemDetail::bad_request(format!(
                "invalid source '{}', expected 'leetcode.com' or 'leetcode.cn'",
                s
            )))
        }
        None => None,
    };

    let from_domain = match domain {
        Some(d) => Some(
            d.parse::<LeetCodeDomain>()
                .map_err(|_| ProblemDetail::bad_request("domain must be 'com' or 'cn'"))?,
        ),
        None => None,
    };

    match (from_domain, from_source) {
        (Some(d), Some(s)) if d != s => Err(ProblemDetail::bad_request(
            "domain and source conflict",
        )),
        (Some(d), _) => Ok(d),
        (None, Some(s)) => Ok(s),
        (None, None) => Ok(LeetCodeDomain::Com),
    }
}

pub async fn get_daily(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DailyQuery>,
) -> impl IntoResponse {
    let domain = match resolve_domain(query.domain.as_deref(), query.source.as_deref()) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let today = domain.today();
    let date = query.date.as_deref().unwrap_or(&today);

    // Validate date format
    let date_re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    if !date_re.is_match(date) {
        return ProblemDetail::bad_request("invalid date format, expected YYYY-MM-DD")
            .into_response();
    }

    let parsed = match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return ProblemDetail::bad_request("invalid calendar date").into_response();
        }
    };

    let lower = chrono::NaiveDate::from_ymd_opt(2020, 4, 1).unwrap();
    let upper = domain.today_naive();

    if parsed < lower {
        return ProblemDetail::bad_request("date must be >= 2020-04-01").into_response();
    }
    if parsed > upper {
        return ProblemDetail::bad_request("date must be <= today").into_response();
    }

    let pool = state.ro_pool.clone();
    let domain_str = domain.to_string();
    let date_owned = date.to_string();
    let result = tokio::task::spawn_blocking(move || {
        crate::db::daily::get_daily(&pool, &domain_str, &date_owned)
    })
    .await
    .unwrap_or(None);

    if let Some(d) = result {
        return Json(d).into_response();
    }

    // Fallback: spawn crawler
    let key = format!("{}:{}", domain, date);
    let now = tokio::time::Instant::now();

    // Atomically check + claim slot under single lock to prevent TOCTOU race
    {
        let mut fallback = state.daily_fallback.lock().await;
        if let Some(entry) = fallback.get(&key) {
            if entry.status == crate::models::CrawlerStatus::Running {
                return (
                    axum::http::StatusCode::ACCEPTED,
                    Json(serde_json::json!({"status": "fetching", "retry_after": 30})),
                )
                    .into_response();
            }
            if let Some(until) = entry.cooldown_until {
                if now < until {
                    let remaining = (until - now).as_secs();
                    return (
                        axum::http::StatusCode::ACCEPTED,
                        Json(serde_json::json!({"status": "fetching", "retry_after": remaining})),
                    )
                        .into_response();
                }
            }
        }
        fallback.insert(
            key.clone(),
            crate::models::DailyFallbackEntry {
                status: crate::models::CrawlerStatus::Running,
                started_at: now,
                cooldown_until: None,
            },
        );
    }

    // Determine args
    let today_str = domain.today();
    let domain_arg = domain.to_string();
    let args: Vec<String> = if date == today_str {
        vec!["--daily".into(), "--domain".into(), domain_arg]
    } else {
        vec![
            "--date".into(),
            date.to_string(),
            "--domain".into(),
            domain_arg,
        ]
    };

    let mut cmd = tokio::process::Command::new("uv");
    cmd.args(["run", "python3", "leetcode.py"]);
    cmd.args(&args);
    cmd.current_dir("scripts/");
    cmd.kill_on_drop(true);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    if let Some(ref cp) = state.config_path {
        cmd.env("CONFIG_PATH", cp);
    }

    let child = match crate::utils::spawn_with_pgid(cmd) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to spawn daily fallback crawler: {}", e);
            let mut fallback = state.daily_fallback.lock().await;
            if let Some(entry) = fallback.get_mut(&key) {
                entry.status = crate::models::CrawlerStatus::Failed;
                entry.cooldown_until = Some(now + std::time::Duration::from_secs(30));
            }
            // Schedule cleanup for failed spawn
            let state_clone = state.clone();
            let key_clone = key;
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                let mut fallback = state_clone.daily_fallback.lock().await;
                if let Some(entry) = fallback.get(&key_clone) {
                    if entry.started_at == now {
                        fallback.remove(&key_clone);
                    }
                }
            });
            return ProblemDetail::internal("failed to spawn crawler").into_response();
        }
    };

    // Spawn background task
    let state_clone = state.clone();
    let key_clone = key.clone();
    let timeout_secs = state
        .config
        .crawler
        .per_source_timeout
        .get("leetcode")
        .copied()
        .unwrap_or(state.config.crawler.timeout_secs);
    let job_id = uuid::Uuid::new_v4().to_string();
    let pid = child.id().expect("child should have a pid");

    tokio::spawn(async move {
        let mut wait_task = tokio::spawn(async move { child.wait_with_output().await });
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), &mut wait_task)
                .await;
        // Flatten JoinHandle layer for consistent matching below
        let result: Result<std::io::Result<std::process::Output>, tokio::time::error::Elapsed> =
            match result {
                Ok(Ok(r)) => Ok(r),
                Ok(Err(e)) => {
                    tracing::error!("daily fallback join error: {}", e);
                    Ok(Err(std::io::Error::other(e.to_string())))
                }
                Err(e) => Err(e),
            };

        let status = match &result {
            Ok(Ok(output)) => {
                // Write log files
                if let Err(e) = tokio::fs::create_dir_all("scripts/logs").await {
                    tracing::warn!("failed to create scripts/logs: {}", e);
                }
                if !output.stdout.is_empty() {
                    if let Err(e) = tokio::fs::write(
                        format!("scripts/logs/{}.stdout.log", job_id),
                        &output.stdout,
                    )
                    .await
                    {
                        tracing::warn!("failed to write stdout log: {}", e);
                    }
                }
                if !output.stderr.is_empty() {
                    if let Err(e) = tokio::fs::write(
                        format!("scripts/logs/{}.stderr.log", job_id),
                        &output.stderr,
                    )
                    .await
                    {
                        tracing::warn!("failed to write stderr log: {}", e);
                    }
                }

                let stdout_str = String::from_utf8_lossy(&output.stdout);
                let preview: String = stdout_str.chars().take(500).collect();
                tracing::info!(
                    "daily fallback [{}] completed: status={}, stdout preview: {}",
                    job_id,
                    output.status,
                    preview
                );

                if output.status.success() {
                    crate::models::CrawlerStatus::Completed
                } else {
                    crate::models::CrawlerStatus::Failed
                }
            }
            Ok(Err(e)) => {
                tracing::error!("daily fallback crawler error: {}", e);
                crate::models::CrawlerStatus::Failed
            }
            Err(_) => {
                tracing::warn!("daily fallback timed out");
                crate::utils::kill_pgid(pid);
                let _ = wait_task.await;
                crate::models::CrawlerStatus::TimedOut
            }
        };

        let cooldown = if status != crate::models::CrawlerStatus::Completed {
            Some(tokio::time::Instant::now() + std::time::Duration::from_secs(30))
        } else {
            None
        };

        {
            let mut fallback = state_clone.daily_fallback.lock().await;
            if let Some(entry) = fallback.get_mut(&key_clone) {
                entry.status = status;
                entry.cooldown_until = cooldown;
            }
        }

        // Clean up entry after 60s, only if it's still ours
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let mut fallback = state_clone.daily_fallback.lock().await;
        if let Some(entry) = fallback.get(&key_clone) {
            if entry.started_at == now {
                fallback.remove(&key_clone);
            }
        }
    });

    (
        axum::http::StatusCode::ACCEPTED,
        Json(serde_json::json!({"status": "fetching", "retry_after": 30})),
    )
        .into_response()
}
