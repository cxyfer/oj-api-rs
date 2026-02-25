# Specs: dual-progress-and-similar-redesign

## R1: Dual Progress Bar

### SPEC-R1-01: Both bars render simultaneously during pipeline
- **Given** phase is `rewriting` or `embedding`
- **And** `rewrite_progress.total > 0`
- **Then** rewriting progress bar is visible with label `Rewriting: {done}/{total} (Skipped: {skipped})`
- **And** if `embed_progress.total > 0`, embedding progress bar is also visible

### SPEC-R1-02: Percentage clamped to [0, 100]
- **Invariant**: `0 <= displayed_percentage <= 100` for both bars
- **Boundary**: `total == 0` → percentage = 0, bar not rendered

### SPEC-R1-03: Terminal phases show status only
- **Given** phase is `completed` or `failed`
- **Then** only status label is rendered (no progress bars)

### SPEC-R1-04: i18n keys exist for skipped
- **Invariant**: `embeddings.progress.skipped` key exists in en, zh-TW, zh-CN

### PBT Properties
- **Idempotency**: Calling `updateEmbedProgressBar(prog)` twice with same data produces identical DOM
- **Monotonicity**: `done <= total` always holds (trusted from backend)
- **Bounds**: Percentage never exceeds 100% even if `done > total` (defensive clamp)

## R2: Similar API Response Wrapper

### SPEC-R2-01: Response is object, not array
- **Invariant**: `GET /api/v1/similar?q=...` returns `{ rewritten_query, results }` (object)
- **Invariant**: `GET /api/v1/similar/{source}/{id}` returns `{ rewritten_query, results }` (object)
- **Falsification**: Response JSON starts with `[` instead of `{`

### SPEC-R2-02: rewritten_query from text query
- **Given** valid text query
- **When** Python returns `{"embedding": [...], "rewritten": "some text"}`
- **Then** response contains `rewritten_query: "some text"`
- **When** Python returns `rewritten` as null/empty
- **Then** response contains `rewritten_query: null`

### SPEC-R2-03: rewritten_query from problem query
- **Given** valid problem source/id with embedding
- **When** `problem_embeddings.rewritten_content` exists and non-blank
- **Then** response contains `rewritten_query: "<content>"`
- **When** no row or NULL/blank
- **Then** response contains `rewritten_query: null`

### SPEC-R2-04: Empty results still wrapped
- **Given** query returns no similar problems
- **Then** response is `{ rewritten_query: "...", results: [] }` (not bare `[]`)

### SPEC-R2-05: Existing error responses unchanged
- **Invariant**: 400/404/502/504 error responses remain RFC 7807 format
- **Falsification**: Error response returns `{ rewritten_query, results }` wrapper

### PBT Properties
- **Round-trip**: `response.results` contains same fields as old array format (source, id, title, difficulty, link, similarity)
- **Idempotency**: Same query with same DB state returns identical response
- **Bounds**: `results.len() <= limit` (max 50)
- **Invariant Preservation**: `rewritten_query` is either `null` or non-empty string (never empty string `""`)
