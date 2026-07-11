use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, TimeZone};

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::api::error::ProblemDetail;
use crate::models::{
    ActiveCrawlerPid, DailyChallengeRecord, JobType, LeetCodeDomain, ProblemRecord, ProblemSummary,
};

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct DailyChallengeResponse {
    pub(crate) date: String,
    pub(crate) source: String,
    pub(crate) problems: Vec<DailyProblemResponse>,
}

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct DailyProblemResponse {
    pub(crate) id: String,
    pub(crate) source: String,
    pub(crate) slug: String,
    pub(crate) title: Option<String>,
    pub(crate) difficulty: Option<String>,
    pub(crate) ac_rate: Option<f64>,
    pub(crate) rating: Option<f64>,
    pub(crate) contest: Option<String>,
    pub(crate) problem_index: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) link: Option<String>,
    pub(crate) category: Option<String>,
    pub(crate) paid_only: Option<i32>,
    pub(crate) content: Option<String>,
    pub(crate) similar_questions: Vec<ProblemSummary>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct DailyFetchingResponse {
    /// Fetch state for clients.
    pub(crate) status: String,
    /// Seconds to wait before retrying.
    pub(crate) retry_after: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) job_started: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) message: Option<String>,
}

impl DailyFetchingResponse {
    fn fetching(retry_after: u64) -> Self {
        Self {
            status: "fetching".to_string(),
            retry_after,
            job_started: None,
            message: None,
        }
    }

    fn ingestion_required() -> Self {
        Self {
            status: "ingestion_required".to_string(),
            retry_after: 30,
            job_started: Some(false),
            message: Some(
                "additional daily source requires scheduled or manual ingestion".to_string(),
            ),
        }
    }
}

fn daily_fetching_response(retry_after: u64) -> axum::response::Response {
    (
        axum::http::StatusCode::ACCEPTED,
        Json(DailyFetchingResponse::fetching(retry_after)),
    )
        .into_response()
}

fn daily_ingestion_required_response() -> axum::response::Response {
    (
        axum::http::StatusCode::ACCEPTED,
        Json(DailyFetchingResponse::ingestion_required()),
    )
        .into_response()
}

fn build_daily_response(
    pool: &crate::db::DbPool,
    record: DailyChallengeRecord,
) -> DailyChallengeResponse {
    let daily_source = record.source.clone();
    let problems = record
        .problems
        .into_iter()
        .map(|problem| project_daily_problem(pool, &daily_source, problem))
        .collect();

    DailyChallengeResponse {
        date: record.date,
        source: record.source,
        problems,
    }
}

fn project_daily_problem(
    pool: &crate::db::DbPool,
    daily_source: &str,
    problem: ProblemRecord,
) -> DailyProblemResponse {
    let mut similar_questions = crate::db::problems::resolve_similar_question_summaries(
        pool,
        &problem.source,
        &problem.similar_questions,
    );
    for summary in &mut similar_questions {
        summary.link = rewrite_leetcode_link(
            daily_source,
            &summary.source,
            &summary.slug,
            summary.link.clone(),
        );
    }

    DailyProblemResponse {
        id: problem.id,
        source: problem.source.clone(),
        slug: problem.slug.clone(),
        title: localized_field(daily_source, problem.title, problem.title_cn),
        difficulty: problem.difficulty,
        ac_rate: problem.ac_rate,
        rating: problem.rating,
        contest: problem.contest,
        problem_index: problem.problem_index,
        tags: problem.tags,
        link: rewrite_leetcode_link(daily_source, &problem.source, &problem.slug, problem.link),
        category: problem.category,
        paid_only: problem.paid_only,
        content: localized_field(daily_source, problem.content, problem.content_cn),
        similar_questions,
    }
}

fn localized_field(
    daily_source: &str,
    default_value: Option<String>,
    cn_value: Option<String>,
) -> Option<String> {
    if daily_source == "leetcode.cn" {
        cn_value
            .filter(|value| !value.trim().is_empty())
            .or(default_value)
    } else {
        default_value
    }
}

fn rewrite_leetcode_link(
    daily_source: &str,
    problem_source: &str,
    slug: &str,
    stored_link: Option<String>,
) -> Option<String> {
    if problem_source == "leetcode" {
        match daily_source {
            "leetcode.cn" => return Some(format!("https://leetcode.cn/problems/{slug}/")),
            "leetcode.com" => return Some(format!("https://leetcode.com/problems/{slug}/")),
            _ => {}
        }
    }
    stored_link
}

#[cfg(test)]
const DAILY_FALLBACK_CLEANUP_DELAY: Duration = Duration::from_millis(20);
#[cfg(not(test))]
const DAILY_FALLBACK_CLEANUP_DELAY: Duration = Duration::from_secs(60);

fn crawler_status_is_terminal(status: &crate::models::CrawlerStatus) -> bool {
    crate::utils::crawler_status_is_terminal(status)
}
use crate::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct DailyQuery {
    pub domain: Option<String>,
    pub source: Option<String>,
    pub date: Option<String>,
    #[param(rename = "async")]
    pub r#async: Option<bool>,
}

