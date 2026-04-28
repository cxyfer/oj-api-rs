## Context

The public API serves competitive programming problems from 5 platforms via `GET /api/v1/problems/{source}/{id}`. All DB calls are synchronous (rusqlite + r2d2), bridged to async via `tokio::task::spawn_blocking`. The existing `get_problem_record` function does a single primary-key lookup. There is no batch-fetch pattern anywhere in the codebase.

## Goals / Non-Goals

**Goals:**
- Allow fetching up to 50 problems in one HTTP request
- Provide two response modes: summary (lightweight) and detail (full content + hydrated similar_questions)
- Track not-found items separately so partial failures don't block the entire response
- Follow every existing code pattern (error handling, DB access, auth, docs)

**Non-Goals:**
- No new DB table or schema changes
- No cross-source deduplication or linking
- No streaming/chunked responses for very large batches
- No caching layer (separate concern)

## Decisions

### 1. POST with JSON body (not GET with query params)

**Choice**: `POST /api/v1/problems/batch` with `[{source, id}, ...]` body.

**Why**: GET requests with bodies are non-standard and poorly supported by HTTP clients. A batch of 50 items exceeds reasonable query-string length. POST is the standard for non-idempotent operations with request bodies.

**Alternative considered**: `GET /api/v1/problems/batch?ids=leetcode:1,codeforces:1A` — rejected because comma-separated compound keys are error-prone and don't scale to 50 items.

### 2. Single spawn_blocking with loop (not new DB function)

**Choice**: Loop over `get_problem_record` N times in one `spawn_blocking` closure on one connection.

**Why**: With max 50 items and SQLite sub-ms primary-key lookups, total latency is < 5ms. Adding a batch SQL query (e.g., `WHERE (source, id) IN (...)`) increases complexity without meaningful performance gain. One connection means no pool exhaustion risk.

**Alternative considered**: Group by source, use `json_each` IN clause per source — rejected as over-engineering for 50 items.

### 3. Two response types via query param (not generic enum)

**Choice**: `?detail=true` returns `ProblemDetailResponse` (with content + similar_questions hydration). Default returns `ProblemSummary` (12 lightweight fields). Both wrapped in `BatchResponse<T>`.

**Why**: Summary mode avoids the cost of resolving similar_questions (extra DB query per problem). The `?detail=true` flag mirrors the existing pattern of query-param-driven response shape.

### 4. Fail-fast validation before DB access

**Choice**: Validate empty array, max size, and all sources before entering `spawn_blocking`.

**Why**: Avoids wasting a DB connection on an obviously invalid request. Consistent with existing handler patterns (e.g., `list_problems` validates `sort_by` before DB call).

## Risks / Trade-offs

- **[Sequential queries]** → 50 individual lookups vs. one batch query. Mitigation: SQLite primary-key lookups are sub-ms; 50 × 1ms = 50ms is acceptable. Can optimize later with a batch SQL function if needed.
- **[No caching]** → Repeated batch requests for the same problems hit DB each time. Mitigation: Can add caching layer separately without changing the endpoint contract.
- **[Duplicate items in request]** → Same (source, id) appears twice → both appear in results. Mitigation: Documented behavior; callers can deduplicate client-side.
