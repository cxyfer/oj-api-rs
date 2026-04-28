## Why

The public API only supports single-problem lookups (`GET /api/v1/problems/{source}/{id}`). Users and AI agents preparing problem sets need to fetch multiple problems in one round trip, which currently requires N sequential HTTP requests — slow and wasteful against rate limits.

## What Changes

- Add `POST /api/v1/problems/batch` endpoint accepting a JSON array of `{source, id}` objects (max 50).
- Default response returns `ProblemSummary` (12 fields) per problem.
- Optional `?detail=true` query param returns full `ProblemDetailResponse` with content, content_cn, and hydrated `similar_questions`.
- Response wraps results in `{ results: [...], not_found: [...] }` so missing items don't fail the entire request.
- Validation: empty array → 400, > 50 items → 400, invalid source → 400 (fail-fast).

## Capabilities

### New Capabilities
- `batch-problem-fetch`: Batch endpoint for fetching multiple problems by source+id pairs in a single POST request, with summary/detail response modes and not-found tracking.

### Modified Capabilities

## Impact

- **Files modified**: `src/api/problems.rs` (new handler, types, helper), `src/api/mod.rs` (route + post import), `src/home.rs` (API docs card + test count), `templates/home.html` (route count text), `static/i18n/{en,zh-TW,zh-CN}.json` (i18n keys), `README.md` (endpoint listing).
- **API surface**: New POST endpoint under existing `/api/v1/*` prefix, automatically covered by bearer auth middleware.
- **No schema changes**: Uses existing `ProblemRecord` and `ProblemSummary` types, reuses `get_problem_record` DB function in a loop.
- **No breaking changes**: Purely additive endpoint.