const ADDITIONAL_DAILY_SOURCE_REFRESH_HOURS: [u32; 3] = [8, 10, 12];
const ADDITIONAL_DAILY_SOURCES: [DailySourceSelection; 2] = [
    DailySourceSelection::sheep(),
    DailySourceSelection::zero_x3f(),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdditionalDailySource {
    Sheep,
    ZeroX3f,
}

impl AdditionalDailySource {
    fn source(self) -> &'static str {
        match self {
            Self::Sheep => "sheep",
            Self::ZeroX3f => "0x3f",
        }
    }

    fn is_available_on(self, date: chrono::NaiveDate) -> bool {
        let (first_date, active_days) = match self {
            Self::Sheep => (chrono::NaiveDate::from_ymd_opt(2024, 2, 26).unwrap(), 6),
            Self::ZeroX3f => (chrono::NaiveDate::from_ymd_opt(2022, 4, 8).unwrap(), 5),
        };

        date >= first_date && date.weekday().num_days_from_monday() < active_days
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DailySourceKind {
    LeetCode(LeetCodeDomain),
    Additional(AdditionalDailySource),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DailySourceSelection {
    source: &'static str,
    kind: DailySourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DailyFallbackPlan {
    job_source: &'static str,
    script: &'static str,
    timeout_key: &'static str,
    args: Vec<String>,
}

impl DailySourceSelection {
    const fn leetcode(domain: LeetCodeDomain) -> Self {
        Self {
            source: canonical_daily_source(domain),
            kind: DailySourceKind::LeetCode(domain),
        }
    }

    const fn sheep() -> Self {
        Self {
            source: "sheep",
            kind: DailySourceKind::Additional(AdditionalDailySource::Sheep),
        }
    }

    const fn zero_x3f() -> Self {
        Self {
            source: "0x3f",
            kind: DailySourceKind::Additional(AdditionalDailySource::ZeroX3f),
        }
    }

    fn today(&self) -> String {
        match self.kind {
            DailySourceKind::LeetCode(domain) => domain.today(),
            DailySourceKind::Additional(_) => utc8_today().to_string(),
        }
    }

    fn today_naive(&self) -> chrono::NaiveDate {
        match self.kind {
            DailySourceKind::LeetCode(domain) => domain.today_naive(),
            DailySourceKind::Additional(_) => utc8_today(),
        }
    }

    fn is_available_on(&self, date: chrono::NaiveDate) -> bool {
        match self.kind {
            DailySourceKind::LeetCode(_) => true,
            DailySourceKind::Additional(source) => source.is_available_on(date),
        }
    }

    fn fallback_plan(
        &self,
        date: &str,
        config: &crate::config::Config,
    ) -> Option<DailyFallbackPlan> {
        match self.kind {
            DailySourceKind::LeetCode(domain) => {
                let domain_arg = domain.to_string();
                let args = if date == domain.today() {
                    vec!["--daily".into(), "--domain".into(), domain_arg]
                } else {
                    vec![
                        "--date".into(),
                        date.to_string(),
                        "--domain".into(),
                        domain_arg,
                    ]
                };
                Some(DailyFallbackPlan {
                    job_source: "leetcode",
                    script: "leetcode.py",
                    timeout_key: "leetcode",
                    args,
                })
            }
            DailySourceKind::Additional(source) => {
                if source == AdditionalDailySource::ZeroX3f
                    && !tencent_docs_token_configured(config)
                {
                    return None;
                }
                Some(DailyFallbackPlan {
                    job_source: "daily_source",
                    script: "daily_source.py",
                    timeout_key: "daily_source",
                    args: vec![
                        "--daily-source".into(),
                        source.source().into(),
                        "--date".into(),
                        date.to_string(),
                    ],
                })
            }
        }
    }
}

fn resolve_daily_source(
    domain: Option<&str>,
    source: Option<&str>,
) -> Result<DailySourceSelection, ProblemDetail> {
    let from_source = match source {
        Some("leetcode.com") => Some(DailySourceSelection::leetcode(LeetCodeDomain::Com)),
        Some("leetcode.cn") => Some(DailySourceSelection::leetcode(LeetCodeDomain::Cn)),
        Some("sheep") => Some(DailySourceSelection::sheep()),
        Some("0x3f") => Some(DailySourceSelection::zero_x3f()),
        Some(s) => {
            return Err(ProblemDetail::bad_request(format!(
                "invalid source '{}', expected 'leetcode.com', 'leetcode.cn', 'sheep', or '0x3f'",
                s
            )))
        }
        None => None,
    };

    let from_domain = match domain {
        Some(d) => {
            let domain = d
                .parse::<LeetCodeDomain>()
                .map_err(|_| ProblemDetail::bad_request("domain must be 'com' or 'cn'"))?;
            Some(DailySourceSelection::leetcode(domain))
        }
        None => None,
    };

    match (from_domain, from_source) {
        (Some(d), Some(s)) if d != s => {
            Err(ProblemDetail::bad_request("domain and source conflict"))
        }
        (Some(d), _) => Ok(d),
        (None, Some(s)) => Ok(s),
        (None, None) => Ok(DailySourceSelection::leetcode(LeetCodeDomain::Com)),
    }
}

const fn canonical_daily_source(domain: LeetCodeDomain) -> &'static str {
    match domain {
        LeetCodeDomain::Com => "leetcode.com",
        LeetCodeDomain::Cn => "leetcode.cn",
    }
}

fn utc8_offset() -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(8 * 3600).unwrap()
}

fn utc8_today() -> chrono::NaiveDate {
    chrono::Utc::now()
        .with_timezone(&utc8_offset())
        .date_naive()
}

fn next_additional_daily_source_refresh_after(
    now: chrono::DateTime<chrono::Utc>,
) -> chrono::DateTime<chrono::Utc> {
    let offset = utc8_offset();
    let now_local = now.with_timezone(&offset);
    let today = now_local.date_naive();

    for hour in ADDITIONAL_DAILY_SOURCE_REFRESH_HOURS {
        let candidate = offset
            .with_ymd_and_hms(today.year(), today.month(), today.day(), hour, 0, 0)
            .single()
            .unwrap();
        if candidate > now_local {
            return candidate.with_timezone(&chrono::Utc);
        }
    }

    let tomorrow = today.succ_opt().unwrap();
    offset
        .with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), 8, 0, 0)
        .single()
        .unwrap()
        .with_timezone(&chrono::Utc)
}

fn tencent_docs_token_configured(config: &crate::config::Config) -> bool {
    config.daily_sources.tencent_docs.resolve_token().is_some()
}

async fn wait_and_fetch(
    notify: Arc<Notify>,
    completed: Arc<std::sync::atomic::AtomicBool>,
    state: &Arc<AppState>,
    source: String,
    date: String,
) -> Option<DailyChallengeResponse> {
    // Register interest before checking completed flag to avoid race where
    // notify_waiters() fires between the flag check and notified() setup.
    let notification = notify.notified();
    tokio::pin!(notification);
    notification.as_mut().enable();

    // If crawler already finished, skip waiting entirely.
    if !completed.load(Ordering::Acquire)
        && tokio::time::timeout(Duration::from_secs(10), &mut notification)
            .await
            .is_err()
    {
        return None;
    }

    let pool = state.ro_pool.clone();
    tokio::task::spawn_blocking(move || {
        let record = crate::db::daily::get_daily_record(&pool, &source, &date)?;
        Some(build_daily_response(&pool, record))
    })
    .await
    .unwrap_or(None)
}

fn resolve_daily_fallback_terminal_status(
    status: crate::models::CrawlerStatus,
    capture_complete: bool,
) -> crate::models::CrawlerStatus {
    if capture_complete {
        status
    } else if status == crate::models::CrawlerStatus::TimedOut {
        crate::models::CrawlerStatus::TimedOut
    } else {
        crate::models::CrawlerStatus::Failed
    }
}

fn remove_daily_fallback_crawler_job_if_matches(
    crawler_jobs: &mut std::collections::HashMap<String, crate::models::CrawlerJob>,
    runtime_key: &str,
    job_id: &str,
) {
    if crawler_jobs.get(runtime_key).map(|job| job.job_id.as_str()) == Some(job_id) {
        crawler_jobs.remove(runtime_key);
    }
}

