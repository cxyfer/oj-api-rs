use std::collections::{HashMap, VecDeque};

use serde::Serialize;

use crate::models::{
    ActiveCrawlerPid, CrawlerJob, CrawlerStatus, EmbeddingJob, JobArtifactMetadata,
    JobArtifactPaths, JobType,
};

pub(super) async fn read_job_progress_json(
    job_type: JobType,
    job_id: &str,
) -> Option<serde_json::Value> {
    let paths = match crate::utils::canonical_job_artifact_paths(job_type, job_id) {
        Ok(paths) => paths,
        Err(_) => return None,
    };
    match tokio::fs::read_to_string(&paths.progress).await {
        Ok(content) => Some(
            serde_json::from_str(&content)
                .unwrap_or_else(|_| serde_json::json!({ "phase": "unknown" })),
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            match tokio::fs::metadata(&paths.job_dir).await {
                Ok(metadata) if metadata.is_dir() => Some(serde_json::json!({ "phase": "queued" })),
                Ok(_) => None,
                Err(meta_err) if meta_err.kind() == std::io::ErrorKind::NotFound => None,
                Err(_) => Some(serde_json::json!({ "phase": "unknown" })),
            }
        }
        Err(_) => Some(serde_json::json!({ "phase": "unknown" })),
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct CrawlerStatusResponse {
    pub(crate) running: bool,
    pub(crate) running_jobs: Vec<CrawlerJob>,
    pub(crate) history: Vec<CrawlerJob>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct EmbeddingStatusResponse {
    pub(crate) running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) current_job: Option<EmbeddingJob>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_job: Option<EmbeddingJob>,
    /// Embedding progress details (phase, counts, etc.).
    pub(crate) progress: Option<serde_json::Value>,
    pub(crate) history: Vec<EmbeddingJob>,
}

pub(super) trait TerminalJob {
    fn status(&self) -> &CrawlerStatus;
    fn status_mut(&mut self) -> &mut CrawlerStatus;
    fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>);
}

impl TerminalJob for CrawlerJob {
    fn status(&self) -> &CrawlerStatus {
        &self.status
    }

    fn status_mut(&mut self) -> &mut CrawlerStatus {
        &mut self.status
    }

    fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        CrawlerJob::set_output(self, stdout, stderr);
    }
}

impl TerminalJob for EmbeddingJob {
    fn status(&self) -> &CrawlerStatus {
        &self.status
    }

    fn status_mut(&mut self) -> &mut CrawlerStatus {
        &mut self.status
    }

    fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        EmbeddingJob::set_output(self, stdout, stderr);
    }
}

pub(super) fn apply_terminal_update<T: TerminalJob>(
    job: &mut T,
    terminal_status: Option<CrawlerStatus>,
    capture_result: std::io::Result<crate::utils::CapturedOutput>,
    job_kind: &str,
) {
    let already_terminal = crate::utils::crawler_status_is_terminal(job.status());

    match capture_result {
        Ok(output) => {
            if let Some(status) = terminal_status.filter(|_| !already_terminal) {
                *job.status_mut() = status;
            }
            job.set_output(output.stdout, output.stderr);
        }
        Err(err) => {
            tracing::error!("{} capture error: {}", job_kind, err);
            if already_terminal {
                return;
            }
            match terminal_status {
                Some(CrawlerStatus::TimedOut) => {
                    *job.status_mut() = CrawlerStatus::TimedOut;
                }
                Some(_) | None => {
                    *job.status_mut() = CrawlerStatus::Failed;
                }
            }
        }
    }
}

pub(super) fn inject_job_environment(
    cmd: &mut tokio::process::Command,
    job_id: &str,
    job_type: JobType,
    artifact_paths: &JobArtifactPaths,
) {
    crate::utils::inject_job_environment(cmd, job_id, job_type, artifact_paths);
}

pub(super) async fn persist_crawler_running_progress(
    artifact_paths: &JobArtifactPaths,
    job: &CrawlerJob,
) {
    crate::utils::persist_crawler_running_progress(artifact_paths, job).await;
}

pub(super) async fn persist_crawler_terminal_progress(
    artifact_paths: &JobArtifactPaths,
    job: &CrawlerJob,
) {
    crate::utils::persist_crawler_terminal_progress(artifact_paths, job).await;
}

fn embedding_phase_is_terminal(phase: &str) -> bool {
    matches!(phase, "completed" | "failed" | "cancelled" | "timed_out")
}

pub(super) async fn persist_embedding_terminal_progress(
    artifact_paths: &JobArtifactPaths,
    job: &EmbeddingJob,
) {
    if job.status == CrawlerStatus::Running {
        return;
    }

    let final_phase = match job.status {
        CrawlerStatus::Completed => "completed",
        CrawlerStatus::Failed => "failed",
        CrawlerStatus::Cancelled => "cancelled",
        CrawlerStatus::TimedOut => "timed_out",
        CrawlerStatus::Running => return,
    };
    let metadata = JobArtifactMetadata::from(job);
    let progress_path = artifact_paths.progress.clone();
    let result =
        crate::utils::update_json_atomic(&progress_path, |current: Option<serde_json::Value>| {
            let mut prog = current.unwrap_or_else(|| serde_json::json!({}));
            let current_phase = prog.get("phase").and_then(|value| value.as_str());
            if !current_phase.is_some_and(embedding_phase_is_terminal) {
                prog["phase"] = serde_json::json!(final_phase);
            }
            if prog.get("started_at").is_none() {
                prog["started_at"] = serde_json::json!(metadata.started_at.clone());
            }
            prog["updated_at"] = serde_json::json!(metadata.updated_at.clone());
            prog["metadata"] = serde_json::to_value(&metadata).unwrap_or(serde_json::Value::Null);
            Ok(prog)
        })
        .await;
    if let Err(err) = result {
        tracing::warn!("failed to persist final embedding progress: {}", err);
    }
}

