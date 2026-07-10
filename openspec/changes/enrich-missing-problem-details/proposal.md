## Why

Sheep and 0x3f ingestion can leave referenced problems as permanently sparse snapshots even though maintained source-specific crawlers can retrieve complete details. Daily ingestion should fill eligible missing details immediately while preserving the existing atomic daily-row contract.

## What Changes

- Select parsed problems for enrichment when they did not exist before ingestion, or when their existing `title` and `content` are both empty or whitespace-only.
- Commit the existing atomic problem-snapshot and daily-reference transaction before running sequential, best-effort detail retrieval.
- Reuse the maintained Codeforces, AtCoder, Luogu, and LeetCode single-problem/detail paths.
- Preserve AtCoder contest paths, Codeforces Gym daily keys, and LeetCode domain and whitespace-detail semantics.
- Keep failed enrichment isolated so it does not remove the daily row or prevent later candidates from being attempted.
- Preserve the existing database schema, Rust API, scheduler, configuration, and crawler CLI flags.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `daily-challenge-sources`: Define eligibility, ordering, source dispatch, key preservation, whitespace handling, and failure isolation for post-ingestion problem detail enrichment.

## Impact

- Python daily-source coordination in `scripts/daily_source.py`.
- Existing source detail behavior in `scripts/codeforces.py` and `scripts/leetcode.py`.
- Focused Python tests for candidate selection, source dispatch, Gym persistence, whitespace replacement, and failure isolation.
- No database migration or public API response change.