fn schedule_daily_fallback_cleanup(
    state: Arc<AppState>,
    runtime_key: String,
    job_id: String,
    started_at: tokio::time::Instant,
) {
    tokio::spawn(async move {
        tokio::time::sleep(DAILY_FALLBACK_CLEANUP_DELAY).await;
        let removed = {
            let mut fallback = state.daily_fallback.lock().await;
            match fallback.get(&runtime_key) {
                Some(entry) if entry.started_at == started_at => {
                    fallback.remove(&runtime_key);
                    true
                }
                _ => false,
            }
        };
        if removed {
            let mut crawler_jobs = state.crawler_jobs.lock().await;
            remove_daily_fallback_crawler_job_if_matches(&mut crawler_jobs, &runtime_key, &job_id);
        }
    });
}

async fn handle_daily_fallback_terminal_failure(
    state: &Arc<AppState>,
    runtime_key: &str,
    job_id: &str,
    started_at: tokio::time::Instant,
    artifact_paths: &crate::models::JobArtifactPaths,
) {
    let mut fallback = state.daily_fallback.lock().await;
    if let Some(entry) = fallback.get_mut(runtime_key) {
        entry.status = crate::models::CrawlerStatus::Failed;
        entry.cooldown_until = Some(started_at + Duration::from_secs(30));
        entry.completed.store(true, Ordering::Release);
        entry.notify.notify_waiters();
    }
    drop(fallback);

    let finished_job = {
        let mut crawler_jobs = state.crawler_jobs.lock().await;
        if let Some(job) = crawler_jobs.get_mut(runtime_key) {
            if !crawler_status_is_terminal(&job.status) {
                job.status = crate::models::CrawlerStatus::Failed;
            }
            if job.finished_at.is_none() {
                job.finished_at = Some(chrono::Utc::now().to_rfc3339());
            }
            Some(job.clone())
        } else {
            None
        }
    };
    if let Some(job) = finished_job {
        crate::utils::persist_crawler_terminal_progress(artifact_paths, &job).await;
        let mut history = state.crawler_history.lock().await;
        crate::utils::push_or_replace_crawler_history(&mut history, job);
    }
    schedule_daily_fallback_cleanup(
        state.clone(),
        runtime_key.to_string(),
        job_id.to_string(),
        started_at,
    );
}

type DailyFallbackWaiter = (Arc<Notify>, Arc<std::sync::atomic::AtomicBool>);

enum DailyFallbackLaunch {
    Ready { waiter: Option<DailyFallbackWaiter> },
    Cooldown(u64),
    IngestionRequired,
    SetupFailed,
    LaunchFailed { message: &'static str },
}

async fn ensure_daily_fallback_running(
    state: &Arc<AppState>,
    selection: DailySourceSelection,
    date: &str,
    should_wait: bool,
) -> DailyFallbackLaunch {
    let Some(plan) = selection.fallback_plan(date, &state.config) else {
        return DailyFallbackLaunch::IngestionRequired;
    };

    let key = crate::models::daily_fallback_crawler_runtime_key(selection.source, date);
    let now = tokio::time::Instant::now();
    let job_id = uuid::Uuid::new_v4().to_string();

    // Atomically check + claim slot under single lock to prevent TOCTOU race
    let waiter = {
        let mut fallback = state.daily_fallback.lock().await;
        if let Some(entry) = fallback.get(&key) {
            if entry.status == crate::models::CrawlerStatus::Running {
                let waiter = should_wait.then(|| (entry.notify.clone(), entry.completed.clone()));
                return DailyFallbackLaunch::Ready { waiter };
            }
            if let Some(until) = entry.cooldown_until {
                if now < until {
                    return DailyFallbackLaunch::Cooldown((until - now).as_secs());
                }
            }
        }
        let notify = Arc::new(Notify::new());
        let completed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        fallback.insert(
            key.clone(),
            crate::models::DailyFallbackEntry {
                job_id: job_id.clone(),
                status: crate::models::CrawlerStatus::Running,
                started_at: now,
                cooldown_until: None,
                notify: notify.clone(),
                completed: completed.clone(),
                stdout: None,
                stderr: None,
            },
        );
        should_wait.then_some((notify, completed))
    };

    let job = crate::models::CrawlerJob {
        job_id: job_id.clone(),
        source: plan.job_source.to_string(),
        args: plan.args.clone(),
        trigger: crate::models::CrawlerTrigger::DailyFallback,
        started_at: chrono::Utc::now().to_rfc3339(),
        finished_at: None,
        status: crate::models::CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    };
    state
        .crawler_jobs
        .lock()
        .await
        .insert(key.clone(), job.clone());

    let artifact_paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id)
        .expect("uuid job id should produce safe artifact paths");
    if let Err(err) = crate::utils::persist_job_metadata(
        &artifact_paths,
        crate::models::JobArtifactMetadata::from(&job),
    )
    .await
    {
        tracing::warn!("failed to persist daily fallback metadata: {}", err);
        handle_daily_fallback_terminal_failure(state, &key, &job_id, now, &artifact_paths).await;
        return DailyFallbackLaunch::SetupFailed;
    }

    let mut cmd = tokio::process::Command::new("uv");
    cmd.args(["run", "python3", plan.script]);
    cmd.args(&plan.args);
    cmd.current_dir("scripts/");
    cmd.kill_on_drop(true);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    if let Some(ref cp) = state.config_path {
        cmd.env("CONFIG_PATH", cp);
    }

    crate::utils::inject_job_environment(&mut cmd, &job_id, JobType::Crawler, &artifact_paths);