pub(super) fn manual_trigger_conflicts(manual_guard: &Option<String>) -> bool {
    manual_guard.is_some()
}

pub(super) fn manual_crawler_launch_allowed(
    manual_guard: &Option<String>,
    job: Option<&CrawlerJob>,
    job_id: &str,
) -> bool {
    manual_guard.as_deref() == Some(job_id)
        && job
            .map(|job| job.job_id == job_id && job.status == CrawlerStatus::Running)
            .unwrap_or(false)
}

pub(super) fn clear_manual_guard_if_matches(manual_guard: &mut Option<String>, job_id: &str) {
    if manual_guard.as_deref() == Some(job_id) {
        *manual_guard = None;
    }
}

pub(super) fn with_owned_manual_crawler_job<T>(
    crawler_jobs: &mut HashMap<String, CrawlerJob>,
    job_id: &str,
    update: impl FnOnce(&mut CrawlerJob) -> T,
) -> Option<T> {
    let job = crawler_jobs.get_mut(crate::models::manual_crawler_runtime_key())?;
    if job.job_id != job_id {
        return None;
    }
    Some(update(job))
}

pub(super) fn finalize_owned_manual_crawler_job(
    crawler_jobs: &mut HashMap<String, CrawlerJob>,
    job_id: &str,
    terminal_status: Option<CrawlerStatus>,
    capture_result: std::io::Result<crate::utils::CapturedOutput>,
) -> Option<CrawlerJob> {
    with_owned_manual_crawler_job(crawler_jobs, job_id, |job| {
        if job.status == CrawlerStatus::Running && job.finished_at.is_none() {
            job.finished_at = Some(chrono::Utc::now().to_rfc3339());
        }
        apply_terminal_update(job, terminal_status, capture_result, "crawler");
        job.clone()
    })
}

pub(super) fn take_manual_crawler_pid(
    active_crawler_pids: &mut HashMap<String, ActiveCrawlerPid>,
) -> Option<u32> {
    active_crawler_pids
        .remove(crate::models::manual_crawler_runtime_key())
        .map(|active_pid| active_pid.pid)
}

pub(super) fn take_owned_manual_crawler_pid(
    active_crawler_pids: &mut HashMap<String, ActiveCrawlerPid>,
    job_id: &str,
) -> Option<u32> {
    let runtime_key = crate::models::manual_crawler_runtime_key();
    match active_crawler_pids.get(runtime_key) {
        Some(active_pid) if active_pid.job_id == job_id => {
            take_manual_crawler_pid(active_crawler_pids)
        }
        _ => None,
    }
}

pub(super) fn finalize_owned_embedding_job(
    embedding_slot: &mut Option<EmbeddingJob>,
    job_id: &str,
    terminal_status: Option<CrawlerStatus>,
    capture_result: std::io::Result<crate::utils::CapturedOutput>,
) -> Option<EmbeddingJob> {
    let job = embedding_slot.as_mut()?;
    if job.job_id != job_id {
        return None;
    }
    if job.status == CrawlerStatus::Running && job.finished_at.is_none() {
        job.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }
    apply_terminal_update(job, terminal_status, capture_result, "embedding");
    Some(job.clone())
}

pub(super) fn embedding_trigger_conflicts(launch_guard: &Option<String>) -> bool {
    launch_guard.is_some()
}

pub(super) fn embedding_launch_allowed(
    launch_guard: &Option<String>,
    job: Option<&EmbeddingJob>,
    job_id: &str,
) -> bool {
    launch_guard.as_deref() == Some(job_id)
        && job
            .map(|job| job.job_id == job_id && job.status == CrawlerStatus::Running)
            .unwrap_or(false)
}

pub(super) fn clear_embedding_launch_guard_if_matches(
    launch_guard: &mut Option<String>,
    job_id: &str,
) {
    if launch_guard.as_deref() == Some(job_id) {
        *launch_guard = None;
    }
}

pub(super) fn push_or_replace_crawler_history(history: &mut VecDeque<CrawlerJob>, job: CrawlerJob) {
    crate::utils::push_or_replace_crawler_history(history, job);
}

pub(super) fn push_or_replace_embedding_history(
    history: &mut VecDeque<EmbeddingJob>,
    job: EmbeddingJob,
) {
    if let Some(existing) = history
        .iter_mut()
        .find(|existing| existing.job_id == job.job_id)
    {
        *existing = job;
        return;
    }
    if history.len() >= crate::utils::RETAINED_JOB_HISTORY_LIMIT {
        history.pop_front();
    }
    history.push_back(job);
}
