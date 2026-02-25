### Requirement: Dual progress bar for embedding pipeline
The admin embeddings page SHALL render two independent progress bars simultaneously during the embedding pipeline. The rewriting bar SHALL display `Rewriting: {done}/{total} (Skipped: {skipped})` when `rewrite_progress.total > 0`. The embedding bar SHALL display `Embedding: {done}/{total}` when `embed_progress.total > 0`. Both bars SHALL clamp percentage to [0, 100]. Terminal phases (`completed`, `failed`) SHALL render only a status label with no progress bars.

#### Scenario: Both bars render simultaneously during pipeline
- **WHEN** phase is `rewriting` or `embedding` and `rewrite_progress.total > 0`
- **THEN** rewriting progress bar is visible with label `Rewriting: {done}/{total} (Skipped: {skipped})`
- **AND** if `embed_progress.total > 0`, embedding progress bar is also visible

#### Scenario: Percentage clamped to bounds
- **INVARIANT** `0 <= displayed_percentage <= 100` for both bars
- **BOUNDARY** `total == 0` → bar not rendered

#### Scenario: Terminal phases show status only
- **WHEN** phase is `completed` or `failed`
- **THEN** only status label is rendered (no progress bars)

#### Scenario: i18n key for skipped exists
- **INVARIANT** `embeddings.progress.skipped` key exists in en, zh-TW, zh-CN locales

## ADDED Requirements

### Requirement: Admin embedding page renders with per-source statistics
The system SHALL serve a `/admin/embeddings` page (Askama template extending `base.html`) that displays per-source embedding statistics. The page SHALL show cards with: total problems, problems with content, already embedded count, and pending count for each source. Statistics SHALL be fetched via `GET /admin/api/embeddings/stats` which queries SQLite directly (no Python subprocess).

#### Scenario: Page loads with multiple sources
- **WHEN** admin navigates to `/admin/embeddings`
- **THEN** page renders with one stats card per source showing total, with_content, embedded, and pending counts
- **AND** counts satisfy invariant: `pending == with_content - embedded` for each source

#### Scenario: Page loads with no problems in database
- **WHEN** admin navigates to `/admin/embeddings` and the problems table is empty
- **THEN** page renders with a message indicating no problems are available

#### Scenario: Stats API returns correct counts
- **WHEN** `GET /admin/api/embeddings/stats` is called
- **THEN** response is JSON with per-source objects containing `total`, `with_content`, `embedded`, `pending` fields
- **AND** all counts are non-negative integers

### Requirement: Admin can trigger embedding jobs
The system SHALL provide a `POST /admin/api/embeddings/trigger` endpoint accepting `{ source, rebuild, dry_run, batch_size, filter }`. The system SHALL validate: `source` is a known source or "all", `batch_size` is in [1, 256], `filter` is optional non-empty string. The trigger SHALL spawn `uv run python3 embedding_cli.py` as a subprocess with appropriate arguments.

#### Scenario: Trigger embedding for specific source
- **WHEN** admin POSTs `{ "source": "leetcode" }` to trigger endpoint
- **THEN** system returns 202 with `{ "job_id": "<uuid>" }`
- **AND** a subprocess is spawned running `embedding_cli.py --build --source leetcode`

#### Scenario: Trigger with invalid source
- **WHEN** admin POSTs `{ "source": "invalid" }` to trigger endpoint
- **THEN** system returns 400 with RFC 7807 error

#### Scenario: Trigger with rebuild flag
- **WHEN** admin POSTs `{ "source": "all", "rebuild": true }` to trigger endpoint
- **THEN** subprocess includes `--rebuild` flag

#### Scenario: Trigger while another embedding job is running
- **WHEN** admin POSTs trigger while an embedding job has status Running
- **THEN** system returns 409 with "an embedding job is already running"

### Requirement: Embedding job uses independent lock from crawler
The system SHALL maintain separate `embedding_lock` and `embedding_history` fields in `AppState`. Embedding jobs and crawler jobs MAY execute concurrently. The embedding lock SHALL prevent concurrent embedding jobs (at most one running at a time).

#### Scenario: Embedding and crawler run in parallel
- **WHEN** a crawler job is running AND admin triggers an embedding job
- **THEN** embedding job starts successfully (no 409)

#### Scenario: Two embedding jobs cannot run concurrently
- **WHEN** an embedding job is running AND admin triggers another embedding job
- **THEN** second trigger returns 409

### Requirement: Real-time embedding job progress via polling
The system SHALL provide `GET /admin/api/embeddings/status` returning current job state and progress. When a job is running, the endpoint SHALL read `scripts/logs/{job_id}.progress.json` to obtain sub-task progress (rewrite and embedding phases). The frontend SHALL poll this endpoint at 3-second intervals while a job is running.

#### Scenario: Poll during rewrite phase
- **WHEN** job is running in rewrite phase
- **THEN** status response includes `{ "phase": "rewriting", "rewrite_progress": { "done": N, "total": M, "skipped": S } }`

#### Scenario: Poll during embedding phase
- **WHEN** job is running in embedding phase
- **THEN** status response includes `{ "phase": "embedding", "embed_progress": { "done": N, "total": M } }`

#### Scenario: Poll after job completion
- **WHEN** job has completed
- **THEN** status response includes final summary with succeeded/skipped/failed breakdown

#### Scenario: Progress file missing or unreadable
- **WHEN** job is running but progress file does not exist or is malformed
- **THEN** status response returns `{ "phase": "unknown" }` without error

### Requirement: Embedding job log viewing
The system SHALL provide `GET /admin/api/embeddings/{job_id}/output` returning stdout and stderr of a completed job. Logs SHALL be checked in-memory first, then fallback to `scripts/logs/{job_id}.stdout.log` and `scripts/logs/{job_id}.stderr.log`.

#### Scenario: View logs of completed job
- **WHEN** admin requests output for a completed job_id
- **THEN** response includes `{ "stdout": "...", "stderr": "..." }`

#### Scenario: View logs of unknown job_id
- **WHEN** admin requests output for a non-existent job_id
- **THEN** system returns 404

### Requirement: Embedding admin page supports i18n
The system SHALL provide i18n keys for all user-visible text on the embeddings page in en, zh-TW, and zh-CN locales. Keys SHALL follow existing naming conventions (e.g., `embeddings.title`, `embeddings.trigger`, `embeddings.stats.*`).

#### Scenario: Page renders in zh-TW
- **WHEN** admin has language set to zh-TW
- **THEN** all labels, buttons, and status messages display in Traditional Chinese

### Requirement: Embedding page follows admin auth middleware
The `/admin/embeddings` page and all `/admin/api/embeddings/*` endpoints SHALL be protected by the existing `admin_auth` middleware, requiring either a valid `oj_admin_session` cookie or `x-admin-secret` header.

#### Scenario: Unauthenticated access to embeddings page
- **WHEN** unauthenticated user accesses `/admin/embeddings`
- **THEN** system redirects to `/admin/login`