    let mut child = match crate::utils::spawn_with_pgid(cmd) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to spawn daily fallback crawler: {}", e);
            handle_daily_fallback_terminal_failure(state, &key, &job_id, now, &artifact_paths)
                .await;
            return DailyFallbackLaunch::LaunchFailed {
                message: "failed to spawn crawler",
            };
        }
    };
    let capture = match crate::utils::start_live_output_capture(&mut child, &artifact_paths).await {
        Ok(capture) => capture,
        Err(err) => {
            tracing::error!("failed to start daily fallback output capture: {}", err);
            if let Some(pid) = child.id() {
                crate::utils::kill_pgid(pid);
            }
            let _ = child.wait().await;
            handle_daily_fallback_terminal_failure(state, &key, &job_id, now, &artifact_paths)
                .await;
            return DailyFallbackLaunch::LaunchFailed {
                message: "failed to capture crawler output",
            };
        }
    };
    let state_clone = state.clone();
    let key_clone = key.clone();
    let timeout_secs = state
        .config
        .crawler
        .per_source_timeout
        .get(plan.timeout_key)
        .copied()
        .unwrap_or(state.config.crawler.timeout_secs);
    let pid = child.id().expect("child should have a pid");
    state.active_crawler_pids.lock().await.insert(
        key.clone(),
        ActiveCrawlerPid {
            job_id: job_id.clone(),
            pid,
        },
    );
    if let Some(job) = {
        let crawler_jobs = state.crawler_jobs.lock().await;
        crawler_jobs.get(&key).cloned().filter(|job| {
            job.job_id == job_id && job.status == crate::models::CrawlerStatus::Running
        })
    } {
        crate::utils::persist_crawler_running_progress(&artifact_paths, &job).await;
    }

    tokio::spawn(async move {
        let mut wait_task = tokio::spawn(async move { child.wait().await });
        let result = tokio::time::timeout(Duration::from_secs(timeout_secs), &mut wait_task).await;

        state_clone
            .active_crawler_pids
            .lock()
            .await
            .remove(&key_clone);

        let (status, capture_result) = match result {
            Ok(Ok(Ok(exit_status))) => {
                let status = if exit_status.success() {
                    crate::models::CrawlerStatus::Completed
                } else {
                    crate::models::CrawlerStatus::Failed
                };
                let capture_result = capture.finish().await.inspect(|output| {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    let preview: String = stdout_str.chars().take(500).collect();
                    tracing::info!(
                        "daily fallback [{}] completed: status={}, stdout preview: {}",
                        job_id,
                        exit_status,
                        preview
                    );
                });
                (status, capture_result)
            }
            Ok(Ok(Err(e))) => {
                tracing::error!("daily fallback crawler error: {}", e);
                (crate::models::CrawlerStatus::Failed, capture.finish().await)
            }
            Ok(Err(e)) => {
                tracing::error!("daily fallback join error: {}", e);
                (crate::models::CrawlerStatus::Failed, capture.finish().await)
            }
            Err(_) => {
                tracing::warn!("daily fallback timed out");
                crate::utils::kill_pgid(pid);
                let _ = wait_task.await;
                (
                    crate::models::CrawlerStatus::TimedOut,
                    capture.finish().await,
                )
            }
        };

        let capture_complete = capture_result.is_ok();
        let terminal_status = resolve_daily_fallback_terminal_status(status, capture_complete);
        let cooldown = if terminal_status != crate::models::CrawlerStatus::Completed {
            Some(tokio::time::Instant::now() + Duration::from_secs(30))
        } else {
            None
        };

        let (stdout, stderr) = match capture_result {
            Ok(output) => (Some(output.stdout), Some(output.stderr)),
            Err(err) => {
                tracing::error!("daily fallback capture error: {}", err);
                (None, None)
            }
        };
        {
            let mut fallback = state_clone.daily_fallback.lock().await;
            if let Some(entry) = fallback.get_mut(&key_clone) {
                match (stdout.as_ref(), stderr.as_ref()) {
                    (Some(stdout), Some(stderr)) => {
                        apply_daily_fallback_terminal_update(
                            entry,
                            terminal_status.clone(),
                            Ok(crate::utils::CapturedOutput {
                                stdout: stdout.clone(),
                                stderr: stderr.clone(),
                            }),
                        );
                    }
                    _ => {
                        apply_daily_fallback_terminal_update(
                            entry,
                            terminal_status.clone(),
                            Err(std::io::Error::other("daily fallback capture missing")),
                        );
                    }
                }
                entry.cooldown_until = cooldown;
                entry.completed.store(true, Ordering::Release);
                entry.notify.notify_waiters();
            }
        }
        let finished_job = {
            let mut crawler_jobs = state_clone.crawler_jobs.lock().await;
            if let Some(job) = crawler_jobs.get_mut(&key_clone) {
                if !crawler_status_is_terminal(&job.status) {
                    job.status = terminal_status;
                }
                if job.finished_at.is_none() {
                    job.finished_at = Some(chrono::Utc::now().to_rfc3339());
                }
                if let (Some(stdout), Some(stderr)) = (stdout.as_ref(), stderr.as_ref()) {
                    job.set_output(stdout.clone(), stderr.clone());
                }
                Some(job.clone())
            } else {
                None
            }
        };
        if let Some(job) = finished_job {
            crate::utils::persist_crawler_terminal_progress(&artifact_paths, &job).await;
            let mut history = state_clone.crawler_history.lock().await;
            crate::utils::push_or_replace_crawler_history(&mut history, job);
        }

        schedule_daily_fallback_cleanup(state_clone, key_clone, job_id, now);
    });

    DailyFallbackLaunch::Ready { waiter }
}

/// Spawn a background task that refreshes additional daily sources at the
/// fixed UTC+8 refresh hours, reusing the daily fallback runtime keys so
/// scheduled jobs and API fallback never run duplicates.
pub fn spawn_additional_daily_source_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let now = chrono::Utc::now();
            let next = next_additional_daily_source_refresh_after(now);
            let wait = (next - now)
                .to_std()
                .unwrap_or_else(|_| std::time::Duration::from_secs(0));
            tokio::time::sleep(wait).await;

            let today = utc8_today();
            let date = today.to_string();
            for selection in ADDITIONAL_DAILY_SOURCES {
                if !selection.is_available_on(today) {
                    tracing::info!(
                        "scheduled daily source refresh skipped (unavailable date): {} {}",
                        selection.source,
                        date
                    );
                    continue;
                }
                match ensure_daily_fallback_running(&state, selection, &date, false).await {
                    DailyFallbackLaunch::Ready { .. } => {
                        tracing::info!(
                            "scheduled daily source refresh running: {} {}",
                            selection.source,
                            date
                        );
                    }
                    DailyFallbackLaunch::IngestionRequired => {
                        tracing::info!(
                            "scheduled daily source refresh skipped (token not configured): {}",
                            selection.source
                        );
                    }
                    DailyFallbackLaunch::Cooldown(secs) => {
                        tracing::info!(
                            "scheduled daily source refresh in cooldown ({}s): {} {}",
                            secs,
                            selection.source,
                            date
                        );
                    }
                    DailyFallbackLaunch::SetupFailed | DailyFallbackLaunch::LaunchFailed { .. } => {
                        tracing::warn!(
                            "scheduled daily source refresh failed to launch: {} {}",
                            selection.source,
                            date
                        );
                    }
                }
            }
        }
    });
}

