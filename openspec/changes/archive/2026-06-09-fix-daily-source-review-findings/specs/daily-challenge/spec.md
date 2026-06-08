## MODIFIED Requirements

### Requirement: Daily challenge not found
The system SHALL return HTTP 404 only when no usable daily challenge record exists in the DB AND no fallback behavior applies. A daily row with malformed JSON, malformed problem refs, or no resolvable problems SHALL be treated as unusable. When fallback behavior applies, the system SHALL return HTTP 202 instead. LeetCode sources SHALL keep spawning the LeetCode fallback crawler. Additional daily sources SHALL return HTTP 202 without spawning a crawler from the API handler, and the response body SHALL expose that no API fallback job was started and that ingestion is required outside the API handler.

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

#### Scenario: Missing additional daily source does not spawn API fallback crawler
- **WHEN** client sends `GET /api/v1/daily?source=sheep&date=2026-06-02` and no usable DB record exists
- **THEN** system returns HTTP 202 with a body that includes `status = "ingestion_required"`, `retry_after = 30`, and `job_started = false`
- **AND** the API handler does not register a daily fallback entry
- **AND** the API handler does not spawn or register `leetcode.py` or `codeforces.py`
