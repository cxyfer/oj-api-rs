## Why

Review identified that missing non-LeetCode daily sources can report a generic fetching state even though no API-triggered crawler exists, and that external daily-source ingestion has data consistency gaps around problem snapshots and daily rows.
This change tightens the API contract and persistence rules before the additional daily-source feature is shipped.

## What Changes

- Make missing additional daily-source responses explicitly indicate that no API fallback job was started and that ingestion must happen outside the API handler.
- Add test coverage that missing additional daily sources do not register fallback/crawler jobs.
- Store additional daily-source problem snapshots and their daily row atomically so partial writes cannot leave inconsistent daily ingestion state.
- Merge curated daily-source Codeforces metadata without silently ignoring improvements for existing sparse problem rows.
- Clarify the validated `--daily-file` path policy as relative safe paths only.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `daily-challenge`: Missing additional daily-source API responses expose no-job/manual-ingestion semantics while preserving no API fallback crawler behavior.
- `daily-challenge-sources`: Additional daily-source ingestion persists problem snapshots and daily rows atomically and merges curated metadata safely.
- `crawler-cli`: `--daily-file` validation policy is clarified as rejecting absolute paths and parent-directory traversal.

## Impact

- Affected Rust API and tests: `src/api/daily.rs`, `tests/api_test.rs`.
- Affected crawler validation: `src/models.rs` tests or docs around Codeforces `--daily-file` args.
- Affected Python crawler/storage code and tests: `scripts/codeforces.py`, `scripts/utils/database.py`, `scripts/test_daily_challenge_storage.py`.
- No dependency or database schema changes are expected.
