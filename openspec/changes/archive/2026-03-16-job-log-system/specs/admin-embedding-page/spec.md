## MODIFIED Requirements

### Requirement: Real-time embedding job progress via polling
The system SHALL provide `GET /admin/api/embeddings/status` returning current embedding job state and progress. When a job is running, the endpoint SHALL read progress from `scripts/logs/embedding/{job_id}/progress.json` using the job-scoped artifact layout. The frontend SHALL poll this endpoint at 3-second intervals while an embedding job is running. If an embedding job has started but `progress.json` is not yet present, the status response SHALL report phase `queued` for that job instead of `unknown`.

#### Scenario: Poll during rewrite phase
- **WHEN** job is running in rewrite phase
- **THEN** status response includes `{ "phase": "rewriting", "rewrite_progress": { "done": N, "total": M, "skipped": S } }`

#### Scenario: Poll during embedding phase
- **WHEN** job is running in embedding phase
- **THEN** status response includes `{ "phase": "embedding", "embed_progress": { "done": N, "total": M } }`

#### Scenario: Poll after job completion
- **WHEN** job has completed
- **THEN** status response includes final summary with succeeded/skipped/failed breakdown

#### Scenario: Progress file missing before first write
- **WHEN** job is running but `scripts/logs/embedding/{job_id}/progress.json` does not yet exist
- **THEN** status response returns `{ "phase": "queued" }` without error

### Requirement: Embedding job log viewing
The system SHALL provide `GET /admin/api/embeddings/{job_id}/output` returning the full current contents of `stdout.log`, `stderr.log`, and `python.log` for both running and completed embedding jobs. The endpoint SHALL use the job-scoped artifact layout under `scripts/logs/embedding/{job_id}/`. The JSON response SHALL always include the fields `stdout`, `stderr`, and `python_log`. If an artifact file is missing or empty, the corresponding field SHALL be an empty string. If the embedding job has been removed by retention cleanup or never existed, the endpoint SHALL return HTTP 404.

#### Scenario: View logs of running embedding job
- **WHEN** admin requests output for a running embedding job_id
- **THEN** response includes `{ "stdout": "...", "stderr": "...", "python_log": "..." }`
- **AND** each field contains the full current artifact content or an empty string

#### Scenario: View logs of completed job
- **WHEN** admin requests output for a completed embedding job_id
- **THEN** response includes `{ "stdout": "...", "stderr": "...", "python_log": "..." }`

#### Scenario: View logs of unknown or cleaned-up job_id
- **WHEN** admin requests output for a non-existent embedding job_id or a job removed by retention cleanup
- **THEN** system returns HTTP 404

## ADDED Requirements

### Requirement: Embedding admin UI shows live multi-stream logs
The `/admin/embeddings` page SHALL allow admins to inspect live embedding job output while the job is still running. When the embedding log modal is open, the frontend SHALL poll `/admin/api/embeddings/{job_id}/output` and refresh the displayed `stdout`, `stderr`, and `python_log` using the full content returned on each poll. The modal SHALL include a dedicated tab for `python.log` in addition to `stdout` and `stderr`.

#### Scenario: Running embedding job opens live log modal
- **WHEN** an embedding job is still running and admin opens its log modal
- **THEN** the UI polls the embedding output endpoint repeatedly
- **AND** the modal updates `stdout`, `stderr`, and `python_log` while the job is still running

#### Scenario: Python log is shown in separate tab
- **WHEN** admin views embedding logs in the modal
- **THEN** the modal provides separate tabs for `stdout`, `stderr`, and `python.log`

### Requirement: Embedding history follows retention cleanup
Embedding history exposed by `/admin/api/embeddings/status` SHALL only include jobs whose job-scoped artifact directories still exist or are currently running. When retention cleanup deletes an embedding job directory older than 7 days, the deleted job SHALL also be removed from embedding history so that the admin page does not present stale rows whose artifacts are unavailable.

#### Scenario: Fresh completed job remains in history
- **WHEN** an embedding job has completed within the retention window and its artifact directory still exists
- **THEN** `/admin/api/embeddings/status` includes that job in history

#### Scenario: Cleaned-up job disappears from history
- **WHEN** retention cleanup deletes `scripts/logs/embedding/{job_id}/`
- **THEN** `/admin/api/embeddings/status` no longer includes that job in history
