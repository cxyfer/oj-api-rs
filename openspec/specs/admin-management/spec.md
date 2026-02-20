# admin-management Specification

## Purpose
TBD - created by archiving change oj-api-rs-v1. Update Purpose after archive.
## Requirements
### Requirement: Admin HTML pages
The system SHALL serve HTML admin pages via Askama templates at `/admin/` (index), `/admin/problems` (problem management). All HTML output SHALL be auto-escaped by Askama to prevent XSS. Problem `content` (raw HTML) SHALL be rendered in an iframe with `sandbox` attribute.

#### Scenario: Admin index page
- **WHEN** client sends `GET /admin/` with valid admin secret
- **THEN** system returns HTML page with navigation to problem and token management

#### Scenario: Problem management page
- **WHEN** client sends `GET /admin/problems` with valid admin secret
- **THEN** system returns HTML page listing problems with pagination and CRUD controls

### Requirement: Problem CRUD via admin API
The system SHALL support creating, updating, and deleting problems via admin API endpoints.

#### Scenario: Create problem
- **WHEN** client sends `POST /admin/api/problems` with valid admin secret and problem JSON body
- **THEN** system inserts the problem into the DB and returns HTTP 201

#### Scenario: Update problem
- **WHEN** client sends `PUT /admin/api/problems/{source}/{id}` with updated fields
- **THEN** system updates the matching problem and returns HTTP 200

#### Scenario: Delete problem
- **WHEN** client sends `DELETE /admin/api/problems/{source}/{id}`
- **THEN** system deletes the problem AND its corresponding embedding from `vec_embeddings` and `problem_embeddings`, then returns HTTP 204

#### Scenario: Delete non-existent problem
- **WHEN** client sends `DELETE /admin/api/problems/leetcode/999999`
- **THEN** system returns HTTP 404

### Requirement: API token management
The system SHALL support listing, creating, and revoking API tokens. Tokens SHALL be 64-character hex strings (32 random bytes).

#### Scenario: List tokens
- **WHEN** client sends `GET /admin/api/tokens`
- **THEN** system returns all tokens with `token`, `label`, `created_at`, `last_used_at`, `is_active`

#### Scenario: Create token
- **WHEN** client sends `POST /admin/api/tokens` with `{"label": "my-bot"}`
- **THEN** system generates a random 64-char hex token, inserts it, and returns HTTP 201 with the new token

#### Scenario: Revoke token
- **WHEN** client sends `DELETE /admin/api/tokens/{token}`
- **THEN** system sets `is_active = 0` for that token and returns HTTP 204

#### Scenario: Revoke non-existent token
- **WHEN** client sends `DELETE /admin/api/tokens/nonexistent`
- **THEN** system returns HTTP 404

### Requirement: Crawler trigger (async)
The system SHALL trigger Python crawlers via `POST /admin/api/crawlers/trigger` with a JSON body specifying the source. Execution SHALL be asynchronous: the endpoint returns immediately with a `job_id`. A single-instance lock SHALL prevent concurrent crawler runs.

#### Scenario: Trigger crawler
- **WHEN** client sends `POST /admin/api/crawlers/trigger` with `{"source": "leetcode"}`
- **THEN** system starts the crawler subprocess in the background, returns HTTP 202 with `{"job_id": "..."}`

#### Scenario: Concurrent trigger rejected
- **WHEN** a crawler is already running and client sends another trigger request
- **THEN** system returns HTTP 409 with error detail indicating a crawler is already running

#### Scenario: Crawler timeout
- **WHEN** the crawler subprocess exceeds 300 seconds
- **THEN** system kills the subprocess and marks the job as timed out

### Requirement: Crawler status query
The system SHALL expose `GET /admin/api/crawlers/status` to check the current crawler execution state.

#### Scenario: No crawler running
- **WHEN** no crawler is active
- **THEN** system returns HTTP 200 with `{"running": false, "last_job": {...}}` or `null` if never run

#### Scenario: Crawler in progress
- **WHEN** a crawler is running
- **THEN** system returns HTTP 200 with `{"running": true, "job_id": "...", "source": "...", "started_at": "..."}`

### Requirement: Admin CORS restriction
Admin routes SHALL NOT include CORS headers allowing cross-origin requests. Only same-origin access SHALL be permitted.

#### Scenario: Cross-origin admin request
- **WHEN** a cross-origin request is made to `/admin/*`
- **THEN** system does not include `Access-Control-Allow-Origin` header

