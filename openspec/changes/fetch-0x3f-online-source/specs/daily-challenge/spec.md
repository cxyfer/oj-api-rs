## MODIFIED Requirements

### Requirement: Daily challenge not found
The system SHALL return HTTP 404 only when no usable daily challenge record exists in the DB AND no fallback behavior applies. A daily row with malformed JSON, malformed problem refs, or no resolvable problems SHALL be treated as unusable. When fallback behavior applies, the system SHALL return HTTP 202 instead. LeetCode sources SHALL keep spawning the LeetCode fallback crawler. Configured additional daily sources SHALL spawn the dedicated daily-source fallback crawler from the API handler. Additional daily sources that cannot be fetched because required configuration is missing SHALL return HTTP 202 with an ingestion-required response and SHALL NOT write an empty daily row.

#### Scenario: No data, fallback triggered (com)
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-06-15` and no DB record exists and no fallback is running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` and spawns background crawler

#### Scenario: No data, fallback triggered (cn)
- **WHEN** client sends `GET /api/v1/daily?domain=cn` and no DB record exists for today (UTC+8) and no fallback is running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` and spawns background crawler with `--domain cn`

#### Scenario: Malformed stored refs treated as unusable
- **WHEN** the DB has a daily row whose `problems` column is invalid JSON or contains no valid problem references
- **THEN** the system treats the row as unusable and follows the existing no-data fallback behavior

#### Scenario: Missing referenced problems treated as unusable when none resolve
- **WHEN** the DB has a daily row but none of its problem references resolve from the `problems` table
- **THEN** the system treats the row as unusable and follows the existing no-data fallback behavior

#### Scenario: Missing Sheep source spawns API fallback crawler
- **WHEN** client sends `GET /api/v1/daily?source=sheep&date=2026-06-09` and no usable DB record exists
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}`
- **AND** the API handler registers a daily fallback entry for `sheep:2026-06-09`
- **AND** the API handler spawns `daily_source.py --daily-source sheep --date 2026-06-09`

#### Scenario: Missing 0x3f source spawns API fallback crawler when token is configured
- **WHEN** client sends `GET /api/v1/daily?source=0x3f&date=2026-06-09` and no usable DB record exists
- **AND** a direct local Tencent Docs token or its configured environment-variable fallback resolves to a non-empty value
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}`
- **AND** the API handler registers a daily fallback entry for `0x3f:2026-06-09`
- **AND** the API handler spawns `daily_source.py --daily-source 0x3f --date 2026-06-09`

#### Scenario: Missing 0x3f source without token returns ingestion required
- **WHEN** client sends `GET /api/v1/daily?source=0x3f&date=2026-06-09` and no usable DB record exists
- **AND** the direct local Tencent Docs token and its configured environment-variable fallback are both missing or empty
- **THEN** system returns HTTP 202 with a body that includes `status = "ingestion_required"`, `retry_after = 30`, and `job_started = false`
- **AND** the API handler does not spawn the daily-source crawler

#### Scenario: Additional source fallback already running
- **WHEN** client sends `GET /api/v1/daily?source=sheep&date=2026-06-09`
- **AND** a fallback for `sheep:2026-06-09` is already Running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` without spawning a new crawler
