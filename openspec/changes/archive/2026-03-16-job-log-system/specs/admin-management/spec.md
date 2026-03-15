## MODIFIED Requirements

### Requirement: Crawler trigger (async)
The system SHALL trigger Python crawlers via `POST /admin/api/crawlers/trigger` with a JSON body specifying the source. Execution SHALL be asynchronous: the endpoint returns immediately with a `job_id`. Manual admin-triggered crawler jobs SHALL remain single-instance: while one manual crawler job is running, a second manual trigger request SHALL be rejected with HTTP 409. Daily fallback crawler jobs MAY run concurrently with a manual crawler job and SHALL be tracked as separate crawler-domain jobs.

#### Scenario: Trigger crawler
- **WHEN** client sends `POST /admin/api/crawlers/trigger` with `{"source": "leetcode"}`
- **THEN** system starts the crawler subprocess in the background, returns HTTP 202 with `{"job_id": "..."}`

#### Scenario: Concurrent manual trigger rejected
- **WHEN** a manual admin-triggered crawler job is already running and client sends another trigger request
- **THEN** system returns HTTP 409 with error detail indicating a manual crawler is already running

#### Scenario: Manual crawler and daily fallback run in parallel
- **WHEN** a manual admin-triggered crawler job is already running and the daily challenge fallback needs to spawn a crawler job
- **THEN** the fallback crawler job is allowed to start as a separate crawler-domain job
- **AND** both jobs remain visible through the crawler admin status/history surface

#### Scenario: Crawler timeout
- **WHEN** the crawler subprocess exceeds 300 seconds
- **THEN** system kills the subprocess and marks the job as timed out

### Requirement: Crawler status query
The system SHALL expose `GET /admin/api/crawlers/status` to check crawler execution state. The response SHALL support multiple concurrent crawler-domain jobs by returning `running_jobs` as an array of running crawler jobs and `history` as an array of retained crawler-domain jobs. The crawler-domain surface SHALL include both manual admin-triggered crawler jobs and daily fallback crawler jobs. Each returned history or running job entry SHALL omit inline `stdout` and `stderr` blobs.

#### Scenario: No crawler jobs running
- **WHEN** no crawler-domain job is active
- **THEN** system returns HTTP 200 with `{"running": false, "running_jobs": [], "history": []}` or an equivalent empty `history` array when no retained jobs exist

#### Scenario: One crawler job running
- **WHEN** exactly one crawler-domain job is running
- **THEN** system returns HTTP 200 with `{"running": true, "running_jobs": [{"job_id": "...", "source": "...", "started_at": "..."}], "history": [...]}`

#### Scenario: Manual crawler and daily fallback both running
- **WHEN** one manual crawler job and one daily fallback crawler job are both active
- **THEN** system returns both jobs in the `running_jobs` array
- **AND** the daily fallback job is distinguishable in history/status metadata through its trigger marker

## ADDED Requirements

### Requirement: Crawler job output polling
The system SHALL expose `GET /admin/api/crawlers/{job_id}/output` for crawler-domain jobs, including daily fallback jobs. The endpoint SHALL return the full current contents of `stdout.log`, `stderr.log`, and `python.log` on each poll using the JSON fields `stdout`, `stderr`, and `python_log`. If an artifact file is missing or empty, the corresponding field SHALL be returned as an empty string. The endpoint SHALL support both running and completed jobs. If the job has been removed by retention cleanup or never existed, the endpoint SHALL return HTTP 404.

#### Scenario: Running crawler output is readable
- **WHEN** admin requests `/admin/api/crawlers/{job_id}/output` for a running crawler-domain job
- **THEN** the response includes `stdout`, `stderr`, and `python_log`
- **AND** each field contains the full current artifact content or an empty string if that artifact has no content yet

#### Scenario: Daily fallback output uses crawler endpoint
- **WHEN** admin requests `/admin/api/crawlers/{job_id}/output` for a daily fallback job
- **THEN** the response succeeds using the same crawler output contract as a manual crawler job

#### Scenario: Retained job output not found after cleanup
- **WHEN** admin requests output for a crawler-domain job whose per-job directory has been removed by retention cleanup
- **THEN** the system returns HTTP 404

### Requirement: Crawler progress polling
The system SHALL expose `GET /admin/api/crawlers/{job_id}/progress` for crawler-domain jobs, including daily fallback jobs. The endpoint SHALL return crawler phase-level progress using the closed phase set `queued`, `running`, `completed`, `failed`, `cancelled`, and `timed_out`. If the job has started but `progress.json` is not yet present, the endpoint SHALL return phase `queued`. If the job has been removed by retention cleanup or never existed, the endpoint SHALL return HTTP 404.

#### Scenario: Missing progress file returns queued
- **WHEN** a crawler-domain job has started and `progress.json` does not yet exist
- **THEN** `/admin/api/crawlers/{job_id}/progress` returns a crawler progress payload with phase `queued`

#### Scenario: Running crawler returns current phase
- **WHEN** a crawler-domain job has written `progress.json` with phase `running`
- **THEN** `/admin/api/crawlers/{job_id}/progress` returns phase `running`

#### Scenario: Cleaned-up progress is not queryable
- **WHEN** admin requests progress for a crawler-domain job whose artifact directory has been removed by retention cleanup
- **THEN** the system returns HTTP 404

### Requirement: Crawler admin UI shows live logs for running jobs
The `/admin/crawlers` page SHALL allow admins to inspect running crawler-domain jobs, not only completed jobs. The crawler history table SHALL display both manual crawler jobs and daily fallback jobs in the same table. The frontend SHALL poll crawler status every 3 seconds while one or more crawler-domain jobs are running, and an open log modal SHALL poll the crawler output endpoint to refresh `stdout`, `stderr`, and `python_log` with full current content. The UI SHALL provide a distinct tab for `python.log`.

#### Scenario: Running job can open live log modal
- **WHEN** a crawler-domain job is still running and admin clicks its log action in the crawler page
- **THEN** the UI opens the log modal
- **AND** the modal polls the crawler output endpoint and refreshes `stdout`, `stderr`, and `python_log`

#### Scenario: Daily fallback appears in shared history table
- **WHEN** a daily fallback crawler job has started or completed
- **THEN** the crawler page includes that job in the same history table used for manual crawler jobs
- **AND** the row metadata identifies it as a daily fallback trigger

#### Scenario: Python log has dedicated tab
- **WHEN** admin opens the crawler log modal
- **THEN** the modal includes separate tabs for `stdout`, `stderr`, and `python.log`
