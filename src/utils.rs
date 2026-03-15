use std::collections::{HashSet, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};

use crate::models::{
    CrawlerJob, CrawlerPhase, CrawlerProgress, CrawlerStatus, EmbeddingJob, EmbeddingProgress,
    JobArtifactMetadata, JobArtifactPaths, JobType,
};

#[cfg(test)]
pub(crate) static TEST_PATH_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[cfg(test)]
pub(crate) static TEST_JOB_ARTIFACTS_ROOT_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Spawns a process in its own process group.
///
/// This allows killing the entire process tree on timeout/cancel.
#[cfg(unix)]
pub fn spawn_with_pgid(mut cmd: Command) -> std::io::Result<Child> {
    unsafe {
        cmd.pre_exec(|| {
            if libc::setpgid(0, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    cmd.spawn()
}

#[cfg(not(unix))]
pub fn spawn_with_pgid(mut cmd: Command) -> std::io::Result<Child> {
    cmd.spawn()
}

/// Kills an entire process group by sending SIGKILL to the negative PID.
///
/// Returns `true` if the signal was sent, `false` if the pid was invalid or
/// the process group no longer exists (ESRCH).
#[cfg(unix)]
pub fn kill_pgid(pid: u32) -> bool {
    if pid <= 1 {
        tracing::warn!("refusing to kill pgid {pid}: unsafe target");
        return false;
    }
    let ret = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
    if ret == -1 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            tracing::debug!("pgid {pid} already exited");
        } else {
            tracing::warn!("kill_pgid({pid}) failed: {err}");
        }
        false
    } else {
        true
    }
}

#[cfg(not(unix))]
pub fn kill_pgid(_pid: u32) -> bool {
    false
}

pub fn canonical_job_artifact_paths(
    job_type: JobType,
    job_id: &str,
) -> Result<JobArtifactPaths, String> {
    JobArtifactPaths::new(job_type, job_id)
}

pub fn decode_lossy_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub fn absolutize_job_artifact_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

pub fn inject_job_environment(
    cmd: &mut Command,
    job_id: &str,
    job_type: JobType,
    artifact_paths: &JobArtifactPaths,
) {
    cmd.env("OJ_JOB_ID", job_id);
    cmd.env("OJ_JOB_TYPE", job_type.as_str());
    cmd.env(
        "OJ_JOB_DIR",
        absolutize_job_artifact_path(&artifact_paths.job_dir),
    );
    cmd.env(
        "OJ_PROGRESS_PATH",
        absolutize_job_artifact_path(&artifact_paths.progress),
    );
    cmd.env(
        "OJ_PYTHON_LOG_PATH",
        absolutize_job_artifact_path(&artifact_paths.python_log),
    );
}

pub fn crawler_status_is_terminal(status: &CrawlerStatus) -> bool {
    matches!(
        status,
        CrawlerStatus::Completed
            | CrawlerStatus::Failed
            | CrawlerStatus::Cancelled
            | CrawlerStatus::TimedOut
    )
}

fn crawler_phase_is_terminal(phase: &CrawlerPhase) -> bool {
    matches!(
        phase,
        CrawlerPhase::Completed
            | CrawlerPhase::Failed
            | CrawlerPhase::Cancelled
            | CrawlerPhase::TimedOut
    )
}

pub async fn persist_crawler_running_progress(artifact_paths: &JobArtifactPaths, job: &CrawlerJob) {
    if job.status != CrawlerStatus::Running {
        return;
    }

    let metadata = JobArtifactMetadata::from(job);
    let stored_progress = match tokio::fs::read_to_string(&artifact_paths.progress).await {
        Ok(content) => serde_json::from_str::<CrawlerProgress>(&content).ok(),
        Err(_) => None,
    };

    let progress = match stored_progress {
        Some(mut progress) => {
            if crawler_phase_is_terminal(&progress.phase) {
                return;
            }
            progress.phase = CrawlerPhase::Running;
            progress.updated_at = metadata.updated_at.clone();
            progress.metadata = Some(metadata.clone());
            progress
        }
        None => CrawlerProgress {
            phase: CrawlerPhase::Running,
            message: None,
            updated_at: metadata.updated_at.clone(),
            metadata: Some(metadata),
        },
    };

    if let Err(err) = write_crawler_progress(artifact_paths, &progress).await {
        tracing::warn!("failed to persist running crawler progress: {}", err);
    }
}

pub async fn persist_crawler_terminal_progress(
    artifact_paths: &JobArtifactPaths,
    job: &CrawlerJob,
) {
    if job.status == CrawlerStatus::Running {
        return;
    }

    let metadata = JobArtifactMetadata::from(job);
    let stored_progress = match tokio::fs::read_to_string(&artifact_paths.progress).await {
        Ok(content) => serde_json::from_str::<CrawlerProgress>(&content).ok(),
        Err(_) => None,
    };

    let progress = match stored_progress {
        Some(mut progress) => {
            if !crawler_phase_is_terminal(&progress.phase) {
                progress.phase = CrawlerPhase::from(&job.status);
            }
            progress.updated_at = metadata.updated_at.clone();
            progress.metadata = Some(metadata.clone());
            progress
        }
        None => CrawlerProgress {
            phase: CrawlerPhase::from(&job.status),
            message: None,
            updated_at: metadata.updated_at.clone(),
            metadata: Some(metadata),
        },
    };

    if let Err(err) = write_crawler_progress(artifact_paths, &progress).await {
        tracing::warn!("failed to persist final crawler progress: {}", err);
    }
}

pub fn push_or_replace_crawler_history(history: &mut VecDeque<CrawlerJob>, job: CrawlerJob) {
    if let Some(existing) = history
        .iter_mut()
        .find(|existing| existing.job_id == job.job_id)
    {
        *existing = job;
        return;
    }

    if history.len() >= RETAINED_JOB_HISTORY_LIMIT {
        history.pop_front();
    }
    history.push_back(job);
}

pub const OUTPUT_TAIL_MAX_BYTES: usize = 64 * 1024;

pub struct LiveOutputCapture {
    stdout_task: tokio::task::JoinHandle<io::Result<Vec<u8>>>,
    stderr_task: tokio::task::JoinHandle<io::Result<Vec<u8>>>,
}

pub struct CapturedOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobOutputSnapshot {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub python_log: Option<String>,
}

impl LiveOutputCapture {
    pub async fn finish(self) -> io::Result<CapturedOutput> {
        let stdout = self
            .stdout_task
            .await
            .map_err(|err| io::Error::other(err.to_string()))??;
        let stderr = self
            .stderr_task
            .await
            .map_err(|err| io::Error::other(err.to_string()))??;
        Ok(CapturedOutput { stdout, stderr })
    }
}

pub async fn ensure_job_artifact_dir(paths: &JobArtifactPaths) -> io::Result<()> {
    tokio::fs::create_dir_all(&paths.job_dir).await
}

pub async fn start_live_output_capture(
    child: &mut Child,
    paths: &JobArtifactPaths,
) -> io::Result<LiveOutputCapture> {
    ensure_job_artifact_dir(paths).await?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "child stdout not piped"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "child stderr not piped"))?;

    let stdout_path = paths.stdout.clone();
    let stderr_path = paths.stderr.clone();

    Ok(LiveOutputCapture {
        stdout_task: tokio::spawn(async move { tee_stream_to_file(stdout, stdout_path).await }),
        stderr_task: tokio::spawn(async move { tee_stream_to_file(stderr, stderr_path).await }),
    })
}

