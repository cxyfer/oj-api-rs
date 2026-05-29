## Why

Users can currently list problems by source with filters (`GET /api/v1/problems/{source}`), but there is no way to obtain a random sampling of problems — especially across all platforms at once. This makes it difficult to build practice sessions, mixed-platform quizzes, or simple "surprise me" discovery flows.

## What Changes

- Add `GET /api/v1/random` endpoint that returns a random set of problems with optional filtering (source, difficulty, tags, tag_mode, rating_min, rating_max, count).
- Add `src/db/random.rs` — DB query logic including cross-platform difficulty mapping (easy/medium/hard → per-platform conditions via difficulty column or rating fallback).
- Add `src/api/random.rs` — handler with parameter validation, utoipa documentation, and response construction via the existing `build_problem_detail_response`.
- Wire the new endpoint into `src/api/mod.rs` (route registration) and `src/db/mod.rs` (module declaration).

## Capabilities

### New Capabilities

- `random-problem-endpoint`: Public API endpoint for retrieving random problems with optional filtering by source, difficulty, tags, tag_mode, rating_min, rating_max, and count.

### Modified Capabilities

None. This is a purely additive change that does not alter any existing API behavior.

## Impact

- **Affected source files** (new): `src/db/random.rs`, `src/api/random.rs`
- **Affected source files** (modified): `src/db/mod.rs`, `src/api/mod.rs`
- **API surface**: New public endpoint `GET /api/v1/random` (Bearer token auth, CORS, same middleware stack as existing `/api/v1/*` routes)
- **Database**: Read-only queries via `spawn_blocking`, no schema changes
- **Dependencies**: None (no new crates)
- **Breaking changes**: None
