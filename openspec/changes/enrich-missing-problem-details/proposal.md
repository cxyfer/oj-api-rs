## Why

Sheep and 0x3f ingestion can leave referenced problems as permanently sparse snapshots even though maintained source-specific crawlers can retrieve complete details. Daily ingestion should fill eligible missing details immediately while preserving the existing atomic daily-row contract.

## What Changes

- Select parsed problems for enrichment when they did not exist before ingestion, when their existing `title` and `content` are both empty or whitespace-only, or when both fields still exactly match the current curated daily snapshot.
- Commit the existing atomic problem-snapshot and daily-reference transaction before running sequential, best-effort detail retrieval.
- Reuse the maintained Codeforces, AtCoder, Luogu, and LeetCode single-problem/detail paths.
- Resolve LeetCode slugs to an existing local numeric problem ID before candidate selection and snapshot storage, while retaining the slug ID when no numeric row is available.
- Prefer non-empty source-fetched title and content for enrichment candidates while preserving curated metadata that the source detail response does not provide.
- Preserve AtCoder contest paths, Codeforces Gym daily keys, and LeetCode domain and whitespace-detail semantics.
- Allow public Codeforces Gym statements to be parsed even when the page navigation contains a normal sign-in link.
- Replace Codeforces ID placeholder titles with the official title parsed from the problem statement header, excluding the redundant problem-index prefix, including retries for previously enriched Gym rows that retained the placeholder.
- Prefer non-empty Codeforces tags from the existing contest metadata API, retain stored curated tags when source tags are unavailable, and retry previously enriched Codeforces rows whose tags are still empty.
- Keep failed enrichment isolated so it does not remove the daily row or prevent later candidates from being attempted.
- Preserve the existing database schema, Rust API, scheduler, configuration, and crawler CLI flags.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `daily-challenge-sources`: Define eligibility, ordering, source dispatch, key preservation, whitespace handling, and failure isolation for post-ingestion problem detail enrichment.

## Impact

- Python daily-source coordination in `scripts/daily_source.py` and local problem-ID lookup in `scripts/utils/database.py`.
- Existing source detail behavior in `scripts/codeforces.py` and `scripts/leetcode.py`.
- Focused Python tests for candidate selection, source dispatch, Gym persistence, whitespace replacement, and failure isolation.
- No database migration or public API response change.