#[utoipa::path(
    get,
    path = "/api/v1/daily",
    params(
        DailyQuery,
    ),
    responses(
        (status = 200, description = "Daily challenge", body = DailyChallengeResponse),
        (status = 202, description = "Crawler triggered, no data yet. Retry after the specified seconds.", body = DailyFetchingResponse),
        (status = 400, description = "Invalid parameters", body = ProblemDetail, content_type = "application/problem+json"),
        (status = 500, description = "Internal error", body = ProblemDetail, content_type = "application/problem+json"),
    ),
    security(("bearer_auth" = [])),
    tag = "Daily"
)]
pub async fn get_daily(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DailyQuery>,
) -> impl IntoResponse {
    let selection = match resolve_daily_source(query.domain.as_deref(), query.source.as_deref()) {
        Ok(selection) => selection,
        Err(e) => return e.into_response(),
    };
    let today = selection.today();
    let date = query.date.as_deref().unwrap_or(&today);
    let should_wait = !query.r#async.unwrap_or(false);

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
    let upper = selection.today_naive();

    if parsed < lower {
        return ProblemDetail::bad_request("date must be >= 2020-04-01").into_response();
    }
    if parsed > upper {
        return ProblemDetail::bad_request("date must be <= today").into_response();
    }
    if !selection.is_available_on(parsed) {
        return ProblemDetail::not_found("daily challenge is unavailable for this source and date")
            .into_response();
    }

    let pool = state.ro_pool.clone();
    let source = selection.source.to_string();
    let date_owned = date.to_string();
    let result = tokio::task::spawn_blocking(move || {
        let record = crate::db::daily::get_daily_record(&pool, &source, &date_owned)?;
        Some(build_daily_response(&pool, record))
    })
    .await
    .unwrap_or(None);

    if let Some(daily) = result {
        return Json(daily).into_response();
    }

    match ensure_daily_fallback_running(&state, selection, date, should_wait).await {
        DailyFallbackLaunch::Ready { waiter } => {
            if let Some((notify, completed)) = waiter {
                if let Some(d) = wait_and_fetch(
                    notify,
                    completed,
                    &state,
                    selection.source.to_string(),
                    date.to_string(),
                )
                .await
                {
                    return Json(d).into_response();
                }
            }
            daily_fetching_response(30)
        }
        DailyFallbackLaunch::Cooldown(remaining) => daily_fetching_response(remaining),
        DailyFallbackLaunch::IngestionRequired => daily_ingestion_required_response(),
        DailyFallbackLaunch::SetupFailed => {
            ProblemDetail::internal("failed to persist daily fallback metadata").into_response()
        }
        DailyFallbackLaunch::LaunchFailed { message } => {
            if should_wait {
                daily_fetching_response(30)
            } else {
                ProblemDetail::internal(message).into_response()
            }
        }
    }
}

