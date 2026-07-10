## Context

Sheep and 0x3f daily ingestion currently persists curated problem snapshots and ordered daily references atomically. Those snapshots can be intentionally sparse, while the project already maintains source-specific detail retrieval paths for Codeforces, AtCoder, Luogu, and LeetCode. Without coordination after daily storage, a newly referenced problem or an existing row with no usable title and content can remain sparse indefinitely.

The enrichment decision must observe the database before the curated snapshot is written. Otherwise every newly inserted problem would appear to exist and curated values could hide whether a previously blank row qualified. The existing atomic snapshot transaction remains the source of truth for daily ingestion and must complete before any network-dependent enrichment begins.

## Goals / Non-Goals

**Goals:**

- Enrich problems that were absent before ingestion or whose existing title and content are both blank after trimming whitespace.
- Reuse each source's maintained single-problem or detail retrieval path.
- Preserve AtCoder contest context, Codeforces Gym storage keys, and LeetCode domain selection.
- Keep enrichment sequential and best-effort after the atomic daily snapshot commits.
- Isolate a failed detail request so later candidates are still attempted and stored daily data remains intact.

**Non-Goals:**

- Changing the database schema, Rust API, scheduler, configuration, or crawler CLI flags.
- Broadening the ordinary Codeforces single-problem input grammar to accept stored `GYM` identifiers.
- Running enrichment when only one of title or content is blank.
- Replacing non-blank existing title or content values.
- Adding a new generic crawler abstraction or parallel enrichment queue.

## Decisions

### Determine candidates before snapshot storage

The daily-source coordinator reads each parsed `(source, id)` before calling the existing atomic storage operation. A problem is a candidate when no row exists, when both stored `title` and `content` are blank after whitespace trimming, or when both stored fields still match the current curated daily snapshot after trimming. The equality case lets a later ingestion retry rows left with only daily-source data after a previous source crawler failure. Once source details differ from the snapshot, later runs skip enrichment. Parsed metadata and tags do not otherwise change this decision.

This keeps the rule tied to pre-ingestion database state. Checking after storage was rejected because newly inserted rows and curated placeholder metadata would erase the distinction the requirement depends on.

### Commit the daily snapshot before network enrichment

The existing transaction continues to upsert snapshots and write ordered daily references as one unit. Enrichment starts only after that transaction returns success. If storage fails, no detail crawler is called.

Including network requests inside the transaction was rejected because it would hold the SQLite transaction open, increase lock time, and allow an external crawler failure to discard otherwise valid daily data.

### Dispatch directly to maintained source detail paths

The coordinator constructs clients with the same data directory and database path, then dispatches in parsed order:

- Codeforces uses `fetch_single_problem`; Gym candidates strip the `GYM` fetch prefix while passing the original stored ID explicitly.
- AtCoder uses `fetch_single_problem` with `<contest>/<task-id>` when contest context is available.
- Luogu uses `fetch_single_problem` with the parsed problem ID.
- LeetCode uses `get_problem` after the snapshot exists and selects `cn` or `com` from the parsed URL.

A new cross-source crawler interface was rejected because the existing methods already own source parsing and persistence, and the required dispatch is small and explicit.

### Preserve Codeforces Gym identity explicitly

Codeforces detail retrieval accepts an optional stored Gym ID that must match the normalized contest and index. It fetches the Gym URL but writes back to the original `GYM...` key. The normal one-argument method continues to reject `GYM`-prefixed input.

Silently normalizing `GYM106539D` to `106539D` was rejected because it would create a second row and break the daily reference.

Public Codeforces pages normally include a navigation link to `/enter` even when the complete problem statement is present. The detail path therefore relies on the existing response-status, challenge-page, and statement checks instead of treating any `/enter` occurrence as an authentication failure.

### Prefer fetched source details without clearing curated metadata

Daily enrichment explicitly marks non-empty source-fetched detail fields as preferred during the existing database merge. Codeforces replaces fetched `content`; AtCoder and Luogu replace fetched `title` and `content`; LeetCode retains its existing blank-text replacement behavior. Other fields still use the normal non-empty merge, so a source response that omits rating, difficulty, or tags does not erase values supplied by Sheep or 0x3f.

AtCoder obtains the official title from its existing contest task-list parser when daily enrichment requests source precedence, then fetches the statement from the explicit contest/task path. Direct single-problem CLI calls keep their prior merge behavior unless the coordinator opts into source precedence.

Forcing the entire fetched problem row to overwrite the snapshot was rejected because Codeforces and AtCoder single-problem responses do not populate every metadata field and would clear useful curated values.

### Treat whitespace-only LeetCode details as missing

LeetCode detail retrieval treats whitespace-only text fields as missing, fetches detail when content is blank even if tags are present, and replaces only blank text values with non-blank fetched values. This lets eligible sparse rows become usable without overwriting richer stored text.

### Run enrichment sequentially and best-effort

Candidates are attempted one at a time in parsed order. False results and exceptions are logged per candidate, then processing continues. Sequential execution preserves deterministic behavior and avoids adding source-specific concurrency or throttling concerns to daily ingestion.

## Risks / Trade-offs

- [Post-commit enrichment can leave a sparse row when a source is unavailable] -> Keep failures visible in logs and allow a later daily ingestion to select the row again while both title and content remain blank.
- [Sequential requests increase total ingestion latency] -> Limit work to newly seen or fully blank rows and preserve the simpler source crawler rate behavior.
- [Source-specific signatures can diverge over time] -> Cover every dispatch path with focused tests and keep dispatch code adjacent in the daily-source coordinator.
- [Gym fetch IDs and storage IDs can be confused] -> Validate that the explicit stored ID is Gym-prefixed and matches the normalized contest/index before persistence.
- [Source precedence could erase metadata absent from a detail response] -> Restrict precedence to selected non-empty title/content fields and retain the existing merge for all other fields.

## Migration Plan

No data or configuration migration is required. Deploy the coordinator and source-client changes together. Rollback consists of reverting those code changes; already enriched rows remain valid, and the atomic daily snapshot format is unchanged.

## Open Questions

None.
