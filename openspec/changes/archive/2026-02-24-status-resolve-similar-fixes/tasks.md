# Tasks: status-resolve-similar-fixes

## T1: Add `(source, slug)` index to ensure_data_tables
- [x] **File**: `src/db/mod.rs`
- **Action**: In `ensure_data_tables`, append to the existing `execute_batch` string (before the closing `"`):
  ```sql
  CREATE INDEX IF NOT EXISTS idx_problems_source_slug ON problems(source, slug);
  ```
- **Verify**: `cargo build` succeeds.

## T2: Add `get_problem_id_by_slug` to db::problems
- [x] **File**: `src/db/problems.rs`
- **Action**: Add function after `get_problem`:
  ```rust
  pub fn get_problem_id_by_slug(pool: &DbPool, source: &str, slug: &str) -> Option<String> {
      let conn = pool.get().ok()?;
      conn.query_row(
          "SELECT id FROM problems WHERE source = ?1 AND slug = ?2 LIMIT 1",
          params![source, slug],
          |row| row.get(0),
      )
      .ok()
  }
  ```
- **Verify**: `cargo build` succeeds.

## T3: Add `platform_stats` to db::problems
- [x] **File**: `src/db/problems.rs`
- **Action**: Add `use serde::Serialize;` to imports. Add struct and function:
  ```rust
  #[derive(Debug, Serialize)]
  pub struct PlatformStats {
      pub source: String,
      pub total: u32,
      pub missing_content: u32,
      pub not_embedded: u32,
  }

  pub fn platform_stats(pool: &DbPool) -> Vec<PlatformStats> {
      let conn = match pool.get() {
          Ok(c) => c,
          Err(_) => return Vec::new(),
      };
      let mut stmt = match conn.prepare(
          "SELECT p.source, COUNT(*) AS total, \
           SUM(CASE WHEN p.content IS NULL OR p.content = '' THEN 1 ELSE 0 END) AS missing_content, \
           SUM(CASE WHEN pe.problem_id IS NULL THEN 1 ELSE 0 END) AS not_embedded \
           FROM problems p \
           LEFT JOIN problem_embeddings pe ON pe.source = p.source AND pe.problem_id = p.id \
           GROUP BY p.source ORDER BY p.source",
      ) {
          Ok(s) => s,
          Err(_) => return Vec::new(),
      };
      let rows = match stmt.query_map([], |row| {
          Ok(PlatformStats {
              source: row.get(0)?,
              total: row.get(1)?,
              missing_content: row.get(2)?,
              not_embedded: row.get(3)?,
          })
      }) {
          Ok(r) => r,
          Err(_) => return Vec::new(),
      };
      rows.filter_map(|r| r.ok()).collect()
  }
  ```
- **Verify**: `cargo build` succeeds.

## T4: Create `src/api/status.rs` handler
- [x] **File**: `src/api/status.rs` (new)
- **Action**: Create handler:
  ```rust
  use std::sync::Arc;

  use axum::extract::State;
  use axum::response::IntoResponse;
  use axum::Json;
  use serde::Serialize;

  use crate::AppState;

  #[derive(Serialize)]
  struct StatusResponse {
      version: &'static str,
      platforms: Vec<crate::db::problems::PlatformStats>,
  }

  pub async fn get_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
      let pool = state.ro_pool.clone();
      let platforms = tokio::task::spawn_blocking(move || {
          crate::db::problems::platform_stats(&pool)
      })
      .await
      .unwrap_or_default();

      Json(StatusResponse {
          version: env!("CARGO_PKG_VERSION"),
          platforms,
      })
  }
  ```
- **Verify**: `cargo build` succeeds.

## T5: Register `/status` route in public_router
- [x] **File**: `src/api/mod.rs`
- **Action**:
  1. Add `pub mod status;` to module declarations (after `pub mod similar;`).
  2. Add `.route("/status", get(status::get_status))` in the Router chain (before `.route_layer`).
- **Verify**: `cargo build` succeeds.

## T6: Fix resolve handler for LeetCode slug lookup
- [x] **File**: `src/api/resolve.rs`
- **Action**: Replace the `spawn_blocking` block. After `detect_source` returns `(source, id)`:
  1. Determine `effective_id` inside the `spawn_blocking` closure:
     - If `source == "leetcode"` and `id` contains any non-digit char:
       - Normalize: `let slug = id.to_lowercase();`
       - Call `get_problem_id_by_slug(&pool, "leetcode", &slug)`.
       - If `Some(resolved_id)` → `effective_id = resolved_id`.
       - If `None` → `effective_id = slug` (original slug, lowercased).
     - Else → `effective_id = id` (numeric or non-leetcode, unchanged).
  2. Call `get_problem(&pool, &source_str, &effective_id)`.
  3. Return both `effective_id` and `problem` from the closure.
  4. Build `ResolveResponse { source, id: effective_id, problem }`.
- **Verify**: `cargo build` succeeds.

## T7: Fix similar_by_text query parameter alias + quote stripping
- [x] **File**: `src/api/similar.rs`
- **Action**:
  1. Add `#[serde(alias = "q")]` to `SimilarByTextQuery.query` field (line 20).
  2. In `similar_by_text` handler, replace the current `let text = match &query.query { ... };` block with:
     ```rust
     let processed = query.query.as_deref().map(|q| {
         let trimmed = q.trim();
         if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
             &trimmed[1..trimmed.len() - 1]
         } else {
             trimmed
         }
     });

     let text = match processed {
         Some(q) if q.is_empty() => {
             return ProblemDetail::bad_request("query parameter is required").into_response();
         }
         Some(q) if q.len() > 2000 => {
             return ProblemDetail::bad_request("query must be at most 2000 characters")
                 .into_response();
         }
         Some(q) if q.len() < 3 => {
             return ProblemDetail::bad_request("query must be at least 3 characters")
                 .into_response();
         }
         Some(q) => q.to_string(),
         None => {
             return ProblemDetail::bad_request("query parameter is required").into_response();
         }
     };
     ```
- **Verify**: `cargo build` succeeds. `?q=two-sum` accepted. `?q=%22ab%22` returns 400.

## Resolved Constraints (from multi-model analysis)

| Decision | Resolution |
|----------|-----------|
| R1 `not_embedded` SQL | `SUM(CASE WHEN pe.problem_id IS NULL THEN 1 ELSE 0 END)` — avoids overcount risk |
| R1 spawn_blocking error | `unwrap_or_default()` — follows existing codebase pattern |
| R1 stats type | `u32` — sufficient for problem counts |
| R2 slug normalization | `.to_lowercase()` before DB query |

## Task Dependencies
```
T1 ──┐
T2 ──┤── T6 (resolve fix needs slug lookup fn + index)
T3 ──┤── T4 ── T5 (status needs stats fn, then handler, then route)
T7 (independent, no dependencies)
```

## Implementation Order
1. T1, T2, T3, T7 (parallel — independent changes)
2. T4 (depends on T3)
3. T5 (depends on T4)
4. T6 (depends on T1, T2)