fn apply_daily_fallback_terminal_update(
    entry: &mut crate::models::DailyFallbackEntry,
    status: crate::models::CrawlerStatus,
    capture_result: std::io::Result<crate::utils::CapturedOutput>,
) {
    let already_terminal = crawler_status_is_terminal(&entry.status);

    match capture_result {
        Ok(output) => {
            if !already_terminal {
                entry.status = status;
            }
            entry.set_output(output.stdout, output.stderr);
        }
        Err(err) => {
            tracing::error!("daily fallback capture error: {}", err);
            if already_terminal {
                return;
            }
            entry.status = if status == crate::models::CrawlerStatus::TimedOut {
                crate::models::CrawlerStatus::TimedOut
            } else {
                crate::models::CrawlerStatus::Failed
            };
        }
    }
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use std::collections::{HashMap, VecDeque};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use axum::extract::{Query, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use rand::Rng;
    use tokio::sync::{Notify, RwLock, Semaphore};

    use super::{
        apply_daily_fallback_terminal_update, handle_daily_fallback_terminal_failure, DailyQuery,
    };
    use crate::config::Config;
    use crate::models::{
        daily_fallback_crawler_runtime_key, CrawlerPhase, CrawlerProgress, CrawlerStatus,
        CrawlerTrigger, DailyFallbackEntry, JobType,
    };
    use crate::utils::CapturedOutput;
    use crate::AppState;

    fn fallback_entry() -> DailyFallbackEntry {
        DailyFallbackEntry {
            job_id: "daily-running".to_string(),
            status: CrawlerStatus::Running,
            started_at: tokio::time::Instant::now(),
            cooldown_until: None,
            notify: Arc::new(Notify::new()),
            completed: Arc::new(AtomicBool::new(false)),
            stdout: None,
            stderr: None,
        }
    }

    fn test_state() -> Arc<AppState> {
        test_state_with_config(Config::default())
    }

    fn test_state_with_config(config: Config) -> Arc<AppState> {
        Arc::new(AppState {
            ro_pool: crate::db::create_ro_pool(":memory:", 1, config.database.busy_timeout_ms),
            rw_pool: crate::db::create_rw_pool(":memory:", 1, config.database.busy_timeout_ms),
            config,
            crawler_jobs: tokio::sync::Mutex::new(HashMap::new()),
            manual_crawler_guard: tokio::sync::Mutex::new(None),
            crawler_history: tokio::sync::Mutex::new(VecDeque::new()),
            embedding_lock: tokio::sync::Mutex::new(None),
            embedding_launch_guard: tokio::sync::Mutex::new(None),
            embedding_history: tokio::sync::Mutex::new(VecDeque::new()),
            active_crawler_pids: tokio::sync::Mutex::new(HashMap::new()),
            active_embedding_pid: tokio::sync::Mutex::new(None),
            daily_fallback: tokio::sync::Mutex::new(HashMap::new()),
            retained_refresh: tokio::sync::Mutex::new(crate::utils::RetainedRefreshState::default()),
            embed_semaphore: Semaphore::new(1),
            token_auth_enabled: Arc::new(AtomicBool::new(true)),
            admin_sessions: Arc::new(RwLock::new(HashMap::new())),
            config_path: None,
        })
    }

    #[cfg(unix)]
    struct FakeUvGuard {
        root: std::path::PathBuf,
        original_path: Option<std::ffi::OsString>,
        marker_root: std::path::PathBuf,
        _path_lock: std::sync::MutexGuard<'static, ()>,
    }

    #[cfg(unix)]
    impl FakeUvGuard {
        async fn install() -> Self {
            Self::install_with_trailer("exit 0\n").await
        }

        async fn install_with_trailer(script_trailer: &str) -> Self {
            let path_lock = crate::utils::TEST_PATH_MUTEX
                .lock()
                .unwrap_or_else(|err| err.into_inner());
            let root = std::env::temp_dir().join(format!(
                "oj-api-rs-daily-fake-uv-{}-{}",
                std::process::id(),
                rand::thread_rng().r#gen::<u64>()
            ));
            let marker_root = std::env::temp_dir().join(format!(
                "oj-api-rs-daily-markers-{}-{}",
                std::process::id(),
                rand::thread_rng().r#gen::<u64>()
            ));
            tokio::fs::create_dir_all(&root).await.unwrap();
            tokio::fs::create_dir_all(&marker_root).await.unwrap();
            let uv_path = root.join("uv");
            let script_body = format!(
                "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\ndest=\"${{OJ_JOB_DIR:-{marker_root}}}\"\nmkdir -p \"$dest\"\nprintf '%s\\n' \"${{OJ_JOB_ID:-}}\" > \"$dest/env-job-id.txt\"\nprintf '%s\\n' \"${{OJ_JOB_TYPE:-}}\" > \"$dest/env-job-type.txt\"\nprintf '%s\\n' \"${{OJ_JOB_DIR:-}}\" > \"$dest/env-job-dir.txt\"\nprintf '%s\\n' \"${{OJ_PROGRESS_PATH:-}}\" > \"$dest/env-progress-path.txt\"\nprintf '%s\\n' \"${{OJ_PYTHON_LOG_PATH:-}}\" > \"$dest/env-python-log-path.txt\"\n{script_trailer}",
                marker_root = marker_root.display(),
                script_trailer = script_trailer,
            );
            tokio::fs::write(&uv_path, script_body).await.unwrap();
            let mut perms = tokio::fs::metadata(&uv_path).await.unwrap().permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&uv_path, perms).await.unwrap();

            let original_path = std::env::var_os("PATH");
            let mut new_path = std::ffi::OsString::from(&root);
            if let Some(existing) = original_path.as_ref() {
                new_path.push(":");
                new_path.push(existing);
            }
            std::env::set_var("PATH", &new_path);

            Self {
                root,
                original_path,
                marker_root,
                _path_lock: path_lock,
            }
        }
    }

    #[cfg(unix)]
    impl Drop for FakeUvGuard {
        fn drop(&mut self) {
            match self.original_path.as_ref() {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
            let _ = std::fs::remove_dir_all(&self.root);
            let _ = std::fs::remove_dir_all(&self.marker_root);
        }
    }

    #[cfg(unix)]
    async fn wait_for_daily_job_terminal(state: &Arc<AppState>, runtime_key: &str) {
        for _ in 0..100 {
            let done = {
                let crawler_jobs = state.crawler_jobs.lock().await;
                crawler_jobs
                    .get(runtime_key)
                    .map(|job| job.finished_at.is_some())
                    .unwrap_or(false)
            };
            if done {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("daily fallback crawler did not reach terminal state in time");
    }

    #[test]
    fn apply_daily_fallback_terminal_update_keeps_timed_out_tail() {
        let mut entry = fallback_entry();

        apply_daily_fallback_terminal_update(
            &mut entry,
            CrawlerStatus::TimedOut,
            Ok(CapturedOutput {
                stdout: b"daily stdout".to_vec(),
                stderr: b"daily stderr".to_vec(),
            }),
        );

        assert_eq!(entry.status, CrawlerStatus::TimedOut);
        assert_eq!(entry.stdout.as_deref(), Some("daily stdout"));
        assert_eq!(entry.stderr.as_deref(), Some("daily stderr"));
    }

    #[test]
    fn apply_daily_fallback_terminal_update_does_not_regress_existing_terminal_status() {
        let mut entry = fallback_entry();
        entry.status = CrawlerStatus::Cancelled;

        apply_daily_fallback_terminal_update(
            &mut entry,
            CrawlerStatus::Completed,
            Ok(CapturedOutput {
                stdout: b"daily stdout".to_vec(),
                stderr: b"daily stderr".to_vec(),
            }),
        );

        assert_eq!(entry.status, CrawlerStatus::Cancelled);
        assert_eq!(entry.stdout.as_deref(), Some("daily stdout"));
        assert_eq!(entry.stderr.as_deref(), Some("daily stderr"));
    }

    #[test]
    fn resolve_daily_fallback_terminal_status_falls_back_to_failed_on_capture_error() {
        assert_eq!(
            super::resolve_daily_fallback_terminal_status(CrawlerStatus::Completed, false),
            CrawlerStatus::Failed
        );
        assert_eq!(
            super::resolve_daily_fallback_terminal_status(CrawlerStatus::Failed, false),
            CrawlerStatus::Failed
        );
        assert_eq!(
            super::resolve_daily_fallback_terminal_status(CrawlerStatus::TimedOut, false),
            CrawlerStatus::TimedOut
        );
        assert_eq!(
            super::resolve_daily_fallback_terminal_status(CrawlerStatus::Completed, true),
            CrawlerStatus::Completed
        );
    }

    #[test]
    fn remove_daily_fallback_crawler_job_if_matches_keeps_newer_job_with_same_runtime_key() {
        let runtime_key = daily_fallback_crawler_runtime_key("leetcode.com", "2026-03-14");
        let newer_job = crate::models::CrawlerJob {
            job_id: "new-job".to_string(),
            source: "leetcode".to_string(),
            args: vec!["--daily".to_string()],
            trigger: CrawlerTrigger::DailyFallback,
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };
        let mut crawler_jobs = HashMap::from([(runtime_key.clone(), newer_job.clone())]);

        super::remove_daily_fallback_crawler_job_if_matches(
            &mut crawler_jobs,
            &runtime_key,
            "old-job",
        );

        assert_eq!(
            crawler_jobs
                .get(&runtime_key)
                .map(|job| job.job_id.as_str()),
            Some("new-job")
        );

        super::remove_daily_fallback_crawler_job_if_matches(
            &mut crawler_jobs,
            &runtime_key,
            "new-job",
        );

        assert!(!crawler_jobs.contains_key(&runtime_key));
    }

    #[tokio::test]
    async fn handle_daily_fallback_terminal_failure_marks_job_and_history_failed() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let state = test_state();
        let runtime_key = daily_fallback_crawler_runtime_key("leetcode.com", "2026-03-14");
        let started_at = tokio::time::Instant::now();
        let job_id = uuid::Uuid::new_v4().to_string();
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

        state.daily_fallback.lock().await.insert(
            runtime_key.clone(),
            DailyFallbackEntry {
                job_id: job_id.clone(),
                status: CrawlerStatus::Running,
                started_at,
                cooldown_until: None,
                notify: Arc::new(Notify::new()),
                completed: Arc::new(AtomicBool::new(false)),
                stdout: None,
                stderr: None,
            },
        );
        state.crawler_jobs.lock().await.insert(
            runtime_key.clone(),
            crate::models::CrawlerJob {
                job_id: job_id.clone(),
                source: "leetcode".to_string(),
                args: vec!["--daily".to_string()],
                trigger: CrawlerTrigger::DailyFallback,
                started_at: chrono::Utc::now().to_rfc3339(),
                finished_at: None,
                status: CrawlerStatus::Running,
                stdout: None,
                stderr: None,
            },
        );

        handle_daily_fallback_terminal_failure(&state, &runtime_key, &job_id, started_at, &paths)
            .await;

        {
            let fallback = state.daily_fallback.lock().await;
            let entry = fallback.get(&runtime_key).unwrap();
            assert_eq!(entry.status, CrawlerStatus::Failed);
            assert!(entry.cooldown_until.is_some());
            assert!(entry.completed.load(Ordering::Acquire));
        }
        {
            let crawler_jobs = state.crawler_jobs.lock().await;
            let job = crawler_jobs.get(&runtime_key).unwrap();
            assert_eq!(job.status, CrawlerStatus::Failed);
            assert!(job.finished_at.is_some());
        }
        {
            let history = state.crawler_history.lock().await;
            assert!(history
                .iter()
                .any(|job| job.job_id == job_id && job.status == CrawlerStatus::Failed));
        }

        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn get_daily_spawn_failure_cleans_up_runtime_entries_after_delay() {
        let _path_lock = crate::utils::TEST_PATH_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", "");

        let state = test_state();
        let runtime_key = daily_fallback_crawler_runtime_key("leetcode.com", "2026-03-14");

        let response = super::get_daily(
            State(state.clone()),
            Query(DailyQuery {
                domain: Some("com".to_string()),
                source: None,
                date: Some("2026-03-14".to_string()),
                r#async: Some(true),
            }),
        )
        .await
        .into_response();

        match original_path.as_ref() {
            Some(path) => std::env::set_var("PATH", path),
            None => std::env::remove_var("PATH"),
        }

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(state.daily_fallback.lock().await.contains_key(&runtime_key));
        assert!(state.crawler_jobs.lock().await.contains_key(&runtime_key));

        tokio::time::sleep(std::time::Duration::from_millis(80)).await;

        assert!(!state.daily_fallback.lock().await.contains_key(&runtime_key));
        assert!(!state.crawler_jobs.lock().await.contains_key(&runtime_key));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn get_daily_persists_running_progress_for_in_flight_daily_fallback() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let fake_uv = FakeUvGuard::install_with_trailer("sleep 0.3\nexit 0\n").await;
        let state = test_state();
        let runtime_key = daily_fallback_crawler_runtime_key("leetcode.com", "2026-03-14");

        let response = super::get_daily(
            State(state.clone()),
            Query(DailyQuery {
                domain: Some("com".to_string()),
                source: None,
                date: Some("2026-03-14".to_string()),
                r#async: Some(true),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let job_id = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            crawler_jobs.get(&runtime_key).unwrap().job_id.clone()
        };
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let still_running = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            crawler_jobs
                .get(&runtime_key)
                .map(|job| job.finished_at.is_none())
                .unwrap_or(false)
        };
        assert!(still_running, "daily fallback job should still be running");

        let progress_raw = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let progress: CrawlerProgress = serde_json::from_str(&progress_raw).unwrap();
        assert_eq!(progress.phase, CrawlerPhase::Running);
        let metadata = progress.metadata.expect("daily fallback metadata missing");
        assert_eq!(metadata.job_id, job_id);
        assert_eq!(metadata.trigger, Some(CrawlerTrigger::DailyFallback));

        wait_for_daily_job_terminal(&state, &runtime_key).await;
        drop(fake_uv);
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn get_daily_persists_progress_metadata_and_history_for_daily_fallback() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let fake_uv = FakeUvGuard::install().await;
        let state = test_state();
        let runtime_key = daily_fallback_crawler_runtime_key("leetcode.com", "2026-03-14");

        let response = super::get_daily(
            State(state.clone()),
            Query(DailyQuery {
                domain: Some("com".to_string()),
                source: None,
                date: Some("2026-03-14".to_string()),
                r#async: Some(true),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        wait_for_daily_job_terminal(&state, &runtime_key).await;

        let job_id = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            crawler_jobs.get(&runtime_key).unwrap().job_id.clone()
        };
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
        let progress_raw = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let progress: CrawlerProgress = serde_json::from_str(&progress_raw).unwrap();

        assert_eq!(progress.phase, CrawlerPhase::Completed);
        let metadata = progress.metadata.expect("daily fallback metadata missing");
        assert_eq!(metadata.job_id, job_id);
        assert_eq!(metadata.job_type, JobType::Crawler);
        assert_eq!(metadata.trigger, Some(CrawlerTrigger::DailyFallback));
        assert_eq!(metadata.source.as_deref(), Some("leetcode"));

        let env_job_id = tokio::fs::read_to_string(paths.job_dir.join("env-job-id.txt"))
            .await
            .unwrap();
        let env_job_type = tokio::fs::read_to_string(paths.job_dir.join("env-job-type.txt"))
            .await
            .unwrap();
        let env_job_dir = tokio::fs::read_to_string(paths.job_dir.join("env-job-dir.txt"))
            .await
            .unwrap();
        let env_progress_path =
            tokio::fs::read_to_string(paths.job_dir.join("env-progress-path.txt"))
                .await
                .unwrap();
        let env_python_log_path =
            tokio::fs::read_to_string(paths.job_dir.join("env-python-log-path.txt"))
                .await
                .unwrap();

        let expected_job_dir = std::env::current_dir().unwrap().join(&paths.job_dir);
        let expected_progress_path = std::env::current_dir().unwrap().join(&paths.progress);
        let expected_python_log_path = std::env::current_dir().unwrap().join(&paths.python_log);

        assert_eq!(env_job_id.trim(), job_id);
        assert_eq!(env_job_type.trim(), JobType::Crawler.as_str());
        assert_eq!(env_job_dir.trim(), expected_job_dir.to_string_lossy());
        assert_eq!(
            env_progress_path.trim(),
            expected_progress_path.to_string_lossy()
        );
        assert_eq!(
            env_python_log_path.trim(),
            expected_python_log_path.to_string_lossy()
        );

        let history = state.crawler_history.lock().await;
        let job = history.iter().find(|job| job.job_id == job_id).unwrap();
        assert_eq!(job.trigger, CrawlerTrigger::DailyFallback);
        assert_eq!(job.status, CrawlerStatus::Completed);

        drop(history);
        drop(fake_uv);
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[test]
    fn next_refresh_picks_next_utc8_slot() {
        use chrono::TimeZone;
        // UTC+8 slots 08:00/10:00/12:00 map to 00:00/02:00/04:00 UTC.
        let cases = [
            // before 08:00 UTC+8 → today 08:00
            ((2026, 6, 9, 23, 30, 0), (2026, 6, 10, 0, 0, 0)),
            // exactly 08:00 UTC+8 → today 10:00
            ((2026, 6, 10, 0, 0, 0), (2026, 6, 10, 2, 0, 0)),
            // 11:00 UTC+8 → today 12:00
            ((2026, 6, 10, 3, 0, 0), (2026, 6, 10, 4, 0, 0)),
            // after 12:00 UTC+8 → tomorrow 08:00
            ((2026, 6, 10, 5, 0, 0), (2026, 6, 11, 0, 0, 0)),
        ];
        for ((y, mo, d, h, mi, s), (ey, emo, ed, eh, emi, es)) in cases {
            let now = chrono::Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap();
            let expected = chrono::Utc
                .with_ymd_and_hms(ey, emo, ed, eh, emi, es)
                .unwrap();
            assert_eq!(
                super::next_additional_daily_source_refresh_after(now),
                expected,
                "now = {now}"
            );
        }
    }

    #[test]
    fn fallback_plan_checks_source_eligibility() {
        let env_lock = crate::utils::TEST_PATH_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let mut config = Config::default();
        config.daily_sources.tencent_docs.token_env = "OJ_TEST_0X3F_ELIGIBILITY".to_string();

        let sheep = super::DailySourceSelection::sheep();
        let plan = sheep
            .fallback_plan("2026-06-09", &config)
            .expect("sheep is always eligible");
        assert_eq!(plan.job_source, "daily_source");
        assert_eq!(plan.script, "daily_source.py");
        assert_eq!(
            plan.args,
            vec!["--daily-source", "sheep", "--date", "2026-06-09"]
        );

        let zero_x3f = super::DailySourceSelection::zero_x3f();
        std::env::remove_var("OJ_TEST_0X3F_ELIGIBILITY");
        assert!(zero_x3f.fallback_plan("2026-06-09", &config).is_none());

        config.daily_sources.tencent_docs.token = " config-token ".to_string();
        let plan = zero_x3f
            .fallback_plan("2026-06-09", &config)
            .expect("0x3f eligible with config token");
        assert_eq!(
            plan.args,
            vec!["--daily-source", "0x3f", "--date", "2026-06-09"]
        );

        config.daily_sources.tencent_docs.token.clear();
        std::env::set_var("OJ_TEST_0X3F_ELIGIBILITY", "token");
        let plan = zero_x3f
            .fallback_plan("2026-06-09", &config)
            .expect("0x3f eligible with token env");
        assert_eq!(
            plan.args,
            vec!["--daily-source", "0x3f", "--date", "2026-06-09"]
        );
        std::env::remove_var("OJ_TEST_0X3F_ELIGIBILITY");
        drop(env_lock);
    }

    #[test]
    fn additional_daily_sources_enforce_weekdays_and_start_dates() {
        let sheep = super::DailySourceSelection::sheep();
        let zero_x3f = super::DailySourceSelection::zero_x3f();

        assert!(sheep.is_available_on(chrono::NaiveDate::from_ymd_opt(2024, 2, 26).unwrap()));
        assert!(!sheep.is_available_on(chrono::NaiveDate::from_ymd_opt(2024, 2, 25).unwrap()));
        assert!(sheep.is_available_on(chrono::NaiveDate::from_ymd_opt(2026, 6, 6).unwrap()));
        assert!(!sheep.is_available_on(chrono::NaiveDate::from_ymd_opt(2026, 6, 7).unwrap()));

        assert!(zero_x3f.is_available_on(chrono::NaiveDate::from_ymd_opt(2022, 4, 8).unwrap()));
        assert!(!zero_x3f.is_available_on(chrono::NaiveDate::from_ymd_opt(2022, 4, 7).unwrap()));
        assert!(zero_x3f.is_available_on(chrono::NaiveDate::from_ymd_opt(2026, 6, 5).unwrap()));
        assert!(!zero_x3f.is_available_on(chrono::NaiveDate::from_ymd_opt(2026, 6, 6).unwrap()));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn get_daily_spawns_daily_source_fallback_for_sheep() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let fake_uv = FakeUvGuard::install().await;
        let state = test_state();
        let runtime_key = daily_fallback_crawler_runtime_key("sheep", "2026-03-14");

        let response = super::get_daily(
            State(state.clone()),
            Query(DailyQuery {
                domain: None,
                source: Some("sheep".to_string()),
                date: Some("2026-03-14".to_string()),
                r#async: Some(true),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let (job_id, source, args) = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            let job = crawler_jobs
                .get(&runtime_key)
                .expect("sheep fallback job missing");
            (job.job_id.clone(), job.source.clone(), job.args.clone())
        };
        assert_eq!(source, "daily_source");
        assert_eq!(
            args,
            vec!["--daily-source", "sheep", "--date", "2026-03-14"]
        );

        wait_for_daily_job_terminal(&state, &runtime_key).await;
        drop(fake_uv);
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn get_daily_spawns_0x3f_fallback_when_token_env_is_set() {
        let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let fake_uv = FakeUvGuard::install().await;
        std::env::set_var("OJ_TEST_0X3F_FALLBACK_TOKEN", "token");
        let mut config = Config::default();
        config.daily_sources.tencent_docs.token_env = "OJ_TEST_0X3F_FALLBACK_TOKEN".to_string();
        let state = test_state_with_config(config);
        let runtime_key = daily_fallback_crawler_runtime_key("0x3f", "2026-03-13");

        let response = super::get_daily(
            State(state.clone()),
            Query(DailyQuery {
                domain: None,
                source: Some("0x3f".to_string()),
                date: Some("2026-03-13".to_string()),
                r#async: Some(true),
            }),
        )
        .await
        .into_response();
        std::env::remove_var("OJ_TEST_0X3F_FALLBACK_TOKEN");
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let (job_id, args) = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            let job = crawler_jobs
                .get(&runtime_key)
                .expect("0x3f fallback job missing");
            (job.job_id.clone(), job.args.clone())
        };
        assert_eq!(args, vec!["--daily-source", "0x3f", "--date", "2026-03-13"]);

        wait_for_daily_job_terminal(&state, &runtime_key).await;
        drop(fake_uv);
        let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
        let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    }

    #[tokio::test]
    async fn get_daily_returns_ingestion_required_for_0x3f_without_token_env() {
        let mut config = Config::default();
        config.daily_sources.tencent_docs.token_env = "OJ_TEST_0X3F_MISSING_TOKEN".to_string();
        let state = test_state_with_config(config);
        let runtime_key = daily_fallback_crawler_runtime_key("0x3f", "2026-03-13");

        let response = super::get_daily(
            State(state.clone()),
            Query(DailyQuery {
                domain: None,
                source: Some("0x3f".to_string()),
                date: Some("2026-03-13".to_string()),
                r#async: Some(true),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["status"], "ingestion_required");
        assert_eq!(parsed["retry_after"], 30);
        assert_eq!(parsed["job_started"], false);

        assert!(!state.daily_fallback.lock().await.contains_key(&runtime_key));
        assert!(!state.crawler_jobs.lock().await.contains_key(&runtime_key));
    }

    #[tokio::test]
    async fn get_daily_reuses_running_additional_source_fallback() {
        let state = test_state();
        let runtime_key = daily_fallback_crawler_runtime_key("sheep", "2026-03-14");
        state
            .daily_fallback
            .lock()
            .await
            .insert(runtime_key.clone(), fallback_entry());

        let response = super::get_daily(
            State(state.clone()),
            Query(DailyQuery {
                domain: None,
                source: Some("sheep".to_string()),
                date: Some("2026-03-14".to_string()),
                r#async: Some(true),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        // No new job spawned: existing Running entry is reused untouched.
        assert!(!state.crawler_jobs.lock().await.contains_key(&runtime_key));
        let fallback = state.daily_fallback.lock().await;
        assert_eq!(
            fallback
                .get(&runtime_key)
                .map(|entry| entry.job_id.as_str()),
            Some("daily-running")
        );
    }
}
