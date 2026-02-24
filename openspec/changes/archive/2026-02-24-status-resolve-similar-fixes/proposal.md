# Proposal: status-resolve-similar-fixes

## Context

Three issues need to be addressed in the public API:

1. No `/status` endpoint exists to expose system version and per-platform statistics.
2. The `/api/v1/resolve/*` endpoint extracts a LeetCode slug from URLs but queries the DB using `(source, id)` — the slug is not the ID, so the lookup always returns `null`.
3. The `/api/v1/similar?q=...` endpoint rejects the `q` parameter because the `SimilarByTextQuery` struct uses field name `query`, and there is no serde alias for `q`.

## Requirements

### R1: `GET /status` endpoint

**Scenario**: User requests `GET /status` with a valid Bearer token (when auth is enabled).

**Constraints**:
- MUST be placed under the same `bearer_auth` middleware layer as the existing `/api/v1/*` routes — same toggle behavior (`TokenAuthEnabled` AtomicBool).
- MUST return JSON with:
  - `version`: `env!("CARGO_PKG_VERSION")` (compile-time, from Cargo.toml `0.1.0`)
  - `platforms`: array of objects, one per distinct `source` in `problems` table, each containing:
    - `source`: platform name
    - `total`: `COUNT(*)` for that source
    - `missing_content`: count where `content IS NULL OR content = ''`
    - `not_embedded`: count of problems without a matching row in `problem_embeddings`
- MUST use `ro_pool` via `spawn_blocking` (existing DB convention).
- Response MUST use `application/json` content-type (standard `Json(...)` response).
- Route path: `/status` (not under `/api/v1/` — top-level like `/health`), but protected by the same bearer auth middleware.

**Hard constraints**:
- The query MUST use `SELECT DISTINCT source FROM problems` to dynamically enumerate platforms — NOT a hardcoded list. This ensures platforms added via future crawlers appear automatically.
- A single SQL query with LEFT JOIN + GROUP BY is preferred over N+1 queries.

### R2: Resolve LeetCode slug to ID

**Scenario**: User requests `GET /api/v1/resolve/https://leetcode.com/problems/two-sum/`.

**Current flow** (`src/detect.rs:48-49`): `LEETCODE_URL_RE` captures `two-sum` → returned as `id` → `get_problem(&pool, "leetcode", "two-sum")` → no match (the actual ID is `"1"`).

**Constraints**:
- When `detect_source` returns `("leetcode", slug)` where slug is non-numeric (i.e., contains non-digit characters), the resolve handler MUST perform a DB lookup: `SELECT id FROM problems WHERE source = 'leetcode' AND slug = ?1`.
- If lookup succeeds, use the resolved numeric ID for `get_problem`.
- If lookup fails (no row), return `source: "leetcode"`, `id: slug` (the original slug), `problem: null` — same behavior as current "not found" case.
- If slug IS purely numeric, skip the lookup (it's already an ID).
- This fix is scoped to the resolve handler only (`src/api/resolve.rs`). The `detect_source` function itself MUST NOT be modified.

**Hard constraints**:
- DB lookup MUST be inside the existing `spawn_blocking` block.
- New DB function `get_problem_id_by_slug` in `src/db/problems.rs`.

### R3: Fix `similar_by_text` query parameter name

**Scenario**: User requests `GET /api/v1/similar?q=%22two-sum%22` (URL-decoded: `q="two-sum"`).

**Root cause**: `SimilarByTextQuery.query` field has no `#[serde(alias = "q")]` attribute. Axum's `Query` extractor deserializes by field name, so `?q=...` is silently ignored and `query` is `None`.

**Constraints**:
- Add `#[serde(alias = "q")]` to the `query` field in `SimilarByTextQuery` (`src/api/similar.rs:20`).
- After URL-decoding, if the value is wrapped in double quotes (`"two-sum"`), strip the surrounding quotes before processing. This ensures `%22two-sum%22` behaves the same as `two-sum`.
- Quote stripping MUST only remove a matching pair of outer double quotes — not inner quotes, not single quotes.

**Hard constraints**:
- The minimum length check (`q.len() >= 3`) MUST apply AFTER quote stripping, not before.

## Success Criteria

1. `GET /status` (with auth) returns 200 with `version` and per-platform stats matching actual DB content.
2. `GET /status` without token (when auth enabled) returns 401.
3. `GET /status` without token (when auth disabled) returns 200.
4. `GET /api/v1/resolve/https://leetcode.com/problems/two-sum/` returns `{"source":"leetcode","id":"1","problem":{...}}` (assuming problem exists in DB).
5. `GET /api/v1/resolve/https://leetcode.com/problems/nonexistent-slug/` returns `{"source":"leetcode","id":"nonexistent-slug","problem":null}`.
6. `GET /api/v1/resolve/leetcode:1` continues to work (numeric ID, no slug lookup).
7. `GET /api/v1/similar?q=two-sum` is accepted (alias works).
8. `GET /api/v1/similar?query=two-sum` still works (original name preserved).
9. `GET /api/v1/similar?q=%22two-sum%22` is accepted and treated as `two-sum` (quotes stripped).

## Dependencies

- R1 depends on no other change.
- R2 depends on no other change.
- R3 depends on no other change.
- All three can be implemented in parallel.

## Risks

- **R2**: If the `problems` table has no index on `(source, slug)`, the slug lookup will be a full table scan per LeetCode source. Mitigation: add `CREATE INDEX IF NOT EXISTS` on `(source, slug)` in `ensure_data_tables`.
- **R1**: If a platform has zero problems, it won't appear in the stats (since we enumerate from `problems` table). This is acceptable — no data means nothing to report.
