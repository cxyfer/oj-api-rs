## Context

The branch adds additional daily challenge sources (`sheep`, `0x3f`) on top of the compact `daily_challenge` storage model. The Rust daily API can serve those sources from stored rows, but only LeetCode sources have an API-triggered fallback crawler. Codeforces-based daily-source ingestion currently writes minimal problem snapshots before writing the daily row.

Review found three areas to tighten before implementation is considered complete: external-source missing-data responses should not imply an active API job, external-source persistence should be atomic, and curated Codeforces metadata should not be ignored when an existing row is sparse. The Codeforces `--daily-file` validation remains intentionally conservative and should be documented/test-locked as relative safe paths only.

## Goals / Non-Goals

**Goals:**

- Make missing additional daily-source API responses explicit about no API fallback job being started.
- Add regression tests for missing additional-source no-job behavior.
- Ensure Codeforces daily-source ingestion writes problem snapshots and the daily row in one SQLite transaction.
- Merge curated daily-source metadata into existing sparse Codeforces rows without clearing richer existing fields.
- Lock the `--daily-file` relative-safe path policy in spec and tests.

**Non-Goals:**

- Do not add API-triggered crawlers for `sheep` or `0x3f`.
- Do not change the `daily_challenge` table schema.
- Do not allow arbitrary absolute import paths for `--daily-file` in this change.
- Do not add new external dependencies.

## Decisions

### Explicit no-job API response for additional daily sources

Keep HTTP 202 for missing additional daily sources to preserve the existing status-code contract, but extend the response body for non-LeetCode sources with machine-readable no-job metadata such as `job_started: false` and a concise status/message indicating manual or scheduled ingestion is required.

Alternative considered: return 404 for missing additional daily sources. This is semantically clean, but it would change the status-code contract already captured for additional-source fallback behavior.

### Single-connection transaction for daily-source ingestion

Add a focused storage helper for Codeforces daily sources that performs metadata merge/upsert and daily row update on the same SQLite connection inside one transaction. This avoids coordinating transactions across the existing `ProblemsDB` and `DailyChallengeDB` wrappers.

Alternative considered: call existing `update_problems()` and `update_daily()` and compensate on failure. Compensation is less reliable because failures may happen after the first commit and cleanup can fail too.

### Metadata merge rather than overwrite

For curated daily-source snapshots, update existing Codeforces rows with non-empty incoming values only where this does not erase richer existing values. Fill sparse fields such as title, link, rating, difficulty, tags, and source URL from curated input when existing values are empty or placeholder-like.

Alternative considered: use `force_update=True`. This is simple but risks replacing detail-rich Codeforces records with minimal daily-source snapshots.

### Relative-safe `--daily-file` policy

Keep Rust validation rejecting absolute paths and parent traversal for `--daily-file`. Tests and spec should make this deliberate so operators know imports must be placed under the crawler working tree or another approved relative location.

Alternative considered: allow absolute paths under a configured import root after canonicalization. That is more flexible but requires new configuration and threat-modeling beyond this fix.

## Risks / Trade-offs

- [Risk] Existing clients may ignore new 202 response fields and continue polling. → Mitigation: keep backward-compatible `status` and `retry_after`, but document the new no-job fields for smarter clients.
- [Risk] A new transaction helper may duplicate some database serialization logic. → Mitigation: keep it narrow and reuse existing row shapes/serialization helpers where practical.
- [Risk] Metadata merge rules can become too clever. → Mitigation: only fill missing/empty fields and avoid overwriting non-empty detail fields.
- [Risk] Relative-only `--daily-file` may be inconvenient for mounted import paths. → Mitigation: document this as the current safe policy; a later change can introduce configured safe import roots.
