//! Persistent per-source schedules. All database functions run on blocking workers.
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use chrono::{DateTime, Timelike, Utc};
use chrono_tz::Tz;
use croner::Cron;
use serde::{Deserialize, Serialize};

use crate::{models::CrawlerSource, AppState};

pub const SETTING_KEY: &str = "crawler_schedules";
pub const SOURCES: &[&str] = &["leetcode", "atcoder", "codeforces", "luogu", "spoj"];

#[derive(Default)]
pub struct ScheduleRuntime {
    pub changed: tokio::sync::Notify,
    // 設定更新與到期派工共用鎖，停用成功後不再派發舊設定。
    pub update: tokio::sync::Mutex<()>,
    // 一般、排程與 daily 的註冊／啟動共用鎖，避免來源檢查競態。
    pub launch: tokio::sync::Mutex<()>,
    pub stopping: AtomicBool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CrawlerSchedule {
    pub enabled: bool,
    pub cron: String,
    /// Empty means inherit crawler.default_timezone (UTC by default).
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub args: Vec<String>,
}

pub type CrawlerSchedules = BTreeMap<String, CrawlerSchedule>;

pub fn parse_timezone(value: &str) -> Result<Tz, String> {
    value
        .parse()
        .map_err(|_| format!("invalid IANA timezone: {value}"))
}

pub fn effective_timezone<'a>(schedule: &'a CrawlerSchedule, default: &'a str) -> &'a str {
    if schedule.timezone.is_empty() {
        default
    } else {
        &schedule.timezone
    }
}

pub fn parse_cron(value: &str) -> Result<Cron, String> {
    if value.len() > 256 || value.split_whitespace().count() != 5 {
        return Err("cron must contain exactly five fields: minute hour day month weekday".into());
    }
    // 僅接受標準五欄語法；不接受秒、年或 Quartz 擴充。
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "*,-/ \t".contains(c))
    {
        return Err("unsupported cron syntax; use five standard fields".into());
    }
    value.parse().map_err(|err| format!("invalid cron: {err}"))
}

pub fn next_runs_after(
    schedule: &CrawlerSchedule,
    default_timezone: &str,
    now: DateTime<Utc>,
    count: usize,
) -> Result<Vec<DateTime<Utc>>, String> {
    let cron = parse_cron(&schedule.cron)?;
    let timezone = parse_timezone(effective_timezone(schedule, default_timezone))?;
    let mut cursor = now
        .with_timezone(&timezone)
        .with_nanosecond(0)
        .expect("zero nanoseconds is valid");
    let mut result = Vec::new();
    for _ in 0..count {
        cursor = cron
            .find_next_occurrence(&cursor, false)
            .map_err(|err| format!("cron has no future occurrence: {err}"))?;
        result.push(cursor.with_timezone(&Utc));
    }
    Ok(result)
}

pub fn validate_schedule(
    source: &str,
    schedule: &mut CrawlerSchedule,
    default: &str,
) -> Result<(), String> {
    if !SOURCES.contains(&source) {
        return Err(format!("source cannot be scheduled: {source}"));
    }
    let parsed = CrawlerSource::parse(source)?;
    schedule.cron = schedule
        .cron
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    schedule.timezone = schedule.timezone.trim().to_owned();
    parse_timezone(effective_timezone(schedule, default))?;
    if schedule.enabled || !schedule.cron.is_empty() {
        next_runs_after(schedule, default, Utc::now(), 1)?;
    }
    if schedule.args.len() > 100 || schedule.args.iter().any(|arg| arg.len() > 1024) {
        return Err("too many or excessively long crawler arguments".into());
    }
    schedule.args = crate::models::validate_args(&parsed, &schedule.args)?;
    // SPOJ 與 Luogu 共用腳本，必須綁定來源以確保互斥及結果正確。
    if matches!(source, "spoj" | "luogu") {
        if let Some(index) = schedule.args.iter().position(|arg| arg == "--source") {
            if schedule.args[index + 1] != source {
                return Err("--source must match the schedule source".into());
            }
        } else {
            schedule.args.extend(["--source".into(), source.into()]);
        }
    }
    Ok(())
}

pub fn default_schedules() -> CrawlerSchedules {
    SOURCES
        .iter()
        .map(|s| ((*s).into(), CrawlerSchedule::default()))
        .collect()
}

