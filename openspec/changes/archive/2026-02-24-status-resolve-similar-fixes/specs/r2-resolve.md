# Spec: R2 — Resolve LeetCode slug to ID

## Requirement
When resolving a LeetCode URL, translate the slug to the numeric problem ID via DB lookup.

## Scenarios

### S2.1: LeetCode URL with known slug
- **Given**: problems table has `(id="1", source="leetcode", slug="two-sum")`
- **When**: GET /api/v1/resolve/https://leetcode.com/problems/two-sum/
- **Then**: 200 with `{ source: "leetcode", id: "1", problem: { ... } }`

### S2.2: LeetCode URL with unknown slug
- **Given**: no problem with slug "nonexistent-slug"
- **When**: GET /api/v1/resolve/https://leetcode.com/problems/nonexistent-slug/
- **Then**: 200 with `{ source: "leetcode", id: "nonexistent-slug", problem: null }`

### S2.3: LeetCode numeric ID (unchanged)
- **Given**: problems table has `(id="1", source="leetcode")`
- **When**: GET /api/v1/resolve/1
- **Then**: 200 with `{ source: "leetcode", id: "1", problem: { ... } }`

### S2.4: Non-LeetCode platform (unchanged)
- **Given**: problems table has `(id="abc321_a", source="atcoder")`
- **When**: GET /api/v1/resolve/https://atcoder.jp/contests/abc321/tasks/abc321_a
- **Then**: 200 with `{ source: "atcoder", id: "abc321_a", problem: { ... } }`

### S2.5: Default fallback slug (detect.rs line 122)
- **Given**: text input "two-sum" (no URL prefix), detect_source returns ("leetcode", "two-sum")
- **When**: GET /api/v1/resolve/two-sum
- **Then**: slug lookup triggered (contains non-digit chars), resolved to numeric ID if found

## PBT Properties

### P2.1: Non-LeetCode passthrough
- For all source != "leetcode": resolve behavior is identical to before this change.

### P2.2: Numeric ID stability
- For LeetCode with all-digit ID: no slug lookup occurs, result identical to current behavior.

### P2.3: Slug resolution correctness
- If `get_problem_id_by_slug(pool, "leetcode", slug)` returns `Some(id)`, then `get_problem(pool, "leetcode", id)` must return `Some(problem)` where `problem.slug == slug`.

### P2.4: Case-insensitive slug matching
- `resolve("Two-Sum")` and `resolve("two-sum")` produce identical results (slug normalized via `.to_lowercase()` before DB query).
