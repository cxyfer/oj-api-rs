#![allow(clippy::await_holding_lock)]

use std::collections::{HashMap, HashSet, VecDeque};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use rand::Rng;
use tokio::sync::{Notify, RwLock, Semaphore};

use super::common::{
    apply_terminal_update, embedding_launch_allowed, embedding_trigger_conflicts,
    finalize_owned_manual_crawler_job, manual_crawler_launch_allowed, manual_trigger_conflicts,
    persist_crawler_terminal_progress, take_manual_crawler_pid, take_owned_manual_crawler_pid,
    with_owned_manual_crawler_job,
};
use super::crawler::TriggerCrawlerRequest;
use super::embedding::TriggerEmbeddingRequest;
use crate::config::Config;
use crate::models::{
    daily_fallback_crawler_runtime_key, manual_crawler_runtime_key, ActiveCrawlerPid,
    CrawlerJob, CrawlerPhase, CrawlerProgress, CrawlerStatus, CrawlerTrigger,
    DailyFallbackEntry, EmbeddingJob, JobType,
};
use crate::utils::CapturedOutput;
use crate::AppState;

fn collect_running_job_keys<'a>(
    crawler_jobs: impl IntoIterator<Item = &'a CrawlerJob>,
    embedding_job: Option<&EmbeddingJob>,
) -> HashSet<(JobType, String)> {
    let mut active_jobs = HashSet::new();
    active_jobs.extend(
        crawler_jobs
            .into_iter()
            .filter(|job| job.status == CrawlerStatus::Running)
            .map(|job| (JobType::Crawler, job.job_id.clone())),
    );
    if let Some(job) = embedding_job.filter(|job| job.status == CrawlerStatus::Running) {
        active_jobs.insert((JobType::Embedding, job.job_id.clone()));
    }
    active_jobs
}
fn test_state() -> Arc<AppState> {
    let config = Config::default();
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

fn crawler_job(job_id: &str, trigger: CrawlerTrigger, status: CrawlerStatus) -> CrawlerJob {
    CrawlerJob {
        job_id: job_id.to_string(),
        source: "leetcode".to_string(),
        args: vec![],
        trigger,
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: None,
        status,
        stdout: None,
        stderr: None,
    }
}

#[test]
fn apply_terminal_update_preserves_cancelled_crawler_and_keeps_tail() {
    let mut job = CrawlerJob {
        job_id: "job-1".to_string(),
        source: "leetcode".to_string(),
        args: vec![],
        trigger: CrawlerTrigger::Admin,
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: Some("2026-03-14T00:01:00Z".to_string()),
        status: CrawlerStatus::Cancelled,
        stdout: None,
        stderr: None,
    };

    apply_terminal_update(
        &mut job,
        None,
        Ok(CapturedOutput {
            stdout: b"crawler stdout".to_vec(),
            stderr: b"crawler stderr".to_vec(),
        }),
        "crawler",
    );

    assert_eq!(job.status, CrawlerStatus::Cancelled);
    assert_eq!(job.stdout.as_deref(), Some("crawler stdout"));
    assert_eq!(job.stderr.as_deref(), Some("crawler stderr"));
}

#[test]
fn apply_terminal_update_marks_embedding_timed_out_and_keeps_tail() {
    let mut job = EmbeddingJob {
        job_id: "job-2".to_string(),
        source: "all".to_string(),
        args: vec![],
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    };

    apply_terminal_update(
        &mut job,
        Some(CrawlerStatus::TimedOut),
        Ok(CapturedOutput {
            stdout: b"embedding stdout".to_vec(),
            stderr: b"embedding stderr".to_vec(),
        }),
        "embedding",
    );

    assert_eq!(job.status, CrawlerStatus::TimedOut);
    assert_eq!(job.stdout.as_deref(), Some("embedding stdout"));
    assert_eq!(job.stderr.as_deref(), Some("embedding stderr"));
}

#[test]
fn apply_terminal_update_does_not_regress_existing_terminal_status() {
    let mut job = CrawlerJob {
        job_id: "job-3".to_string(),
        source: "leetcode".to_string(),
        args: vec![],
        trigger: CrawlerTrigger::Admin,
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: Some("2026-03-14T00:01:00Z".to_string()),
        status: CrawlerStatus::Cancelled,
        stdout: None,
        stderr: None,
    };

    apply_terminal_update(
        &mut job,
        Some(CrawlerStatus::Completed),
        Ok(CapturedOutput {
            stdout: b"late stdout".to_vec(),
            stderr: b"late stderr".to_vec(),
        }),
        "crawler",
    );

    assert_eq!(job.status, CrawlerStatus::Cancelled);
    assert_eq!(job.stdout.as_deref(), Some("late stdout"));
    assert_eq!(job.stderr.as_deref(), Some("late stderr"));
}

#[test]
fn embedding_launch_allowed_only_for_owned_running_job() {
    let running = EmbeddingJob {
        job_id: "embed-1".to_string(),
        source: "all".to_string(),
        args: vec![],
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    };
    let cancelled = EmbeddingJob {
        status: CrawlerStatus::Cancelled,
        finished_at: Some("2026-03-14T00:01:00Z".to_string()),
        ..running.clone()
    };

    assert!(!embedding_trigger_conflicts(&None));
    assert!(embedding_trigger_conflicts(&Some("embed-1".to_string())));
    assert!(embedding_launch_allowed(
        &Some("embed-1".to_string()),
        Some(&running),
        "embed-1"
    ));
    assert!(!embedding_launch_allowed(
        &Some("embed-1".to_string()),
        Some(&running),
        "embed-2"
    ));
    assert!(!embedding_launch_allowed(
        &Some("embed-1".to_string()),
        Some(&cancelled),
        "embed-1"
    ));
    assert!(!embedding_launch_allowed(&None, Some(&running), "embed-1"));
    assert!(!embedding_launch_allowed(
        &Some("embed-1".to_string()),
        None,
        "embed-1"
    ));
}

#[test]
fn collect_running_job_keys_includes_all_running_job_types() {
    let manual_job = CrawlerJob {
        job_id: "crawler-running".to_string(),
        source: "leetcode".to_string(),
        args: vec![],
        trigger: CrawlerTrigger::Admin,
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    };
    let daily_job = CrawlerJob {
        job_id: "daily-running".to_string(),
        source: "leetcode".to_string(),
        args: vec![
            "--daily".to_string(),
            "--domain".to_string(),
            "com".to_string(),
        ],
        trigger: CrawlerTrigger::DailyFallback,
        started_at: "2026-03-14T00:02:00Z".to_string(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    };
    let completed_daily_job = CrawlerJob {
        status: CrawlerStatus::Completed,
        ..daily_job.clone()
    };
    let embedding_job = EmbeddingJob {
        job_id: "embedding-running".to_string(),
        source: "all".to_string(),
        args: vec![],
        started_at: "2026-03-14T00:01:00Z".to_string(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    };
    let crawler_jobs = HashMap::from([
        (manual_crawler_runtime_key().to_string(), manual_job),
        (
            daily_fallback_crawler_runtime_key("com", "2026-03-14"),
            daily_job,
        ),
        (
            daily_fallback_crawler_runtime_key("com", "2026-03-13"),
            completed_daily_job,
        ),
    ]);

    let active = collect_running_job_keys(crawler_jobs.values(), Some(&embedding_job));

    assert_eq!(
        active,
        HashSet::from([
            (JobType::Crawler, "crawler-running".to_string()),
            (JobType::Crawler, "daily-running".to_string()),
            (JobType::Embedding, "embedding-running".to_string()),
        ])
    );
}

#[test]
fn manual_trigger_conflicts_only_when_manual_guard_is_present() {
    assert!(!manual_trigger_conflicts(&None));
    assert!(manual_trigger_conflicts(&Some(
        "manual-running".to_string()
    )));
}

#[test]
fn take_manual_crawler_pid_only_removes_manual_slot() {
    let daily_key = daily_fallback_crawler_runtime_key("com", "2026-03-14");
    let mut pids = HashMap::from([
        (
            manual_crawler_runtime_key().to_string(),
            ActiveCrawlerPid {
                job_id: "manual-job".to_string(),
                pid: 101_u32,
            },
        ),
        (
            daily_key.clone(),
            ActiveCrawlerPid {
                job_id: "daily-job".to_string(),
                pid: 202_u32,
            },
        ),
    ]);

    assert_eq!(take_manual_crawler_pid(&mut pids), Some(101_u32));
    assert_eq!(
        pids.get(&daily_key),
        Some(&ActiveCrawlerPid {
            job_id: "daily-job".to_string(),
            pid: 202_u32,
        })
    );
    assert_eq!(take_manual_crawler_pid(&mut pids), None);
}

#[test]
fn take_owned_manual_crawler_pid_requires_matching_manual_job() {
    let manual_key = manual_crawler_runtime_key();
    let mut pids = HashMap::from([(
        manual_key.to_string(),
        ActiveCrawlerPid {
            job_id: "new-job".to_string(),
            pid: 303_u32,
        },
    )]);

    assert_eq!(take_owned_manual_crawler_pid(&mut pids, "old-job"), None);
    assert_eq!(
        pids.get(manual_key),
        Some(&ActiveCrawlerPid {
            job_id: "new-job".to_string(),
            pid: 303_u32,
        })
    );
    assert_eq!(
        take_owned_manual_crawler_pid(&mut pids, "new-job"),
        Some(303_u32)
    );
    assert_eq!(pids.get(manual_key), None);
}

#[test]
fn with_owned_manual_crawler_job_ignores_reused_manual_slot() {
    let manual_key = manual_crawler_runtime_key().to_string();
    let mut crawler_jobs = HashMap::from([(
        manual_key.clone(),
        crawler_job("new-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
    )]);

    let updated = with_owned_manual_crawler_job(&mut crawler_jobs, "old-job", |job| {
        job.status = CrawlerStatus::Failed;
        job.finished_at = Some("2026-03-15T00:00:00Z".to_string());
        job.clone()
    });

    assert!(updated.is_none());
    let job = crawler_jobs.get(&manual_key).unwrap();
    assert_eq!(job.job_id, "new-job");
    assert_eq!(job.status, CrawlerStatus::Running);
    assert!(job.finished_at.is_none());
}

#[test]
fn finalize_owned_manual_crawler_job_ignores_reused_manual_slot() {
    let manual_key = manual_crawler_runtime_key().to_string();
    let mut crawler_jobs = HashMap::from([(
        manual_key.clone(),
        crawler_job("new-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
    )]);

    let finished_job = finalize_owned_manual_crawler_job(
        &mut crawler_jobs,
        "old-job",
        Some(CrawlerStatus::Failed),
        Ok(CapturedOutput {
            stdout: b"old stdout".to_vec(),
            stderr: b"old stderr".to_vec(),
        }),
    );

    assert!(finished_job.is_none());
    let job = crawler_jobs.get(&manual_key).unwrap();
    assert_eq!(job.job_id, "new-job");
    assert_eq!(job.status, CrawlerStatus::Running);
    assert!(job.finished_at.is_none());
    assert!(job.stdout.is_none());
    assert!(job.stderr.is_none());
}

#[test]
fn manual_crawler_launch_allowed_only_for_owned_running_job() {
    let running_job = crawler_job(
        "manual-running",
        CrawlerTrigger::Admin,
        CrawlerStatus::Running,
    );
    let cancelled_job = crawler_job(
        "manual-running",
        CrawlerTrigger::Admin,
        CrawlerStatus::Cancelled,
    );

    assert!(manual_crawler_launch_allowed(
        &Some("manual-running".to_string()),
        Some(&running_job),
        "manual-running",
    ));
    assert!(!manual_crawler_launch_allowed(
        &Some("manual-running".to_string()),
        Some(&cancelled_job),
        "manual-running",
    ));
    assert!(!manual_crawler_launch_allowed(
        &Some("other-job".to_string()),
        Some(&running_job),
        "manual-running",
    ));
    assert!(!manual_crawler_launch_allowed(
        &Some("manual-running".to_string()),
        None,
        "manual-running",
    ));
}

#[tokio::test]
async fn trigger_crawler_returns_conflict_when_manual_job_is_reserved() {
    let state = test_state();
    *state.manual_crawler_guard.lock().await = Some("manual-running".to_string());

    let response = super::crawler::trigger_crawler(
        State(state),
        Json(TriggerCrawlerRequest {
            source: "leetcode".to_string(),
            args: vec![],
        }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn cancel_crawler_clears_manual_guard_before_pid_registration() {
    let state = test_state();
    let runtime_key = manual_crawler_runtime_key().to_string();
    *state.manual_crawler_guard.lock().await = Some("manual-job".to_string());
    state.crawler_jobs.lock().await.insert(
        runtime_key,
        crawler_job("manual-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
    );

    let response = super::crawler::cancel_crawler(State(state.clone()))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(*state.manual_crawler_guard.lock().await, None);
    let crawler_jobs = state.crawler_jobs.lock().await;
    let job = crawler_jobs.get(manual_crawler_runtime_key()).unwrap();
    assert_eq!(job.status, CrawlerStatus::Cancelled);
    assert!(job.finished_at.is_some());
}

#[tokio::test]
async fn cancel_crawler_waits_for_manual_launch_critical_section() {
    let state = test_state();
    let runtime_key = manual_crawler_runtime_key().to_string();
    *state.manual_crawler_guard.lock().await = Some("manual-job".to_string());
    state.crawler_jobs.lock().await.insert(
        runtime_key,
        crawler_job("manual-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
    );

    let launch_guard = state.manual_crawler_guard.lock().await;
    let cancel_state = state.clone();
    let cancel_task = tokio::spawn(async move {
        super::crawler::cancel_crawler(State(cancel_state))
            .await
            .into_response()
            .status()
    });

    tokio::task::yield_now().await;
    assert!(!cancel_task.is_finished());

    drop(launch_guard);

    let status = cancel_task.await.unwrap();
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn cancel_embedding_waits_for_launch_critical_section() {
    let state = test_state();
    *state.embedding_lock.lock().await = Some(EmbeddingJob {
        job_id: "embed-job".to_string(),
        source: "all".to_string(),
        args: vec![],
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    });
    *state.embedding_launch_guard.lock().await = Some("embed-job".to_string());

    let launch_guard = state.embedding_launch_guard.lock().await;
    let cancel_state = state.clone();
    let cancel_task = tokio::spawn(async move {
        super::embedding::cancel_embedding(State(cancel_state))
            .await
            .into_response()
            .status()
    });

    tokio::task::yield_now().await;
    assert!(!cancel_task.is_finished());

    drop(launch_guard);

    let status = cancel_task.await.unwrap();
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn cancel_crawler_persists_cancelled_progress_before_pid_registration() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let state = test_state();
    let runtime_key = manual_crawler_runtime_key().to_string();
    let job_id = uuid::Uuid::new_v4().to_string();
    let job = CrawlerJob {
        job_id: job_id.clone(),
        ..crawler_job(&job_id, CrawlerTrigger::Admin, CrawlerStatus::Running)
    };
    let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    crate::utils::persist_job_metadata(&paths, crate::models::JobArtifactMetadata::from(&job))
        .await
        .unwrap();

    *state.manual_crawler_guard.lock().await = Some(job_id.clone());
    state.crawler_jobs.lock().await.insert(runtime_key, job);

    let response = super::crawler::cancel_crawler(State(state.clone()))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
    let progress: CrawlerProgress = serde_json::from_str(&content).unwrap();
    assert_eq!(progress.phase, CrawlerPhase::Cancelled);
    assert!(progress.metadata.is_some());

    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[tokio::test]
async fn cancel_crawler_preserves_cancelled_job_in_history_when_manual_slot_is_reused() {
    let state = test_state();
    let runtime_key = manual_crawler_runtime_key().to_string();
    *state.manual_crawler_guard.lock().await = Some("old-job".to_string());
    state.crawler_jobs.lock().await.insert(
        runtime_key.clone(),
        crawler_job("old-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
    );

    let response = super::crawler::cancel_crawler(State(state.clone()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    state.crawler_jobs.lock().await.insert(
        runtime_key.clone(),
        crawler_job("new-job", CrawlerTrigger::Admin, CrawlerStatus::Running),
    );

    let finished_job = {
        let mut crawler_jobs = state.crawler_jobs.lock().await;
        finalize_owned_manual_crawler_job(
            &mut crawler_jobs,
            "old-job",
            None,
            Ok(CapturedOutput {
                stdout: b"old stdout".to_vec(),
                stderr: b"old stderr".to_vec(),
            }),
        )
    };

    assert!(finished_job.is_none());

    let crawler_jobs = state.crawler_jobs.lock().await;
    let new_job = crawler_jobs.get(&runtime_key).unwrap();
    assert_eq!(new_job.job_id, "new-job");
    assert_eq!(new_job.status, CrawlerStatus::Running);
    drop(crawler_jobs);

    let history = state.crawler_history.lock().await;
    let cancelled_job = history.iter().find(|job| job.job_id == "old-job");
    assert!(cancelled_job.is_some());
    let cancelled_job = cancelled_job.unwrap();
    assert_eq!(cancelled_job.status, CrawlerStatus::Cancelled);
    assert!(cancelled_job.finished_at.is_some());
}

#[cfg(unix)]
struct JobArtifactsRootBlocker {
    root: std::path::PathBuf,
    backup: Option<std::path::PathBuf>,
    _root_lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(unix)]
impl JobArtifactsRootBlocker {
    async fn install() -> Self {
        let root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let root = std::env::current_dir()
            .unwrap()
            .join(crate::models::JOB_ARTIFACTS_ROOT);
        let backup = if tokio::fs::metadata(&root).await.is_ok() {
            let backup = root.with_file_name(format!(
                ".job-artifacts-backup-{}-{}",
                std::process::id(),
                rand::thread_rng().r#gen::<u64>()
            ));
            tokio::fs::rename(&root, &backup).await.unwrap();
            Some(backup)
        } else {
            None
        };
        if let Some(parent) = root.parent() {
            tokio::fs::create_dir_all(parent).await.unwrap();
        }
        tokio::fs::write(&root, b"blocked").await.unwrap();

        Self {
            root,
            backup,
            _root_lock: root_lock,
        }
    }
}

#[cfg(unix)]
impl Drop for JobArtifactsRootBlocker {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.root);
        if let Some(backup) = self.backup.as_ref() {
            let _ = std::fs::rename(backup, &self.root);
        }
    }
}

#[cfg(unix)]
struct FakeUvGuard {
    root: std::path::PathBuf,
    original_path: Option<std::ffi::OsString>,
    _path_lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(unix)]
impl FakeUvGuard {
    async fn install(script_body: &str) -> Self {
        let path_lock = crate::utils::TEST_PATH_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let root = std::env::temp_dir().join(format!(
            "oj-api-rs-fake-uv-{}-{}",
            std::process::id(),
            rand::thread_rng().r#gen::<u64>()
        ));
        tokio::fs::create_dir_all(&root).await.unwrap();
        let uv_path = root.join("uv");
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
    }
}

#[cfg(unix)]
async fn wait_for_manual_job_terminal(state: &Arc<AppState>, runtime_key: &str) {
    for _ in 0..100 {
        let finished = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            crawler_jobs
                .get(runtime_key)
                .map(|job| job.finished_at.is_some())
                .unwrap_or(false)
        };
        if finished {
            let (_, progress) = read_crawler_progress_from_state(state, runtime_key).await;
            if matches!(
                progress.phase,
                CrawlerPhase::Completed
                    | CrawlerPhase::Failed
                    | CrawlerPhase::Cancelled
                    | CrawlerPhase::TimedOut
            ) {
                return;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("manual crawler did not reach terminal state in time");
}

#[cfg(unix)]
async fn wait_for_embedding_job_terminal(state: &Arc<AppState>) {
    for _ in 0..100 {
        let done = {
            let lock = state.embedding_lock.lock().await;
            lock.as_ref()
                .map(|job| job.finished_at.is_some())
                .unwrap_or(false)
        };
        if done {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("embedding job did not reach terminal state in time");
}

#[cfg(unix)]
async fn read_crawler_progress_from_state(
    state: &Arc<AppState>,
    runtime_key: &str,
) -> (String, CrawlerProgress) {
    let job_id = {
        let crawler_jobs = state.crawler_jobs.lock().await;
        crawler_jobs.get(runtime_key).unwrap().job_id.clone()
    };
    let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
    let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
    let progress: CrawlerProgress = serde_json::from_str(&content).unwrap();
    (job_id, progress)
}

#[cfg(unix)]
async fn wait_for_manual_job_progress_phase(
    state: &Arc<AppState>,
    runtime_key: &str,
    phase: CrawlerPhase,
) -> bool {
    for _ in 0..100 {
        let finished = {
            let crawler_jobs = state.crawler_jobs.lock().await;
            crawler_jobs
                .get(runtime_key)
                .and_then(|job| job.finished_at.as_ref())
                .is_some()
        };
        let (_, progress) = read_crawler_progress_from_state(state, runtime_key).await;
        if progress.phase == phase {
            return true;
        }
        if finished {
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    false
}

#[cfg(unix)]
#[tokio::test]
async fn trigger_crawler_injects_job_env_into_manual_subprocess() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let fake_uv = FakeUvGuard::install(
        "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\nprintf '%s\\n' \"$OJ_JOB_ID\" > \"$OJ_JOB_DIR/env-job-id.txt\"\nprintf '%s\\n' \"$OJ_JOB_TYPE\" > \"$OJ_JOB_DIR/env-job-type.txt\"\nprintf '%s\\n' \"$OJ_JOB_DIR\" > \"$OJ_JOB_DIR/env-job-dir.txt\"\nprintf '%s\\n' \"$OJ_PROGRESS_PATH\" > \"$OJ_JOB_DIR/env-progress-path.txt\"\nprintf '%s\\n' \"$OJ_PYTHON_LOG_PATH\" > \"$OJ_JOB_DIR/env-python-log-path.txt\"\nexit 0\n",
    )
    .await;
    let state = test_state();
    let runtime_key = manual_crawler_runtime_key().to_string();

    let response = super::crawler::trigger_crawler(
        State(state.clone()),
        Json(TriggerCrawlerRequest {
            source: "leetcode".to_string(),
            args: vec![],
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    wait_for_manual_job_terminal(&state, &runtime_key).await;

    let job_id = {
        let crawler_jobs = state.crawler_jobs.lock().await;
        crawler_jobs.get(&runtime_key).unwrap().job_id.clone()
    };
    let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();

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

    assert_eq!(env_job_id.trim(), job_id);
    let expected_job_dir = std::env::current_dir().unwrap().join(&paths.job_dir);
    let expected_progress_path = std::env::current_dir().unwrap().join(&paths.progress);
    let expected_python_log_path = std::env::current_dir().unwrap().join(&paths.python_log);

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

    drop(fake_uv);
}

#[cfg(unix)]
#[tokio::test]
async fn trigger_crawler_fails_before_spawn_when_metadata_persist_fails() {
    let root_blocker = JobArtifactsRootBlocker::install().await;
    let spawned_marker = std::env::temp_dir().join(format!(
        "oj-api-rs-manual-crawler-spawned-{}-{}",
        std::process::id(),
        rand::thread_rng().r#gen::<u64>()
    ));
    let _ = tokio::fs::remove_file(&spawned_marker).await;
    let fake_uv = FakeUvGuard::install(&format!(
        "#!/bin/sh\nprintf 'spawned\\n' > \"{}\"\nexit 0\n",
        spawned_marker.display()
    ))
    .await;
    let state = test_state();
    let runtime_key = manual_crawler_runtime_key().to_string();

    let response = super::crawler::trigger_crawler(
        State(state.clone()),
        Json(TriggerCrawlerRequest {
            source: "leetcode".to_string(),
            args: vec![],
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(*state.manual_crawler_guard.lock().await, None);
    assert!(state.active_crawler_pids.lock().await.is_empty());

    let job = {
        let crawler_jobs = state.crawler_jobs.lock().await;
        crawler_jobs.get(&runtime_key).cloned().unwrap()
    };
    assert_eq!(job.status, CrawlerStatus::Failed);
    assert!(job.finished_at.is_some());

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(tokio::fs::metadata(&spawned_marker).await.is_err());
    let history = state.crawler_history.lock().await;
    assert!(history
        .iter()
        .any(|entry| entry.job_id == job.job_id && entry.status == CrawlerStatus::Failed));

    drop(fake_uv);
    let _ = tokio::fs::remove_file(&spawned_marker).await;
    drop(root_blocker);
}

#[cfg(unix)]
#[tokio::test]
async fn trigger_crawler_persists_running_progress_phase_for_manual_job() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let fake_uv = FakeUvGuard::install(
        "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\nsleep 0.3\nexit 0\n",
    )
    .await;
    let state = test_state();
    let runtime_key = manual_crawler_runtime_key().to_string();

    let response = super::crawler::trigger_crawler(
        State(state.clone()),
        Json(TriggerCrawlerRequest {
            source: "leetcode".to_string(),
            args: vec![],
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    assert!(
        wait_for_manual_job_progress_phase(&state, &runtime_key, CrawlerPhase::Running).await
    );

    wait_for_manual_job_terminal(&state, &runtime_key).await;

    let (_, progress) = read_crawler_progress_from_state(&state, &runtime_key).await;
    assert_eq!(progress.phase, CrawlerPhase::Completed);
    assert!(progress.metadata.is_some());

    drop(fake_uv);
}

#[tokio::test]
async fn persist_crawler_terminal_progress_preserves_existing_terminal_phase() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let job_id = uuid::Uuid::new_v4().to_string();
    let job = CrawlerJob {
        job_id: job_id.clone(),
        source: "leetcode".to_string(),
        args: vec![],
        trigger: CrawlerTrigger::Admin,
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: Some("2026-03-14T00:01:00Z".to_string()),
        status: CrawlerStatus::Completed,
        stdout: None,
        stderr: None,
    };
    let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    crate::utils::persist_job_metadata(&paths, crate::models::JobArtifactMetadata::from(&job))
        .await
        .unwrap();
    crate::utils::write_crawler_progress(
        &paths,
        &CrawlerProgress {
            phase: CrawlerPhase::Cancelled,
            message: Some("cancelled earlier".to_string()),
            updated_at: Some("2026-03-14T00:00:30Z".to_string()),
            metadata: None,
        },
    )
    .await
    .unwrap();

    persist_crawler_terminal_progress(&paths, &job).await;

    let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
    let progress: CrawlerProgress = serde_json::from_str(&content).unwrap();
    assert_eq!(progress.phase, CrawlerPhase::Cancelled);
    assert_eq!(progress.message.as_deref(), Some("cancelled earlier"));
    assert!(progress.metadata.is_some());

    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[cfg(unix)]
#[tokio::test]
async fn trigger_embedding_injects_job_env_into_subprocess() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let fake_uv = FakeUvGuard::install(
        "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\nprintf '%s\\n' \"$OJ_JOB_ID\" > \"$OJ_JOB_DIR/env-job-id.txt\"\nprintf '%s\\n' \"$OJ_JOB_TYPE\" > \"$OJ_JOB_DIR/env-job-type.txt\"\nprintf '%s\\n' \"$OJ_JOB_DIR\" > \"$OJ_JOB_DIR/env-job-dir.txt\"\nprintf '%s\\n' \"$OJ_PROGRESS_PATH\" > \"$OJ_JOB_DIR/env-progress-path.txt\"\nprintf '%s\\n' \"$OJ_PYTHON_LOG_PATH\" > \"$OJ_JOB_DIR/env-python-log-path.txt\"\nexit 0\n",
    )
    .await;
    let state = test_state();

    let response = super::embedding::trigger_embedding(
        State(state.clone()),
        Json(TriggerEmbeddingRequest {
            source: "all".to_string(),
            rebuild: false,
            dry_run: false,
            batch_size: None,
            filter: None,
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    wait_for_embedding_job_terminal(&state).await;

    let job_id = {
        let lock = state.embedding_lock.lock().await;
        lock.as_ref().unwrap().job_id.clone()
    };
    let paths =
        crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();

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
    assert_eq!(env_job_type.trim(), JobType::Embedding.as_str());
    assert_eq!(env_job_dir.trim(), expected_job_dir.to_string_lossy());
    assert_eq!(
        env_progress_path.trim(),
        expected_progress_path.to_string_lossy()
    );
    assert_eq!(
        env_python_log_path.trim(),
        expected_python_log_path.to_string_lossy()
    );

    drop(fake_uv);
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[tokio::test]
async fn embedding_status_reports_queued_when_progress_file_is_missing() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let state = test_state();
    let job_id = uuid::Uuid::new_v4().to_string();
    let paths =
        crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

    *state.embedding_lock.lock().await = Some(EmbeddingJob {
        job_id: job_id.clone(),
        source: "all".to_string(),
        args: vec![
            "--source".to_string(),
            "all".to_string(),
            "--build".to_string(),
        ],
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    });

    let response = super::embedding::embedding_status(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["running"], serde_json::Value::Bool(true));
    assert_eq!(
        payload["progress"]["phase"],
        serde_json::Value::String("queued".to_string())
    );

    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[tokio::test]
async fn embedding_output_returns_empty_strings_when_stream_files_are_missing() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let state = test_state();
    let job_id = uuid::Uuid::new_v4().to_string();
    let job = EmbeddingJob {
        job_id: job_id.clone(),
        source: "all".to_string(),
        args: vec![
            "--source".to_string(),
            "all".to_string(),
            "--build".to_string(),
        ],
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    };
    let paths =
        crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    crate::utils::persist_job_metadata(&paths, crate::models::JobArtifactMetadata::from(&job))
        .await
        .unwrap();

    let response = super::embedding::embedding_output(State(state), Path(job_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["stdout"], serde_json::Value::String(String::new()));
    assert_eq!(payload["stderr"], serde_json::Value::String(String::new()));
    assert_eq!(
        payload["python_log"],
        serde_json::Value::String(String::new())
    );

    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[tokio::test]
async fn embedding_output_returns_not_found_after_artifacts_are_removed() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let state = test_state();
    let job_id = uuid::Uuid::new_v4().to_string();
    let paths =
        crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

    state
        .embedding_history
        .lock()
        .await
        .push_back(EmbeddingJob {
            job_id: job_id.clone(),
            source: "all".to_string(),
            args: vec![
                "--source".to_string(),
                "all".to_string(),
                "--build".to_string(),
            ],
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: Some("2026-03-14T00:01:00Z".to_string()),
            status: CrawlerStatus::Completed,
            stdout: Some("history stdout".to_string()),
            stderr: Some("history stderr".to_string()),
        });

    let response = super::embedding::embedding_output(State(state), Path(job_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[cfg(unix)]
#[tokio::test]
async fn trigger_embedding_writes_python_log_and_output_endpoint_includes_all_streams() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let fake_uv = FakeUvGuard::install(
        "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\npython3 - <<'PY'\nimport sys\nfrom utils.logger import get_core_logger\nlogger = get_core_logger()\nlogger.info('python logger ready')\nprint('stdout line')\nprint('stderr line', file=sys.stderr)\nPY\n",
    )
    .await;
    let state = test_state();

    let response = super::embedding::trigger_embedding(
        State(state.clone()),
        Json(TriggerEmbeddingRequest {
            source: "all".to_string(),
            rebuild: false,
            dry_run: false,
            batch_size: None,
            filter: None,
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    wait_for_embedding_job_terminal(&state).await;

    let job_id = {
        let lock = state.embedding_lock.lock().await;
        lock.as_ref().unwrap().job_id.clone()
    };
    let paths =
        crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
    let python_log = tokio::fs::read_to_string(&paths.python_log).await.unwrap();
    assert!(python_log.contains("python logger ready"));
    assert!(!python_log.contains('\u{1b}'));

    let output_response = super::embedding::embedding_output(State(state.clone()), Path(job_id))
        .await
        .into_response();
    assert_eq!(output_response.status(), StatusCode::OK);

    let body = to_bytes(output_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["stdout"],
        serde_json::Value::String("stdout line\n".to_string())
    );
    assert_eq!(
        payload["stderr"],
        serde_json::Value::String("stderr line\n".to_string())
    );
    assert_eq!(payload["python_log"], serde_json::Value::String(python_log));

    drop(fake_uv);
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[cfg(unix)]
#[tokio::test]
async fn trigger_embedding_preserves_summary_and_writes_terminal_metadata() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let fake_uv = FakeUvGuard::install(
        "#!/bin/sh\nif [ \"$1\" = \"run\" ] && [ \"$2\" = \"python3\" ]; then\n  shift 2\nfi\npython3 - <<'PY'\nimport json, os\nwith open(os.environ['OJ_PROGRESS_PATH'], 'w', encoding='utf-8') as fh:\n    json.dump({\n        'phase': 'embedding',\n        'rewrite_progress': {'done': 3, 'total': 4, 'skipped': 1},\n        'embed_progress': {'done': 2, 'total': 2},\n        'summary': {'succeeded': 2, 'skipped': {'rewrite_timeout': 1}, 'failed': {}, 'duration_secs': 1.2},\n        'started_at': '2026-03-14T00:00:00Z'\n    }, fh)\nPY\n",
    )
    .await;
    let state = test_state();

    let response = super::embedding::trigger_embedding(
        State(state.clone()),
        Json(TriggerEmbeddingRequest {
            source: "all".to_string(),
            rebuild: false,
            dry_run: false,
            batch_size: None,
            filter: None,
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    wait_for_embedding_job_terminal(&state).await;

    let job_id = {
        let lock = state.embedding_lock.lock().await;
        lock.as_ref().unwrap().job_id.clone()
    };
    let paths =
        crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
    let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
    let payload: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(
        payload["phase"],
        serde_json::Value::String("completed".to_string())
    );
    assert_eq!(
        payload["summary"]["succeeded"],
        serde_json::Value::Number(2.into())
    );
    assert_eq!(
        payload["metadata"]["job_id"],
        serde_json::Value::String(job_id)
    );
    assert!(payload["metadata"]["finished_at"].is_string());
    assert!(payload["updated_at"].is_string());

    drop(fake_uv);
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[cfg(unix)]
#[tokio::test]
async fn trigger_embedding_spawn_failure_persists_failed_progress() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let _path_lock = crate::utils::TEST_PATH_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let original_path = std::env::var_os("PATH");
    std::env::set_var("PATH", "");

    let state = test_state();
    let response = super::embedding::trigger_embedding(
        State(state.clone()),
        Json(TriggerEmbeddingRequest {
            source: "all".to_string(),
            rebuild: false,
            dry_run: false,
            batch_size: None,
            filter: None,
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    wait_for_embedding_job_terminal(&state).await;

    match original_path.as_ref() {
        Some(path) => std::env::set_var("PATH", path),
        None => std::env::remove_var("PATH"),
    }

    let job_id = {
        let lock = state.embedding_lock.lock().await;
        let job = lock.as_ref().unwrap();
        assert_eq!(job.status, CrawlerStatus::Failed);
        job.job_id.clone()
    };

    let response = super::embedding::embedding_progress(Path(job_id.clone()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["phase"],
        serde_json::Value::String("failed".to_string())
    );
    assert!(payload["metadata"]["finished_at"].is_string());

    let paths =
        crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[cfg(unix)]
#[tokio::test]
async fn trigger_embedding_fails_before_spawn_when_metadata_persist_fails() {
    let root_blocker = JobArtifactsRootBlocker::install().await;
    let spawned_marker = std::env::temp_dir().join(format!(
        "oj-api-rs-embedding-spawned-{}-{}",
        std::process::id(),
        rand::thread_rng().r#gen::<u64>()
    ));
    let _ = tokio::fs::remove_file(&spawned_marker).await;
    let fake_uv = FakeUvGuard::install(&format!(
        "#!/bin/sh\nprintf 'spawned\\n' > \"{}\"\nexit 0\n",
        spawned_marker.display()
    ))
    .await;
    let state = test_state();

    let response = super::embedding::trigger_embedding(
        State(state.clone()),
        Json(TriggerEmbeddingRequest {
            source: "all".to_string(),
            rebuild: false,
            dry_run: false,
            batch_size: None,
            filter: None,
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(*state.embedding_launch_guard.lock().await, None);
    assert!(state.active_embedding_pid.lock().await.is_none());

    let job = {
        let lock = state.embedding_lock.lock().await;
        lock.as_ref().cloned().unwrap()
    };
    assert_eq!(job.status, CrawlerStatus::Failed);
    assert!(job.finished_at.is_some());

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(tokio::fs::metadata(&spawned_marker).await.is_err());
    let history = state.embedding_history.lock().await;
    assert!(history
        .iter()
        .any(|entry| entry.job_id == job.job_id && entry.status == CrawlerStatus::Failed));

    drop(fake_uv);
    let _ = tokio::fs::remove_file(&spawned_marker).await;
    drop(root_blocker);
}

#[test]
fn embedding_finalize_ignores_reused_slot() {
    let original_job = EmbeddingJob {
        job_id: "old-job".to_string(),
        source: "all".to_string(),
        args: vec!["--source".to_string(), "all".to_string()],
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: Some("2026-03-14T00:01:00Z".to_string()),
        status: CrawlerStatus::Cancelled,
        stdout: None,
        stderr: None,
    };
    let mut reused_slot = Some(EmbeddingJob {
        job_id: "new-job".to_string(),
        source: "leetcode".to_string(),
        args: vec!["--source".to_string(), "leetcode".to_string()],
        started_at: "2026-03-14T00:02:00Z".to_string(),
        finished_at: None,
        status: CrawlerStatus::Running,
        stdout: None,
        stderr: None,
    });

    let finished = super::common::finalize_owned_embedding_job(
        &mut reused_slot,
        "old-job",
        None,
        Ok(CapturedOutput {
            stdout: b"old stdout".to_vec(),
            stderr: b"old stderr".to_vec(),
        }),
    );

    assert!(finished.is_none());
    let current = reused_slot.as_ref().unwrap();
    assert_eq!(current.job_id, "new-job");
    assert_eq!(current.status, CrawlerStatus::Running);
    assert!(current.finished_at.is_none());
    assert!(current.stdout.is_none());
    assert!(current.stderr.is_none());

    let mut owned_slot = Some(original_job.clone());
    let finished = super::common::finalize_owned_embedding_job(
        &mut owned_slot,
        "old-job",
        None,
        Ok(CapturedOutput {
            stdout: b"old stdout".to_vec(),
            stderr: b"old stderr".to_vec(),
        }),
    );

    let finished = finished.expect("owned embedding job should finalize");
    assert_eq!(finished.job_id, "old-job");
    assert_eq!(finished.status, CrawlerStatus::Cancelled);
    assert_eq!(finished.stdout.as_deref(), Some("old stdout"));
    assert_eq!(finished.stderr.as_deref(), Some("old stderr"));
}

#[tokio::test]
async fn embedding_progress_reports_unknown_for_malformed_progress_json() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let job_id = uuid::Uuid::new_v4().to_string();
    let paths =
        crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
    tokio::fs::create_dir_all(&paths.job_dir).await.unwrap();
    tokio::fs::write(&paths.progress, b"{not-json")
        .await
        .unwrap();

    let response = super::embedding::embedding_progress(Path(job_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["phase"],
        serde_json::Value::String("unknown".to_string())
    );

    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}

#[tokio::test]
async fn crawler_status_projects_manual_job_while_daily_fallback_coexists() {
    let state = test_state();
    let manual_key = manual_crawler_runtime_key().to_string();
    let daily_key = daily_fallback_crawler_runtime_key("com", "2026-03-14");
    let manual_job = crawler_job("manual-job", CrawlerTrigger::Admin, CrawlerStatus::Running);
    let daily_job = CrawlerJob {
        args: vec![
            "--daily".to_string(),
            "--domain".to_string(),
            "com".to_string(),
        ],
        ..crawler_job(
            "daily-job",
            CrawlerTrigger::DailyFallback,
            CrawlerStatus::Running,
        )
    };

    {
        let mut crawler_jobs = state.crawler_jobs.lock().await;
        crawler_jobs.insert(manual_key, manual_job);
        crawler_jobs.insert(daily_key.clone(), daily_job);
    }
    state.daily_fallback.lock().await.insert(
        daily_key,
        DailyFallbackEntry {
            job_id: "daily-job".to_string(),
            status: CrawlerStatus::Running,
            started_at: tokio::time::Instant::now(),
            cooldown_until: None,
            notify: Arc::new(Notify::new()),
            completed: Arc::new(AtomicBool::new(false)),
            stdout: None,
            stderr: None,
        },
    );

    let response = super::crawler::crawler_status(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["running"], serde_json::Value::Bool(true));
    assert_eq!(payload["running_jobs"].as_array().map(Vec::len), Some(2));
    assert_eq!(payload["running_jobs"][0]["job_id"], "daily-job");
    assert_eq!(payload["running_jobs"][0]["trigger"], "daily_fallback");
    assert_eq!(payload["running_jobs"][1]["job_id"], "manual-job");
    assert_eq!(payload["running_jobs"][1]["trigger"], "admin");
    assert!(payload["history"].is_array());
}

#[tokio::test]
async fn crawler_progress_returns_queued_for_running_job_without_progress_file() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let state = test_state();
    let runtime_key = manual_crawler_runtime_key().to_string();
    let job_id = uuid::Uuid::new_v4().to_string();
    let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

    state.crawler_jobs.lock().await.insert(
        runtime_key,
        CrawlerJob {
            job_id: job_id.clone(),
            ..crawler_job(&job_id, CrawlerTrigger::Admin, CrawlerStatus::Running)
        },
    );

    let response = super::crawler::crawler_progress(State(state), Path(job_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["phase"],
        serde_json::Value::String("queued".to_string())
    );
}

#[tokio::test]
async fn crawler_output_returns_not_found_after_artifacts_are_removed() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let state = test_state();
    let job_id = uuid::Uuid::new_v4().to_string();
    let paths = crate::utils::canonical_job_artifact_paths(JobType::Crawler, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

    state.crawler_history.lock().await.push_back(CrawlerJob {
        job_id: job_id.clone(),
        source: "leetcode".to_string(),
        args: vec![],
        trigger: CrawlerTrigger::Admin,
        started_at: "2026-03-14T00:00:00Z".to_string(),
        finished_at: Some("2026-03-14T00:01:00Z".to_string()),
        status: CrawlerStatus::Completed,
        stdout: Some("history stdout".to_string()),
        stderr: Some("history stderr".to_string()),
    });

    let response = super::crawler::crawler_output(State(state), Path(job_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn embedding_progress_returns_not_found_for_missing_non_running_job() {
    let _root_lock = crate::utils::TEST_JOB_ARTIFACTS_ROOT_MUTEX
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let job_id = uuid::Uuid::new_v4().to_string();
    let paths =
        crate::utils::canonical_job_artifact_paths(JobType::Embedding, &job_id).unwrap();
    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;

    let response = super::embedding::embedding_progress(Path(job_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let _ = tokio::fs::remove_dir_all(&paths.job_dir).await;
}