pub fn load_schedules(pool: &crate::db::DbPool, default: &str) -> Result<CrawlerSchedules, String> {
    use rusqlite::OptionalExtension;
    let conn = pool.get().map_err(|e| e.to_string())?;
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key=?1",
            [SETTING_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let mut result = default_schedules();
    let Some(raw) = raw else {
        return Ok(result);
    };
    let stored: BTreeMap<String, serde_json::Value> = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!("invalid stored schedules; disabled defaults used: {err}");
            return Ok(result);
        }
    };
    for (source, value) in stored {
        if !SOURCES.contains(&source.as_str()) {
            continue;
        }
        match serde_json::from_value::<CrawlerSchedule>(value) {
            Ok(mut schedule) => match validate_schedule(&source, &mut schedule, default) {
                Ok(()) => {
                    result.insert(source, schedule);
                }
                Err(err) => tracing::warn!("disabled invalid schedule {source}: {err}"),
            },
            Err(err) => tracing::warn!("disabled invalid schedule {source}: {err}"),
        }
    }
    Ok(result)
}

pub fn save_schedules(
    pool: &crate::db::DbPool,
    schedules: &CrawlerSchedules,
) -> Result<(), String> {
    let raw = serde_json::to_string(schedules).map_err(|e| e.to_string())?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO app_settings (key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [SETTING_KEY, &raw]).map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn read_schedules(state: &AppState) -> Result<CrawlerSchedules, String> {
    let pool = state.rw_pool.clone();
    let default = state.config.crawler.default_timezone.clone();
    tokio::task::spawn_blocking(move || load_schedules(&pool, &default))
        .await
        .map_err(|e| e.to_string())?
}

pub fn spawn_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        if let Err(err) = run_scheduler(state).await {
            tracing::error!("crawler scheduler stopped: {err}");
        }
    });
}

type Deadlines = BTreeMap<String, (CrawlerSchedule, DateTime<Utc>)>;

fn reconcile_deadlines(
    deadlines: &mut Deadlines,
    schedules: CrawlerSchedules,
    now: DateTime<Utc>,
    default_timezone: &str,
) -> Result<(), String> {
    deadlines.retain(|source, (old, _)| {
        schedules
            .get(source)
            .is_some_and(|new| new.enabled && new == old)
    });
    for (source, schedule) in schedules.into_iter().filter(|(_, s)| s.enabled) {
        if let std::collections::btree_map::Entry::Vacant(entry) = deadlines.entry(source) {
            let next = next_runs_after(&schedule, default_timezone, now, 1)?[0];
            entry.insert((schedule, next));
        }
    }
    Ok(())
}

// 每次只保留下一次執行；重啟及設定修改不補跑停機期間的工作。
async fn run_scheduler(state: Arc<AppState>) -> Result<(), String> {
    let mut deadlines = Deadlines::new();
    loop {
        let lock = state.crawler_schedule.update.lock().await;
        if state.crawler_schedule.stopping.load(Ordering::Acquire) {
            return Ok(());
        }
        let schedules = match read_schedules(&state).await {
            Ok(s) => s,
            Err(err) => {
                tracing::error!("failed to load crawler schedules: {err}");
                drop(lock);
                tokio::select! { _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {}, _ = state.crawler_schedule.changed.notified() => {} }
                continue;
            }
        };
        let now = Utc::now();
        reconcile_deadlines(
            &mut deadlines,
            schedules,
            now,
            &state.config.crawler.default_timezone,
        )?;
        for (source, (schedule, deadline)) in deadlines.iter_mut() {
            if *deadline <= now {
                let result = super::crawler::launch(
                    state.clone(),
                    source.clone(),
                    schedule.args.clone(),
                    crate::models::CrawlerTrigger::Scheduled,
                )
                .await;
                if let Err(err) = result {
                    tracing::warn!(
                        "scheduled crawler {source} skipped or failed: {}",
                        err.detail
                    );
                    let pool = state.rw_pool.clone();
                    let key = format!("crawler_schedule_event:{source}");
                    let value =
                        serde_json::json!({"at": Utc::now().to_rfc3339(), "message": err.detail})
                            .to_string();
                    let _ = tokio::task::spawn_blocking(move || {
                        crate::db::settings::set_setting(&pool, &key, &value)
                    })
                    .await;
                }
                *deadline = next_runs_after(
                    schedule,
                    &state.config.crawler.default_timezone,
                    Utc::now(),
                    1,
                )?[0];
            }
        }
        // 最多等待 30 秒，以便主機時鐘調整後重新檢查 UTC 到期時間。
        let wait = deadlines
            .values()
            .map(|(_, next)| (*next - Utc::now()).to_std().unwrap_or_default())
            .min()
            .unwrap_or(std::time::Duration::from_secs(30))
            .min(std::time::Duration::from_secs(30));
        drop(lock);
        tokio::select! { _ = tokio::time::sleep(wait) => {}, _ = state.crawler_schedule.changed.notified() => {} }
    }
}

#[cfg(test)]
mod tests;
