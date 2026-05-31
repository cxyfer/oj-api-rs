## Why

LeetCode problemset sync currently stores new problems with `rating = 0` and does not revisit existing zero ratings unless a later detail-oriented path calls `get_problem()`. This leaves list, random, search, and MCP responses with stale or missing rating metadata even after administrators run the problemset sync.

## What Changes

- Update LeetCode problemset synchronization so ratings metadata is fetched and merged as part of the sync flow.
- Preserve existing detail-rich fields such as content, tags, and similar questions when refreshing problemset metadata.
- Treat the external rating source as best-effort: rating fetch failures must not fail the overall problemset sync.
- Keep daily/detail flows as fallback paths that can still fill missing ratings for individual problems.
- No breaking API changes.

## Capabilities

### New Capabilities

### Modified Capabilities
- `crawler-cli`: LeetCode problemset sync must refresh rating metadata for new and existing problems without overwriting richer detail data.

## Impact

- Affected code: `scripts/leetcode.py`, `scripts/utils/database.py`, and related crawler tests.
- Affected behavior: admin-triggered LeetCode `--sync-problemset` / `--init` runs will update ratings when the rating source is available.
- Affected data: `problems.rating`, and rating-adjacent metadata such as `contest` and `problem_index` for LeetCode rows.
- No public REST, MCP, authentication, or configuration interface changes are expected.
