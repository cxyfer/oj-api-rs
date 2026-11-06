//! Shared terminal-state and output handling for crawler and embedding jobs.
use crate::models::{CrawlerJob, CrawlerStatus, EmbeddingJob};

pub(crate) trait TerminalJob {
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

pub(crate) fn apply_terminal_update<T: TerminalJob>(
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
