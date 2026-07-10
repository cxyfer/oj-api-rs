## 1. API Contract and Rust Tests

- [x] 1.1 Extend the daily missing-data response type to support no-job metadata for additional daily sources while preserving existing LeetCode `fetching` responses.
- [x] 1.2 Update `GET /api/v1/daily` missing additional-source handling to return HTTP 202 with `status = "ingestion_required"`, `retry_after = 30`, and `job_started = false` without registering fallback state.
- [x] 1.3 Add API tests that assert missing `source=sheep`/`source=0x3f` responses do not create `daily_fallback` entries or crawler jobs.
- [x] 1.4 Add or update Rust argument-validation tests for `--daily-file` rejecting absolute paths while accepting relative safe paths.

## 2. Python Daily Source Persistence

- [x] 2.1 Add a focused Codeforces daily-source storage helper that uses one SQLite connection and transaction for problem snapshot merge/upsert plus daily row update.
- [x] 2.2 Implement curated metadata merge rules that fill sparse Codeforces fields without clearing existing non-empty detail fields.
- [x] 2.3 Route Sheep and 0x3f daily-source ingestion through the atomic storage helper.
- [x] 2.4 Add Python storage tests for atomic rollback when the daily row write fails.
- [x] 2.5 Add Python storage tests for filling sparse metadata and preserving richer existing metadata.

## 3. Verification

- [x] 3.1 Run targeted Rust API and model validation tests covering daily endpoint and crawler argument validation.
- [x] 3.2 Run targeted Python daily-source storage tests.
- [x] 3.3 Run formatting/lint checks for touched Rust and Python files.
- [x] 3.4 Run `openspec status --change "fix-daily-source-review-findings"` and confirm the change is apply-ready.
