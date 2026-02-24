# Design: status-resolve-similar-fixes

## D1: GET /status endpoint

### Route placement
- Add `.route("/status", get(status::get_status))` inside `api::public_router()` in `src/api/mod.rs`.
- This inherits `bearer_auth` route_layer and CORS from the existing public router.
- New handler module: `src/api/status.rs`, declared as `pub mod status;` in `src/api/mod.rs`.

### Handler implementation
- Extract `State(state): State<Arc<AppState>>`.
- Use `state.ro_pool` via `spawn_blocking`.
- Call `db::problems::platform_stats(&pool)` returning `Vec<PlatformStats>`.
- Return `Json(StatusResponse { version, platforms })`.

### DB query (single query)
```sql
SELECT
    p.source,
    COUNT(*) AS total,
    SUM(CASE WHEN p.content IS NULL OR p.content = '' THEN 1 ELSE 0 END) AS missing_content,
    SUM(CASE WHEN pe.problem_id IS NULL THEN 1 ELSE 0 END) AS not_embedded
FROM problems p
LEFT JOIN problem_embeddings pe
    ON pe.source = p.source AND pe.problem_id = p.id
GROUP BY p.source
ORDER BY p.source
```
Note: `not_embedded` uses `SUM(CASE WHEN pe.problem_id IS NULL ...)` instead of `COUNT(*) - COUNT(pe.problem_id)` to avoid overcount if problem_embeddings ever has duplicate rows.

### Response structure
```json
{
  "version": "0.1.0",
  "platforms": [
    { "source": "leetcode", "total": 3200, "missing_content": 12, "not_embedded": 45 },
    { "source": "codeforces", "total": 8500, "missing_content": 0, "not_embedded": 8500 }
  ]
}
```

## D2: Resolve LeetCode slug to ID

### Scope
- Only `src/api/resolve.rs` handler logic changes. `detect.rs` remains untouched.
- Only LeetCode source is affected. Other platforms pass through unchanged.

### Logic
After `detect_source` returns `(source, id)`:
1. If `source != "leetcode"` → proceed as-is.
2. If `source == "leetcode"` and `id` contains any non-digit character → slug lookup path.
3. If `source == "leetcode"` and `id` is all digits → direct ID path (current behavior).

### Slug lookup
- New function `db::problems::get_problem_id_by_slug(pool, source, slug) -> Option<String>`.
- SQL: `SELECT id FROM problems WHERE source = ?1 AND slug = ?2 LIMIT 1`.
- Slug is normalized via `.to_lowercase()` before query to ensure case-insensitive matching.
- If found → use resolved ID for `get_problem`.
- If not found → return response with `id: slug, problem: null`.

### Index
- Add `CREATE INDEX IF NOT EXISTS idx_problems_source_slug ON problems(source, slug)` in `ensure_data_tables`.

### Response change
- `ResolveResponse.id` will now contain the resolved numeric ID when slug lookup succeeds, instead of the slug. This is the correct behavior since `id` should represent the problem's actual identifier.

## D3: Fix similar_by_text query parameter

### Serde alias
- Add `#[serde(alias = "q")]` to `SimilarByTextQuery.query` field.
- Both `?query=...` and `?q=...` will work. Existing consumers unaffected.

### Quote stripping
- In `similar_by_text` handler, after extracting `query.query`:
  1. Trim whitespace.
  2. If value starts with `"` and ends with `"` and length >= 2, strip the outer quotes.
  3. Apply length validation (>= 3, <= 2000) AFTER stripping.

### Implementation location
- Quote stripping in the handler body, not in serde deserialization. Keeps deserialization predictable.
