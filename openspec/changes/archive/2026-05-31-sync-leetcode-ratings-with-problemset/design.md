## Context

LeetCode crawler problemset sync currently builds problem rows from the public problem list and stores `rating = 0` as a placeholder. Existing rows are skipped by the default `update_problems()` path, so rerunning `--sync-problemset` or `--init` does not repair zero ratings. Ratings are only filled later when `get_problem()` is invoked by detail, daily, full, or missing-content flows.

The rating data comes from an external GitHub-hosted `ratings.txt` file. That source is useful metadata, but it is not required for the core problem list sync to succeed.

## Goals / Non-Goals

**Goals:**
- Make LeetCode problemset sync merge rating metadata for both new and existing LeetCode problems.
- Preserve richer detail fields already stored in the database, including content, tags, and similar questions.
- Keep rating refresh best-effort so external rating source failures do not fail problemset sync.
- Keep existing daily/detail lazy-rating behavior as a fallback for individual problems.

**Non-Goals:**
- Add a new public REST or MCP endpoint.
- Add a scheduler or recurring background rating refresh.
- Change rating sources or introduce a new external dependency.
- Rework non-LeetCode crawlers.

## Decisions

### Merge ratings during LeetCode problemset initialization

`LeetCodeClient.init_all_problems()` should fetch the problem list, attempt to fetch ratings, merge rating metadata by frontend problem id, then persist the merged rows.

Rationale: `--sync-problemset` and `--init` are the administrator-visible operations for refreshing LeetCode metadata. Ratings are metadata and should be refreshed in the same operation instead of relying on later detail reads.

Alternative considered: add a separate `--refresh-ratings` command. This would be precise, but it keeps `--sync-problemset` semantically incomplete and requires administrators to remember another maintenance action.

### Use metadata-safe upsert semantics for LeetCode problemset sync

The persistence path should update problemset-level metadata and rating fields on existing LeetCode rows while preserving detail-rich fields when incoming values are empty or placeholders. At minimum, existing `content`, `content_cn`, `tags`, and `similar_questions` must not be cleared by a problemset-only sync.

Rationale: the existing default `INSERT OR IGNORE` cannot repair stale ratings, while a broad force update risks replacing detail fields with placeholders from the problem list response.

Alternative considered: call `update_problems(..., force_update=True)`. This is simpler, but it is too broad for this flow because problemset rows do not carry full detail data.

### Treat rating fetch as best-effort

If `fetch_ratings()` fails or returns no data, problemset sync should still write the latest problem list metadata. Existing positive ratings should not be overwritten by zero or null placeholders.

Rationale: LeetCode problem discovery should not depend on a third-party rating mirror. The system should prefer partial freshness over total sync failure.

Alternative considered: fail the whole sync when ratings are unavailable. This would keep rating completeness stricter, but it makes crawler reliability worse and ties core problemset sync to an optional metadata source.

### Keep detail and daily flows as rating backstops

Existing `get_problem()` rating fill behavior should remain. If a rating was unavailable during problemset sync, detail or daily flows can still fill it later.

Rationale: this keeps current behavior compatible while improving the common sync path.

## Risks / Trade-offs

- Rating source format changes → Parse defensively, skip malformed rows, and continue syncing problem metadata.
- External rating source unavailable → Log a warning and avoid overwriting existing positive ratings with placeholders.
- Metadata-safe upsert misses a field that should refresh → Keep the upsert policy explicit and covered by tests for rating, contest, problem index, and detail preservation.
- Problem ids arrive as strings in LeetCode data and integers in ratings data → Normalize ids before merging.
- Sync duration increases due to one extra external fetch → Reuse the existing ratings fetch retry behavior and keep it best-effort.
