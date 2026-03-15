## ADDED Requirements

### Requirement: Job-scoped log artifact layout
The system SHALL store every Rust-launched crawler, embedding, and daily fallback job artifact under `scripts/logs/{job_type}/{job_id}/`. The `job_type` path segment SHALL be a closed set containing only `crawler` and `embedding`. Daily fallback executions SHALL use `job_type=crawler`. Each job directory SHALL use exactly these filenames: `stdout.log`, `stderr.log`, `python.log`, and `progress.json`. The system SHALL NOT create new flat files such as `scripts/logs/{job_id}.stdout.log`, `scripts/logs/{job_id}.stderr.log`, or `scripts/logs/{job_id}.progress.json` for jobs created after this change.

#### Scenario: Crawler job creates canonical artifact directory
- **WHEN** the admin triggers a crawler job and the subprocess is prepared
- **THEN** the system creates `scripts/logs/crawler/{job_id}/`
- **AND** the recognized artifact filenames for that job are `stdout.log`, `stderr.log`, `python.log`, and `progress.json`

#### Scenario: Embedding job creates canonical artifact directory
- **WHEN** the admin triggers an embedding job and the subprocess is prepared
- **THEN** the system creates `scripts/logs/embedding/{job_id}/`
- **AND** the recognized artifact filenames for that job are `stdout.log`, `stderr.log`, `python.log`, and `progress.json`

#### Scenario: Daily fallback uses crawler artifact taxonomy
- **WHEN** the daily challenge fallback spawns a background crawler because no DB record exists
- **THEN** the system stores that job under `scripts/logs/crawler/{job_id}/`
- **AND** the job remains queryable from the crawler admin surface rather than a separate namespace

### Requirement: Live subprocess output capture
The system SHALL capture subprocess `stdout` and `stderr` incrementally while the job is running. The capture pipeline SHALL append live output to `stdout.log` and `stderr.log` during execution rather than waiting until process completion. Polling the admin output endpoints multiple times for the same running job SHALL return log content that is monotonically non-decreasing for each stream.

#### Scenario: Running job exposes incremental stdout
- **WHEN** a crawler or embedding subprocess writes bytes to stdout while still running
- **THEN** the system appends those bytes to `stdout.log` before the process exits
- **AND** a later poll of the corresponding admin output endpoint includes at least the bytes returned by earlier polls

#### Scenario: Running job exposes incremental stderr
- **WHEN** a crawler or embedding subprocess writes bytes to stderr while still running
- **THEN** the system appends those bytes to `stderr.log` before the process exits
- **AND** a later poll of the corresponding admin output endpoint includes at least the bytes returned by earlier polls

#### Scenario: Quiet stream remains readable
- **WHEN** a running job has not yet produced output for one or more streams
- **THEN** the output endpoint still returns the `stdout`, `stderr`, and `python_log` fields
- **AND** any stream without data is represented as an empty string

### Requirement: Python logger stream is a first-class artifact
Python application logger output for a job SHALL be written to `python.log` as a separate artifact. The system SHALL keep the existing root date log files at `scripts/logs/YYYY-MM-DD.log`, and those root date logs MAY retain ANSI-colored output. The per-job `python.log` artifact SHALL be ANSI-clean plain text and SHALL NOT be merged into `stderr.log`.

#### Scenario: Root date log keeps ANSI formatting
- **WHEN** Python logging writes to the shared daily log file under `scripts/logs/YYYY-MM-DD.log`
- **THEN** the system preserves the existing ANSI-colored formatting for that root date log

#### Scenario: Per-job python log is plain text
- **WHEN** Python code emits logger output while running under a job-scoped environment
- **THEN** the system writes the same logger records into `python.log`
- **AND** the stored `python.log` content contains no ANSI escape sequences

#### Scenario: Logger output is isolated from stderr artifact
- **WHEN** a Python job emits normal application logger records and also writes separate raw stderr output
- **THEN** the logger records appear in `python.log`
- **AND** raw stderr output appears in `stderr.log`
- **AND** normal logger records are not duplicated into `stderr.log`

### Requirement: Progress schema partition by job type
The system SHALL persist `progress.json` per job. For crawler-domain jobs, including daily fallback, the progress schema SHALL be phase-level only and SHALL use the closed phase set `queued`, `running`, `completed`, `failed`, `cancelled`, and `timed_out`. Crawler progress MAY include `message` and `updated_at`, but SHALL NOT require `rewrite_progress` or `embed_progress`. For embedding jobs, the system SHALL preserve the detailed progress structure with `phase`, `rewrite_progress`, and `embed_progress`.

#### Scenario: Crawler job starts in queued phase
- **WHEN** a crawler-domain job has been accepted and `progress.json` has not yet been written by the child process
- **THEN** the crawler progress endpoint reports phase `queued`

#### Scenario: Crawler progress stays phase-level
- **WHEN** the system returns progress for a crawler-domain job
- **THEN** the response uses the phase-level crawler schema
- **AND** the response does not require `rewrite_progress` or `embed_progress`

#### Scenario: Embedding progress keeps detailed sections
- **WHEN** the system returns progress for an embedding job during the rewrite or embedding pipeline
- **THEN** the response includes `phase`
- **AND** the response preserves `rewrite_progress` and `embed_progress` when available

#### Scenario: Terminal phase does not regress
- **WHEN** a job reaches one of the terminal phases `completed`, `failed`, `cancelled`, or `timed_out`
- **THEN** subsequent progress reads for that job do not return `queued` or `running`

### Requirement: Job artifact lifecycle and retention cleanup
The system SHALL apply automated retention only to per-job directories under `scripts/logs/{job_type}/{job_id}/`. Job directories older than 7 days SHALL be eligible for deletion as a unit. Root date log files `scripts/logs/YYYY-MM-DD.log` SHALL NOT be deleted by this retention pass. Legacy flat files SHALL NOT be deleted by this retention pass. A running job directory SHALL NEVER be deleted by retention cleanup. When a retained job directory is deleted, the corresponding job SHALL also be removed from admin-visible history so that stale history rows do not remain after artifacts expire.

#### Scenario: Old completed job directory is deleted
- **WHEN** a completed job directory is older than 7 days according to the retention policy
- **THEN** the system deletes the entire `scripts/logs/{job_type}/{job_id}/` directory
- **AND** the deleted job no longer appears in crawler or embedding history responses

#### Scenario: Root date log is preserved
- **WHEN** retention cleanup runs and encounters `scripts/logs/2026-03-14.log`
- **THEN** the system leaves that root date log untouched

#### Scenario: Running job directory is skipped by cleanup
- **WHEN** retention cleanup runs while a crawler or embedding job is still running
- **THEN** the system does not delete that job's artifact directory even if its timestamps are older than 7 days

#### Scenario: Legacy flat files are ignored by cleanup
- **WHEN** retention cleanup encounters old files such as `scripts/logs/{job_id}.stdout.log`
- **THEN** the system does not treat them as cleanup targets for this change