async fn tee_stream_to_file<R>(mut reader: R, path: PathBuf) -> io::Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut file = tokio::fs::File::create(&path).await?;
    let mut tail = Vec::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read]).await?;
        append_bounded_tail(&mut tail, &buffer[..read]);
    }

    file.flush().await?;
    Ok(tail)
}

fn append_bounded_tail(tail: &mut Vec<u8>, chunk: &[u8]) {
    if chunk.len() >= OUTPUT_TAIL_MAX_BYTES {
        tail.clear();
        tail.extend_from_slice(&chunk[chunk.len() - OUTPUT_TAIL_MAX_BYTES..]);
        return;
    }

    let overflow = tail
        .len()
        .saturating_add(chunk.len())
        .saturating_sub(OUTPUT_TAIL_MAX_BYTES);
    if overflow > 0 {
        tail.drain(..overflow);
    }
    tail.extend_from_slice(chunk);
}

async fn read_optional_text(path: &Path) -> io::Result<Option<String>> {
    match tokio::fs::read(path).await {
        Ok(content) => Ok(Some(decode_lossy_text(&content))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

pub async fn read_job_output(
    paths: &JobArtifactPaths,
    fallback_stdout: Option<&str>,
    fallback_stderr: Option<&str>,
) -> io::Result<Option<JobOutputSnapshot>> {
    let stdout = read_optional_text(&paths.stdout).await?;
    let stderr = read_optional_text(&paths.stderr).await?;
    let python_log = read_optional_text(&paths.python_log).await?;

    if stdout.is_some() || stderr.is_some() || python_log.is_some() {
        return Ok(Some(JobOutputSnapshot {
            stdout,
            stderr,
            python_log,
        }));
    }

    if fallback_stdout.is_some() || fallback_stderr.is_some() {
        return Ok(Some(JobOutputSnapshot {
            stdout: fallback_stdout.map(str::to_owned),
            stderr: fallback_stderr.map(str::to_owned),
            python_log: None,
        }));
    }

    match tokio::fs::metadata(&paths.job_dir).await {
        Ok(metadata) if metadata.is_dir() => Ok(Some(JobOutputSnapshot {
            stdout: None,
            stderr: None,
            python_log: None,
        })),
        Ok(_) => Ok(None),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

pub async fn write_json_atomic<T>(path: impl AsRef<Path>, value: &T) -> std::io::Result<()>
where
    T: Serialize,
{
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let payload = serde_json::to_vec_pretty(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let tmp = path.with_extension(format!(
        "{}.{}.tmp",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("json"),
        nonce
    ));
    tokio::fs::write(&tmp, payload).await?;
    tokio::fs::rename(&tmp, path).await
}

pub async fn write_crawler_progress(
    paths: &JobArtifactPaths,
    progress: &CrawlerProgress,
) -> std::io::Result<()> {
    write_json_atomic(&paths.progress, progress).await
}

pub async fn write_embedding_progress(
    paths: &JobArtifactPaths,
    progress: &EmbeddingProgress,
) -> std::io::Result<()> {
    write_json_atomic(&paths.progress, progress).await
}

pub async fn persist_job_metadata(
    paths: &JobArtifactPaths,
    metadata: JobArtifactMetadata,
) -> std::io::Result<()> {
    match paths.job_type {
        JobType::Crawler => {
            let progress = CrawlerProgress::queued(metadata);
            write_crawler_progress(paths, &progress).await
        }
        JobType::Embedding => {
            let progress = EmbeddingProgress {
                phase: "queued".to_string(),
                rewrite_progress: None,
                embed_progress: None,
                started_at: metadata.started_at.clone(),
                message: None,
                updated_at: metadata.updated_at.clone(),
                metadata: Some(metadata),
            };
            write_embedding_progress(paths, &progress).await
        }
    }
}

pub const RETAINED_JOB_HISTORY_LIMIT: usize = 50;
pub const JOB_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Debug, Default)]
pub struct RetainedJobState {
    pub crawler_history: VecDeque<CrawlerJob>,
    pub embedding_history: VecDeque<EmbeddingJob>,
}

pub async fn reconstruct_retained_job_state(
    root: impl AsRef<Path>,
) -> io::Result<RetainedJobState> {
    let root = root.as_ref();
    Ok(RetainedJobState {
        crawler_history: reconstruct_crawler_history(root).await?,
        embedding_history: reconstruct_embedding_history(root).await?,
    })
}

pub async fn reconcile_retained_job_state(
    root: impl AsRef<Path>,
    active_jobs: &HashSet<(JobType, String)>,
    crawler_history: &mut VecDeque<CrawlerJob>,
    embedding_history: &mut VecDeque<EmbeddingJob>,
) -> io::Result<()> {
    let root = root.as_ref();
    let retained = reconstruct_retained_job_state(root).await?;
    merge_crawler_history(crawler_history, retained.crawler_history);
    merge_embedding_history(embedding_history, retained.embedding_history);
    cleanup_expired_job_artifacts(root, active_jobs, crawler_history, embedding_history).await
}

pub async fn cleanup_expired_job_artifacts(
    root: impl AsRef<Path>,
    active_jobs: &HashSet<(JobType, String)>,
    crawler_history: &mut VecDeque<CrawlerJob>,
    embedding_history: &mut VecDeque<EmbeddingJob>,
) -> io::Result<()> {
    let root = root.as_ref();
    let now = SystemTime::now();

    cleanup_expired_job_dirs_for_type(root, JobType::Crawler, active_jobs, now).await?;
    cleanup_expired_job_dirs_for_type(root, JobType::Embedding, active_jobs, now).await?;

    let existing_crawler_ids = existing_job_ids(root, JobType::Crawler).await?;
    let existing_embedding_ids = existing_job_ids(root, JobType::Embedding).await?;

    crawler_history.retain(|job| existing_crawler_ids.contains(&job.job_id));
    embedding_history.retain(|job| existing_embedding_ids.contains(&job.job_id));
    trim_history(crawler_history);
    trim_history(embedding_history);

    Ok(())
}

async fn reconstruct_crawler_history(root: &Path) -> io::Result<VecDeque<CrawlerJob>> {
    let mut jobs = Vec::new();
    for paths in list_job_dirs(root, JobType::Crawler).await? {
        let Some(progress) = read_json_file::<CrawlerProgress>(&paths.progress).await? else {
            continue;
        };
        let output = read_job_output(&paths, None, None).await?;
        if let Some(job) = crawler_job_from_progress(&paths, progress, output) {
            jobs.push(job);
        }
    }
    jobs.sort_by(|a, b| {
        a.started_at
            .cmp(&b.started_at)
            .then_with(|| a.job_id.cmp(&b.job_id))
    });
    Ok(limit_history(jobs))
}

async fn reconstruct_embedding_history(root: &Path) -> io::Result<VecDeque<EmbeddingJob>> {
    let mut jobs = Vec::new();
    for paths in list_job_dirs(root, JobType::Embedding).await? {
        let Some(progress) = read_json_file::<EmbeddingProgress>(&paths.progress).await? else {
            continue;
        };
        let output = read_job_output(&paths, None, None).await?;
        if let Some(job) = embedding_job_from_progress(&paths, progress, output) {
            jobs.push(job);
        }
    }
    jobs.sort_by(|a, b| {
        a.started_at
            .cmp(&b.started_at)
            .then_with(|| a.job_id.cmp(&b.job_id))
    });
    Ok(limit_history(jobs))
}

async fn cleanup_expired_job_dirs_for_type(
    root: &Path,
    job_type: JobType,
    active_jobs: &HashSet<(JobType, String)>,
    now: SystemTime,
) -> io::Result<()> {
    for paths in list_job_dirs(root, job_type).await? {
        if active_jobs.contains(&(job_type, paths.job_id.clone())) {
            continue;
        }
        if !job_dir_is_terminal(&paths).await? || !job_dir_is_expired(&paths, now)? {
            continue;
        }
        match tokio::fs::remove_dir_all(&paths.job_dir).await {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

async fn existing_job_ids(root: &Path, job_type: JobType) -> io::Result<HashSet<String>> {
    let mut ids = HashSet::new();
    for paths in list_job_dirs(root, job_type).await? {
        ids.insert(paths.job_id);
    }
    Ok(ids)
}

async fn list_job_dirs(root: &Path, job_type: JobType) -> io::Result<Vec<JobArtifactPaths>> {
    let mut jobs = Vec::new();
    let type_root = root.join(job_type.as_str());
    let mut entries = match tokio::fs::read_dir(&type_root).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(jobs),
        Err(err) => return Err(err),
    };

    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_dir() {
            continue;
        }
        let Some(job_id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(paths) = JobArtifactPaths::with_root(root, job_type, &job_id) else {
            continue;
        };
        jobs.push(paths);
    }

    Ok(jobs)
}

async fn read_json_file<T>(path: &Path) -> io::Result<Option<T>>
where
    T: DeserializeOwned,
{
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };

    match serde_json::from_slice(&bytes) {
        Ok(value) => Ok(Some(value)),
        Err(err) => {
            tracing::warn!("failed to parse {}: {}", path.display(), err);
            Ok(None)
        }
    }
}

fn crawler_job_from_progress(
    paths: &JobArtifactPaths,
    progress: CrawlerProgress,
    output: Option<JobOutputSnapshot>,
) -> Option<CrawlerJob> {
    let metadata = progress.metadata?;
    if !metadata_matches_paths(&metadata, paths) {
        return None;
    }

    let trigger = metadata.trigger?;

    Some(CrawlerJob {
        job_id: metadata.job_id,
        source: metadata.source?,
        args: metadata.args,
        trigger,
        started_at: metadata.started_at?,
        finished_at: metadata.finished_at.or(progress.updated_at),
        status: crawler_status_from_phase(&progress.phase)?,
        stdout: output.as_ref().and_then(|snapshot| snapshot.stdout.clone()),
        stderr: output.and_then(|snapshot| snapshot.stderr),
    })
}

fn embedding_job_from_progress(
    paths: &JobArtifactPaths,
    progress: EmbeddingProgress,
    output: Option<JobOutputSnapshot>,
) -> Option<EmbeddingJob> {
    let metadata = progress.metadata?;
    if !metadata_matches_paths(&metadata, paths) {
        return None;
    }
    Some(EmbeddingJob {
        job_id: metadata.job_id,
        source: metadata.source?,
        args: metadata.args,
        started_at: metadata.started_at.or(progress.started_at)?,
        finished_at: metadata.finished_at.or(progress.updated_at),
        status: embedding_status_from_phase(&progress.phase)?,
        stdout: output.as_ref().and_then(|snapshot| snapshot.stdout.clone()),
        stderr: output.and_then(|snapshot| snapshot.stderr),
    })
}

fn metadata_matches_paths(metadata: &JobArtifactMetadata, paths: &JobArtifactPaths) -> bool {
    metadata.job_type == paths.job_type && metadata.job_id == paths.job_id
}

fn crawler_status_from_phase(phase: &CrawlerPhase) -> Option<CrawlerStatus> {
    match phase {
        CrawlerPhase::Completed => Some(CrawlerStatus::Completed),
        CrawlerPhase::Failed => Some(CrawlerStatus::Failed),
        CrawlerPhase::Cancelled => Some(CrawlerStatus::Cancelled),
        CrawlerPhase::TimedOut => Some(CrawlerStatus::TimedOut),
        CrawlerPhase::Queued | CrawlerPhase::Running => None,
    }
}

fn embedding_status_from_phase(phase: &str) -> Option<CrawlerStatus> {
    match phase {
        "completed" => Some(CrawlerStatus::Completed),
        "failed" => Some(CrawlerStatus::Failed),
        "cancelled" => Some(CrawlerStatus::Cancelled),
        "timed_out" => Some(CrawlerStatus::TimedOut),
        _ => None,
    }
}

async fn job_dir_is_terminal(paths: &JobArtifactPaths) -> io::Result<bool> {
    match paths.job_type {
        JobType::Crawler => Ok(read_json_file::<CrawlerProgress>(&paths.progress)
            .await?
            .and_then(|progress| crawler_status_from_phase(&progress.phase))
            .is_some()),
        JobType::Embedding => Ok(read_json_file::<EmbeddingProgress>(&paths.progress)
            .await?
            .and_then(|progress| embedding_status_from_phase(&progress.phase))
            .is_some()),
    }
}

fn job_dir_is_expired(paths: &JobArtifactPaths, now: SystemTime) -> io::Result<bool> {
    let Some(modified) = latest_job_artifact_mtime(paths)? else {
        return Ok(false);
    };
    Ok(match now.duration_since(modified) {
        Ok(age) => age >= JOB_RETENTION,
        Err(_) => false,
    })
}

fn latest_job_artifact_mtime(paths: &JobArtifactPaths) -> io::Result<Option<SystemTime>> {
    let mut latest = None;

    for path in [
        &paths.progress,
        &paths.stdout,
        &paths.stderr,
        &paths.python_log,
    ] {
        let modified = match std::fs::metadata(path) {
            Ok(metadata) => Some(metadata.modified()?),
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => return Err(err),
        };

        if let Some(modified) = modified {
            latest = Some(match latest {
                Some(current) if current > modified => current,
                _ => modified,
            });
        }
    }

    if latest.is_some() {
        return Ok(latest);
    }

    match std::fs::metadata(&paths.job_dir) {
        Ok(metadata) => Ok(Some(metadata.modified()?)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn merge_crawler_history(current: &mut VecDeque<CrawlerJob>, retained: VecDeque<CrawlerJob>) {
    merge_job_history(current, retained, |job| &job.job_id, |job| &job.started_at);
}

fn merge_embedding_history(current: &mut VecDeque<EmbeddingJob>, retained: VecDeque<EmbeddingJob>) {
    merge_job_history(current, retained, |job| &job.job_id, |job| &job.started_at);
}

fn merge_job_history<T, FJobId, FStartedAt>(
    current: &mut VecDeque<T>,
    retained: VecDeque<T>,
    job_id: FJobId,
    started_at: FStartedAt,
) where
    T: Clone,
    FJobId: Fn(&T) -> &str,
    FStartedAt: Fn(&T) -> &str,
{
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for job in current.iter().cloned() {
        seen.insert(job_id(&job).to_string());
        merged.push(job);
    }
    for job in retained {
        if seen.insert(job_id(&job).to_string()) {
            merged.push(job);
        }
    }

    merged.sort_by(|a, b| {
        started_at(a)
            .cmp(started_at(b))
            .then_with(|| job_id(a).cmp(job_id(b)))
    });
    *current = limit_history(merged);
}

fn limit_history<T>(jobs: Vec<T>) -> VecDeque<T> {
    let keep_from = jobs.len().saturating_sub(RETAINED_JOB_HISTORY_LIMIT);
    jobs.into_iter().skip(keep_from).collect()
}

fn trim_history<T>(history: &mut VecDeque<T>) {
    while history.len() > RETAINED_JOB_HISTORY_LIMIT {
        history.pop_front();
    }
}

pub fn natural_sort_key(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut buf = String::new();
    let mut in_digit: Option<bool> = None;

    for ch in s.chars() {
        let is_digit = ch.is_ascii_digit();
        match in_digit {
            None => {
                in_digit = Some(is_digit);
                buf.push(ch);
            }
            Some(d) if d == is_digit => buf.push(ch),
            Some(d) => {
                flush_segment(&mut out, &buf, d);
                buf.clear();
                in_digit = Some(is_digit);
                buf.push(ch);
            }
        }
    }
    if let Some(d) = in_digit {
        flush_segment(&mut out, &buf, d);
    }
    out
}

fn flush_segment(out: &mut String, buf: &str, is_digit: bool) {
    const PAD: usize = 20;
    if is_digit {
        let len = buf.len();
        if len < PAD {
            for _ in 0..PAD - len {
                out.push('0');
            }
        }
        out.push_str(buf);
    } else {
        out.push_str(&buf.to_ascii_lowercase());
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;
    use std::process::Stdio;
    use std::time::{Duration, SystemTime};

    use super::{
        canonical_job_artifact_paths, cleanup_expired_job_artifacts, decode_lossy_text,
        natural_sort_key, persist_crawler_running_progress, persist_crawler_terminal_progress,
        persist_job_metadata, read_job_output, reconstruct_retained_job_state,
        start_live_output_capture, write_crawler_progress, write_embedding_progress,
        OUTPUT_TAIL_MAX_BYTES,
    };
    use crate::models::{
        CrawlerJob, CrawlerPhase, CrawlerProgress, CrawlerStatus, CrawlerTrigger, EmbeddingJob,
        JobArtifactMetadata, JobArtifactPaths, JobType,
    };

    #[test]
    fn numeric_ordering() {
        assert!(natural_sort_key("P2000") < natural_sort_key("P10000"));
        assert!(natural_sort_key("P999") < natural_sort_key("P1000"));
    }

    #[test]
    fn pure_numeric() {
        assert!(natural_sort_key("999") < natural_sort_key("1000"));
        assert!(natural_sort_key("1") < natural_sort_key("10"));
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(natural_sort_key("P1"), natural_sort_key("p1"));
        assert_eq!(natural_sort_key("ABC"), natural_sort_key("abc"));
    }

    #[test]
    fn empty_and_null_equivalent() {
        assert_eq!(natural_sort_key(""), "");
    }

    #[test]
    fn multi_segment() {
        let k = natural_sort_key("abc001_a");
        assert_eq!(k, "abc00000000000000000001_a");
    }

    #[test]
    fn pure_alpha() {
        assert!(natural_sort_key("A") < natural_sort_key("B"));
        assert!(natural_sort_key("abc") < natural_sort_key("abd"));
    }

    #[test]
    fn numeric_prefix_ordering() {
        assert!(natural_sort_key("1000A") < natural_sort_key("1000B"));
        assert!(natural_sort_key("1A") < natural_sort_key("2A"));
        assert!(natural_sort_key("9A") < natural_sort_key("10A"));
    }

    #[test]
    fn canonical_paths_follow_job_type_layout() {
        let crawler = canonical_job_artifact_paths(JobType::Crawler, "job-1").unwrap();
        assert_eq!(crawler.job_dir, Path::new("scripts/logs/crawler/job-1"));
        assert_eq!(
            crawler.stdout,
            Path::new("scripts/logs/crawler/job-1/stdout.log")
        );
        assert_eq!(
            crawler.stderr,
            Path::new("scripts/logs/crawler/job-1/stderr.log")
        );
        assert_eq!(
            crawler.python_log,
            Path::new("scripts/logs/crawler/job-1/python.log")
        );
        assert_eq!(
            crawler.progress,
            Path::new("scripts/logs/crawler/job-1/progress.json")
        );

        let embedding = canonical_job_artifact_paths(JobType::Embedding, "job-2").unwrap();
        assert_eq!(embedding.job_dir, Path::new("scripts/logs/embedding/job-2"));
        assert_eq!(
            embedding.stdout,
            Path::new("scripts/logs/embedding/job-2/stdout.log")
        );
        assert_eq!(
            embedding.stderr,
            Path::new("scripts/logs/embedding/job-2/stderr.log")
        );
        assert_eq!(
            embedding.python_log,
            Path::new("scripts/logs/embedding/job-2/python.log")
        );
        assert_eq!(
            embedding.progress,
            Path::new("scripts/logs/embedding/job-2/progress.json")
        );
    }

    #[test]
    fn lossy_decode_handles_invalid_utf8() {
        let decoded = decode_lossy_text(&[0x66, 0x6f, 0x80, 0x6f]);
        assert_eq!(decoded, "fo�o");
    }

    #[test]
    fn canonical_paths_reject_unsafe_job_ids() {
        assert!(canonical_job_artifact_paths(JobType::Crawler, "../escape").is_err());
        assert!(canonical_job_artifact_paths(JobType::Crawler, "nested/job").is_err());
        assert!(canonical_job_artifact_paths(JobType::Crawler, "").is_err());
    }

    #[test]
    fn metadata_round_trip_for_crawler_job() {
        let job = CrawlerJob {
            job_id: "crawler-job".to_string(),
            source: "leetcode".to_string(),
            args: vec!["--daily".to_string()],
            trigger: CrawlerTrigger::DailyFallback,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: Some("2026-03-14T00:01:00Z".to_string()),
            status: CrawlerStatus::Completed,
            stdout: None,
            stderr: None,
        };
        let metadata = JobArtifactMetadata::from(&job);
        assert_eq!(metadata.job_type, JobType::Crawler);
        assert_eq!(metadata.source.as_deref(), Some("leetcode"));
        assert_eq!(metadata.args, vec!["--daily"]);
        assert_eq!(metadata.trigger, Some(CrawlerTrigger::DailyFallback));
        assert_eq!(metadata.started_at.as_deref(), Some("2026-03-14T00:00:00Z"));
        assert_eq!(
            metadata.finished_at.as_deref(),
            Some("2026-03-14T00:01:00Z")
        );
    }

    #[test]
    fn metadata_round_trip_for_embedding_job() {
        let job = EmbeddingJob {
            job_id: "embedding-job".to_string(),
            source: "leetcode".to_string(),
            args: vec!["--source".to_string(), "leetcode".to_string()],
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };
        let metadata = JobArtifactMetadata::from(&job);
        assert_eq!(metadata.job_type, JobType::Embedding);
        assert_eq!(metadata.source.as_deref(), Some("leetcode"));
        assert_eq!(metadata.trigger, None);
    }

    #[test]
    fn crawler_progress_preserves_terminal_phase() {
        let progress = CrawlerProgress {
            phase: CrawlerPhase::Completed,
            message: Some("done".to_string()),
            updated_at: Some("2026-03-14T00:01:00Z".to_string()),
            metadata: None,
        };
        let value = serde_json::to_value(&progress).unwrap();
        assert_eq!(value["phase"], "completed");
        let restored: CrawlerProgress = serde_json::from_value(value).unwrap();
        assert_eq!(restored.phase, CrawlerPhase::Completed);
    }

    #[tokio::test]
    async fn persist_job_metadata_writes_queued_progress() {
        let root =
            std::env::temp_dir().join(format!("oj-api-rs-utils-test-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&root).await;
        let paths =
            crate::models::JobArtifactPaths::with_root(&root, JobType::Crawler, "job-queued")
                .unwrap();
        let metadata = JobArtifactMetadata {
            job_id: "job-queued".to_string(),
            job_type: JobType::Crawler,
            source: Some("leetcode".to_string()),
            args: vec!["--daily".to_string()],
            trigger: Some(CrawlerTrigger::Admin),
            started_at: Some("2026-03-14T00:00:00Z".to_string()),
            finished_at: None,
            updated_at: Some("2026-03-14T00:00:00Z".to_string()),
        };

        persist_job_metadata(&paths, metadata).await.unwrap();

        let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let stored: CrawlerProgress = serde_json::from_str(&content).unwrap();
        assert_eq!(stored.phase, CrawlerPhase::Queued);
        assert_eq!(stored.metadata.unwrap().job_id, "job-queued");

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn persist_embedding_metadata_writes_queued_progress() {
        let root =
            std::env::temp_dir().join(format!("oj-api-rs-embedding-test-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&root).await;
        let paths =
            crate::models::JobArtifactPaths::with_root(&root, JobType::Embedding, "job-embed")
                .unwrap();
        let metadata = JobArtifactMetadata {
            job_id: "job-embed".to_string(),
            job_type: JobType::Embedding,
            source: Some("leetcode".to_string()),
            args: vec!["--source".to_string(), "leetcode".to_string()],
            trigger: None,
            started_at: Some("2026-03-14T00:00:00Z".to_string()),
            finished_at: None,
            updated_at: Some("2026-03-14T00:00:00Z".to_string()),
        };

        persist_job_metadata(&paths, metadata).await.unwrap();

        let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let stored: crate::models::EmbeddingProgress = serde_json::from_str(&content).unwrap();
        assert_eq!(stored.phase, "queued");
        assert_eq!(stored.metadata.unwrap().job_id, "job-embed");

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn persist_crawler_running_progress_advances_queued_phase() {
        let paths = temp_job_paths(JobType::Crawler, "running-progress");
        tokio::fs::create_dir_all(&paths.job_dir).await.unwrap();
        write_crawler_progress(
            &paths,
            &CrawlerProgress {
                phase: CrawlerPhase::Queued,
                message: Some("accepted".to_string()),
                updated_at: Some("2026-03-14T00:00:00Z".to_string()),
                metadata: Some(JobArtifactMetadata {
                    job_id: "running-progress".to_string(),
                    job_type: JobType::Crawler,
                    source: Some("leetcode".to_string()),
                    args: vec!["--daily".to_string()],
                    trigger: Some(CrawlerTrigger::Admin),
                    started_at: Some("2026-03-14T00:00:00Z".to_string()),
                    finished_at: None,
                    updated_at: Some("2026-03-14T00:00:00Z".to_string()),
                }),
            },
        )
        .await
        .unwrap();

        let job = CrawlerJob {
            job_id: "running-progress".to_string(),
            source: "leetcode".to_string(),
            args: vec!["--daily".to_string()],
            trigger: CrawlerTrigger::Admin,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };

        persist_crawler_running_progress(&paths, &job).await;

        let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let stored: CrawlerProgress = serde_json::from_str(&content).unwrap();
        assert_eq!(stored.phase, CrawlerPhase::Running);
        assert_eq!(stored.message.as_deref(), Some("accepted"));
        assert_eq!(stored.updated_at.as_deref(), Some("2026-03-14T00:00:00Z"));
        assert_eq!(stored.metadata.unwrap().finished_at, None);

        let _ = tokio::fs::remove_dir_all(&paths.root_dir).await;
    }

    #[tokio::test]
    async fn persist_crawler_running_progress_preserves_existing_terminal_phase() {
        let paths = temp_job_paths(JobType::Crawler, "running-terminal");
        tokio::fs::create_dir_all(&paths.job_dir).await.unwrap();
        write_crawler_progress(
            &paths,
            &CrawlerProgress {
                phase: CrawlerPhase::Completed,
                message: Some("done".to_string()),
                updated_at: Some("2026-03-14T00:01:00Z".to_string()),
                metadata: Some(JobArtifactMetadata {
                    job_id: "running-terminal".to_string(),
                    job_type: JobType::Crawler,
                    source: Some("leetcode".to_string()),
                    args: vec!["--daily".to_string()],
                    trigger: Some(CrawlerTrigger::Admin),
                    started_at: Some("2026-03-14T00:00:00Z".to_string()),
                    finished_at: Some("2026-03-14T00:01:00Z".to_string()),
                    updated_at: Some("2026-03-14T00:01:00Z".to_string()),
                }),
            },
        )
        .await
        .unwrap();

        let job = CrawlerJob {
            job_id: "running-terminal".to_string(),
            source: "leetcode".to_string(),
            args: vec!["--daily".to_string()],
            trigger: CrawlerTrigger::DailyFallback,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: None,
            status: CrawlerStatus::Running,
            stdout: None,
            stderr: None,
        };

        persist_crawler_running_progress(&paths, &job).await;

        let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let stored: CrawlerProgress = serde_json::from_str(&content).unwrap();
        assert_eq!(stored.phase, CrawlerPhase::Completed);
        assert_eq!(stored.message.as_deref(), Some("done"));
        assert_eq!(stored.updated_at.as_deref(), Some("2026-03-14T00:01:00Z"));

        let metadata = stored.metadata.unwrap();
        assert_eq!(metadata.trigger, Some(CrawlerTrigger::Admin));
        assert_eq!(
            metadata.finished_at.as_deref(),
            Some("2026-03-14T00:01:00Z")
        );

        let _ = tokio::fs::remove_dir_all(&paths.root_dir).await;
    }

    #[tokio::test]
    async fn persist_crawler_terminal_progress_refreshes_terminal_metadata() {
        let paths = temp_job_paths(JobType::Crawler, "terminal-refresh");
        tokio::fs::create_dir_all(&paths.job_dir).await.unwrap();
        write_crawler_progress(
            &paths,
            &CrawlerProgress {
                phase: CrawlerPhase::Completed,
                message: Some("done".to_string()),
                updated_at: Some("2026-03-14T00:01:00Z".to_string()),
                metadata: Some(JobArtifactMetadata {
                    job_id: "terminal-refresh".to_string(),
                    job_type: JobType::Crawler,
                    source: Some("leetcode".to_string()),
                    args: vec!["--daily".to_string()],
                    trigger: Some(CrawlerTrigger::Admin),
                    started_at: Some("2026-03-14T00:00:00Z".to_string()),
                    finished_at: Some("2026-03-14T00:01:00Z".to_string()),
                    updated_at: Some("2026-03-14T00:01:00Z".to_string()),
                }),
            },
        )
        .await
        .unwrap();

        let job = CrawlerJob {
            job_id: "terminal-refresh".to_string(),
            source: "leetcode".to_string(),
            args: vec!["--daily".to_string()],
            trigger: CrawlerTrigger::DailyFallback,
            started_at: "2026-03-14T00:00:00Z".to_string(),
            finished_at: Some("2026-03-14T00:05:00Z".to_string()),
            status: CrawlerStatus::Cancelled,
            stdout: None,
            stderr: None,
        };

        persist_crawler_terminal_progress(&paths, &job).await;

        let content = tokio::fs::read_to_string(&paths.progress).await.unwrap();
        let stored: CrawlerProgress = serde_json::from_str(&content).unwrap();
        assert_eq!(stored.phase, CrawlerPhase::Completed);
        assert_eq!(stored.updated_at.as_deref(), Some("2026-03-14T00:05:00Z"));

        let metadata = stored.metadata.unwrap();
        assert_eq!(metadata.trigger, Some(CrawlerTrigger::DailyFallback));
        assert_eq!(
            metadata.finished_at.as_deref(),
            Some("2026-03-14T00:05:00Z")
        );
        assert_eq!(metadata.updated_at.as_deref(), Some("2026-03-14T00:05:00Z"));

        let _ = tokio::fs::remove_dir_all(&paths.root_dir).await;
    }

    fn temp_job_paths(job_type: JobType, job_id: &str) -> JobArtifactPaths {
        let root = std::env::temp_dir().join(format!(
            "oj-api-rs-live-output-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        JobArtifactPaths::with_root(root, job_type, job_id).unwrap()
    }

    #[tokio::test]
    async fn live_output_capture_appends_logs_during_execution() {
        let paths = temp_job_paths(JobType::Crawler, "live-capture");
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c");
        cmd.arg("printf 'out-1'; sleep 0.3; printf 'err-1' >&2; sleep 0.3; printf 'out-2'");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().unwrap();
        let capture = start_live_output_capture(&mut child, &paths).await.unwrap();

        tokio::time::sleep(Duration::from_millis(150)).await;
        let stdout_during = tokio::fs::read_to_string(&paths.stdout).await.unwrap();
        let stderr_during = tokio::fs::read_to_string(&paths.stderr).await.unwrap();
        assert_eq!(stdout_during, "out-1");
        assert_eq!(stderr_during, "");
        assert!(!paths.root_dir.join("live-capture.stdout.log").exists());
        assert!(!paths.root_dir.join("live-capture.stderr.log").exists());

        tokio::time::sleep(Duration::from_millis(350)).await;
        let stdout_mid = tokio::fs::read_to_string(&paths.stdout).await.unwrap();
        let stderr_mid = tokio::fs::read_to_string(&paths.stderr).await.unwrap();
        assert_eq!(stdout_mid, "out-1");
        assert_eq!(stderr_mid, "err-1");

        let status = child.wait().await.unwrap();
        assert!(status.success());
        let output = capture.finish().await.unwrap();
        assert_eq!(output.stdout, b"out-1out-2");
        assert_eq!(output.stderr, b"err-1");

        let stdout_final = tokio::fs::read_to_string(&paths.stdout).await.unwrap();
        let stderr_final = tokio::fs::read_to_string(&paths.stderr).await.unwrap();
        assert_eq!(stdout_final, "out-1out-2");
        assert_eq!(stderr_final, "err-1");

        let _ = tokio::fs::remove_dir_all(&paths.root_dir).await;
    }

    #[tokio::test]
    async fn live_output_capture_keeps_only_bounded_tail_in_memory() {
        let paths = temp_job_paths(JobType::Embedding, "bounded-tail");
        let stdout_len = OUTPUT_TAIL_MAX_BYTES + 321;
        let stderr_len = OUTPUT_TAIL_MAX_BYTES + 123;
        let mut cmd = tokio::process::Command::new("python3");
        cmd.arg("-c");
        cmd.arg(format!(
            "import sys; sys.stdout.write('A' * {stdout_len}); sys.stderr.write('B' * {stderr_len})"
        ));
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().unwrap();
        let capture = start_live_output_capture(&mut child, &paths).await.unwrap();

        let status = child.wait().await.unwrap();
        assert!(status.success());
        let output = capture.finish().await.unwrap();
        assert_eq!(output.stdout.len(), OUTPUT_TAIL_MAX_BYTES);
        assert_eq!(output.stderr.len(), OUTPUT_TAIL_MAX_BYTES);
        assert!(output.stdout.iter().all(|byte| *byte == b'A'));
        assert!(output.stderr.iter().all(|byte| *byte == b'B'));

        let stdout_meta = tokio::fs::metadata(&paths.stdout).await.unwrap();
        let stderr_meta = tokio::fs::metadata(&paths.stderr).await.unwrap();
        assert_eq!(stdout_meta.len(), stdout_len as u64);
        assert_eq!(stderr_meta.len(), stderr_len as u64);

        let _ = tokio::fs::remove_dir_all(&paths.root_dir).await;
    }

    #[tokio::test]
    async fn read_job_output_prefers_canonical_files_over_history_tail() {
        let paths = temp_job_paths(JobType::Crawler, "prefer-files");
        tokio::fs::create_dir_all(&paths.job_dir).await.unwrap();
        tokio::fs::write(&paths.stdout, "full stdout from file")
            .await
            .unwrap();
        tokio::fs::write(&paths.stderr, "full stderr from file")
            .await
            .unwrap();

        let output = read_job_output(&paths, Some("tail stdout"), Some("tail stderr"))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(output.stdout.as_deref(), Some("full stdout from file"));
        assert_eq!(output.stderr.as_deref(), Some("full stderr from file"));

        let _ = tokio::fs::remove_dir_all(&paths.root_dir).await;
    }

    #[tokio::test]
    async fn read_job_output_falls_back_to_history_when_files_missing() {
        let paths = temp_job_paths(JobType::Embedding, "fallback-history");

        let output = read_job_output(&paths, Some("tail stdout"), Some("tail stderr"))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(output.stdout.as_deref(), Some("tail stdout"));
        assert_eq!(output.stderr.as_deref(), Some("tail stderr"));

        let _ = tokio::fs::remove_dir_all(&paths.root_dir).await;
    }

    #[tokio::test]
    async fn read_job_output_decodes_invalid_utf8_lossily() {
        let paths = temp_job_paths(JobType::Crawler, "lossy-output");
        tokio::fs::create_dir_all(&paths.job_dir).await.unwrap();
        tokio::fs::write(&paths.stdout, [0x66, 0x6f, 0x80, 0x6f])
            .await
            .unwrap();

        let output = read_job_output(&paths, None, None).await.unwrap().unwrap();

        assert_eq!(output.stdout.as_deref(), Some("fo�o"));
        assert_eq!(output.stderr, None);

        let _ = tokio::fs::remove_dir_all(&paths.root_dir).await;
    }

    fn temp_history_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "oj-api-rs-history-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ))
    }

    async fn write_crawler_job_fixture(
        root: &std::path::Path,
        job_id: &str,
        phase: CrawlerPhase,
        started_at: &str,
        finished_at: Option<&str>,
        trigger: Option<CrawlerTrigger>,
    ) -> JobArtifactPaths {
        let paths = JobArtifactPaths::with_root(root, JobType::Crawler, job_id).unwrap();
        tokio::fs::create_dir_all(&paths.job_dir).await.unwrap();
        let metadata = trigger.map(|trigger| JobArtifactMetadata {
            job_id: job_id.to_string(),
            job_type: JobType::Crawler,
            source: Some("leetcode".to_string()),
            args: vec!["--daily".to_string()],
            trigger: Some(trigger),
            started_at: Some(started_at.to_string()),
            finished_at: finished_at.map(str::to_string),
            updated_at: finished_at
                .map(str::to_string)
                .or_else(|| Some(started_at.to_string())),
        });
        write_crawler_progress(
            &paths,
            &CrawlerProgress {
                phase,
                message: None,
                updated_at: finished_at
                    .map(str::to_string)
                    .or_else(|| Some(started_at.to_string())),
                metadata,
            },
        )
        .await
        .unwrap();
        tokio::fs::write(&paths.stdout, format!("stdout-{job_id}"))
            .await
            .unwrap();
        tokio::fs::write(&paths.stderr, format!("stderr-{job_id}"))
            .await
            .unwrap();
        paths
    }

    async fn write_embedding_job_fixture(
        root: &std::path::Path,
        job_id: &str,
        phase: &str,
        started_at: &str,
        finished_at: Option<&str>,
        with_metadata: bool,
    ) -> JobArtifactPaths {
        let paths = JobArtifactPaths::with_root(root, JobType::Embedding, job_id).unwrap();
        tokio::fs::create_dir_all(&paths.job_dir).await.unwrap();
        let metadata = with_metadata.then(|| JobArtifactMetadata {
            job_id: job_id.to_string(),
            job_type: JobType::Embedding,
            source: Some("all".to_string()),
            args: vec!["--source".to_string(), "all".to_string()],
            trigger: None,
            started_at: Some(started_at.to_string()),
            finished_at: finished_at.map(str::to_string),
            updated_at: finished_at
                .map(str::to_string)
                .or_else(|| Some(started_at.to_string())),
        });
        write_embedding_progress(
            &paths,
            &crate::models::EmbeddingProgress {
                phase: phase.to_string(),
                rewrite_progress: None,
                embed_progress: None,
                started_at: Some(started_at.to_string()),
                message: None,
                updated_at: finished_at
                    .map(str::to_string)
                    .or_else(|| Some(started_at.to_string())),
                metadata,
            },
        )
        .await
        .unwrap();
        tokio::fs::write(&paths.stdout, format!("stdout-{job_id}"))
            .await
            .unwrap();
        tokio::fs::write(&paths.stderr, format!("stderr-{job_id}"))
            .await
            .unwrap();
        paths
    }

    #[cfg(unix)]
    fn set_path_mtime(path: &std::path::Path, time: SystemTime) {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        use std::time::UNIX_EPOCH;

        let ts = time.duration_since(UNIX_EPOCH).unwrap();
        let spec = libc::timespec {
            tv_sec: ts.as_secs() as libc::time_t,
            tv_nsec: ts.subsec_nanos() as libc::c_long,
        };
        let specs = [spec, spec];
        let c_path = CString::new(path.as_os_str().as_bytes()).unwrap();
        let rc = unsafe { libc::utimensat(libc::AT_FDCWD, c_path.as_ptr(), specs.as_ptr(), 0) };
        assert_eq!(rc, 0, "failed to set mtime for {}", path.display());
    }

    #[cfg(unix)]
    fn set_job_artifact_mtime(paths: &JobArtifactPaths, time: SystemTime) {
        set_path_mtime(&paths.progress, time);
        set_path_mtime(&paths.stdout, time);
        set_path_mtime(&paths.stderr, time);
        if paths.python_log.exists() {
            set_path_mtime(&paths.python_log, time);
        }
    }

    #[tokio::test]
    async fn reconstruct_retained_job_state_restores_terminal_jobs_in_order() {
        let root = temp_history_root("reconstruct");

        write_crawler_job_fixture(
            &root,
            "crawler-old",
            CrawlerPhase::Completed,
            "2026-03-01T00:00:00Z",
            Some("2026-03-01T00:05:00Z"),
            Some(CrawlerTrigger::Admin),
        )
        .await;
        write_crawler_job_fixture(
            &root,
            "crawler-skip-running",
            CrawlerPhase::Running,
            "2026-03-01T00:10:00Z",
            None,
            Some(CrawlerTrigger::Admin),
        )
        .await;
        write_crawler_job_fixture(
            &root,
            "crawler-daily-fallback",
            CrawlerPhase::Completed,
            "2026-03-01T00:15:00Z",
            Some("2026-03-01T00:16:00Z"),
            Some(CrawlerTrigger::DailyFallback),
        )
        .await;
        write_crawler_job_fixture(
            &root,
            "crawler-new",
            CrawlerPhase::Failed,
            "2026-03-01T00:20:00Z",
            Some("2026-03-01T00:25:00Z"),
            Some(CrawlerTrigger::Admin),
        )
        .await;
        write_embedding_job_fixture(
            &root,
            "embedding-terminal",
            "timed_out",
            "2026-03-01T01:00:00Z",
            Some("2026-03-01T01:10:00Z"),
            true,
        )
        .await;
        write_embedding_job_fixture(
            &root,
            "embedding-missing-metadata",
            "completed",
            "2026-03-01T02:00:00Z",
            Some("2026-03-01T02:10:00Z"),
            false,
        )
        .await;

        let retained = reconstruct_retained_job_state(&root).await.unwrap();

        let crawler_ids: Vec<_> = retained
            .crawler_history
            .iter()
            .map(|job| job.job_id.as_str())
            .collect();
        assert_eq!(
            crawler_ids,
            vec!["crawler-old", "crawler-daily-fallback", "crawler-new"]
        );
        assert_eq!(retained.crawler_history[0].status, CrawlerStatus::Completed);
        assert_eq!(retained.crawler_history[1].status, CrawlerStatus::Completed);
        assert_eq!(retained.crawler_history[2].status, CrawlerStatus::Failed);
        assert_eq!(
            retained.crawler_history[1].trigger,
            CrawlerTrigger::DailyFallback
        );
        assert_eq!(
            retained.crawler_history[0].stdout.as_deref(),
            Some("stdout-crawler-old")
        );
        assert_eq!(
            retained.crawler_history[2].stderr.as_deref(),
            Some("stderr-crawler-new")
        );

        let embedding_ids: Vec<_> = retained
            .embedding_history
            .iter()
            .map(|job| job.job_id.as_str())
            .collect();
        assert_eq!(embedding_ids, vec!["embedding-terminal"]);
        assert_eq!(
            retained.embedding_history[0].status,
            CrawlerStatus::TimedOut
        );

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn cleanup_expired_job_artifacts_deletes_old_terminal_dirs_and_updates_history() {
        let root = temp_history_root("cleanup-expired");
        let crawler_paths = write_crawler_job_fixture(
            &root,
            "crawler-expired",
            CrawlerPhase::Completed,
            "2026-03-01T00:00:00Z",
            Some("2026-03-01T00:05:00Z"),
            Some(CrawlerTrigger::Admin),
        )
        .await;
        let embedding_paths = write_embedding_job_fixture(
            &root,
            "embedding-expired",
            "completed",
            "2026-03-01T01:00:00Z",
            Some("2026-03-01T01:05:00Z"),
            true,
        )
        .await;
        let root_date_log = root.join("2026-03-14.log");
        let legacy_flat = root.join("legacy-job.stdout.log");
        tokio::fs::write(&root_date_log, "ansi root log")
            .await
            .unwrap();
        tokio::fs::write(&legacy_flat, "legacy flat log")
            .await
            .unwrap();

        #[cfg(unix)]
        {
            let expired_at = SystemTime::now() - Duration::from_secs(8 * 24 * 60 * 60);
            set_job_artifact_mtime(&crawler_paths, expired_at);
            set_job_artifact_mtime(&embedding_paths, expired_at);
        }

        let mut retained = reconstruct_retained_job_state(&root).await.unwrap();
        cleanup_expired_job_artifacts(
            &root,
            &HashSet::new(),
            &mut retained.crawler_history,
            &mut retained.embedding_history,
        )
        .await
        .unwrap();

        assert!(!crawler_paths.job_dir.exists());
        assert!(!embedding_paths.job_dir.exists());
        assert!(root_date_log.exists());
        assert!(legacy_flat.exists());
        assert!(retained.crawler_history.is_empty());
        assert!(retained.embedding_history.is_empty());

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn cleanup_expired_job_artifacts_skips_active_and_non_terminal_dirs() {
        let root = temp_history_root("cleanup-skip");
        let active_paths = write_crawler_job_fixture(
            &root,
            "crawler-active",
            CrawlerPhase::Completed,
            "2026-03-01T00:00:00Z",
            Some("2026-03-01T00:05:00Z"),
            Some(CrawlerTrigger::Admin),
        )
        .await;
        let running_paths = write_embedding_job_fixture(
            &root,
            "embedding-running",
            "running",
            "2026-03-01T01:00:00Z",
            None,
            true,
        )
        .await;

        #[cfg(unix)]
        {
            let expired_at = SystemTime::now() - Duration::from_secs(8 * 24 * 60 * 60);
            set_job_artifact_mtime(&active_paths, expired_at);
            set_job_artifact_mtime(&running_paths, expired_at);
        }

        let mut retained = reconstruct_retained_job_state(&root).await.unwrap();
        let mut active_jobs = HashSet::new();
        active_jobs.insert((JobType::Crawler, "crawler-active".to_string()));

        cleanup_expired_job_artifacts(
            &root,
            &active_jobs,
            &mut retained.crawler_history,
            &mut retained.embedding_history,
        )
        .await
        .unwrap();

        assert!(active_paths.job_dir.exists());
        assert!(running_paths.job_dir.exists());
        let crawler_ids: Vec<_> = retained
            .crawler_history
            .iter()
            .map(|job| job.job_id.as_str())
            .collect();
        assert_eq!(crawler_ids, vec!["crawler-active"]);
        assert!(retained.embedding_history.is_empty());

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn cleanup_expired_job_artifacts_uses_latest_artifact_mtime_not_directory_mtime() {
        let root = temp_history_root("cleanup-latest-artifact-mtime");
        let crawler_paths = write_crawler_job_fixture(
            &root,
            "crawler-fresh-artifact",
            CrawlerPhase::Completed,
            "2026-03-01T00:00:00Z",
            Some("2026-03-01T00:05:00Z"),
            Some(CrawlerTrigger::Admin),
        )
        .await;

        #[cfg(unix)]
        {
            let expired_at = SystemTime::now() - Duration::from_secs(8 * 24 * 60 * 60);
            let fresh_at = SystemTime::now() - Duration::from_secs(60);
            set_path_mtime(&crawler_paths.job_dir, expired_at);
            set_job_artifact_mtime(&crawler_paths, expired_at);
            set_path_mtime(&crawler_paths.progress, fresh_at);
        }

        let mut retained = reconstruct_retained_job_state(&root).await.unwrap();
        cleanup_expired_job_artifacts(
            &root,
            &HashSet::new(),
            &mut retained.crawler_history,
            &mut retained.embedding_history,
        )
        .await
        .unwrap();

        assert!(crawler_paths.job_dir.exists());
        let crawler_ids: Vec<_> = retained
            .crawler_history
            .iter()
            .map(|job| job.job_id.as_str())
            .collect();
        assert_eq!(crawler_ids, vec!["crawler-fresh-artifact"]);

        let _ = tokio::fs::remove_dir_all(&root).await;
    }
}
