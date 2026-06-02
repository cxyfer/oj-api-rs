## Why

Problem metadata discovery endpoints currently live outside the problem resource namespace, which makes the public API less consistent for clients browsing problem-related capabilities. Moving difficulty and tag discovery under `/api/v1/problems` groups problem list filters with the resource they support.

## What Changes

- **BREAKING** Move difficulty discovery from `GET /api/v1/difficulties/{source}` to `GET /api/v1/problems/difficulties/{source}`.
- **BREAKING** Move tag discovery from `GET /api/v1/tags/{source}` to `GET /api/v1/problems/tags/{source}`.
- Update OpenAPI path metadata so generated docs advertise only the new public API paths.
- Preserve existing response bodies, authentication requirements, source validation, and database behavior for both discovery handlers.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `problem-query`: Problem filter discoverability routes move under the `/api/v1/problems` namespace.
- `difficulty-list-api`: The public difficulty discovery endpoint path changes while preserving existing behavior.
- `openapi-spec-generation`: The generated public API inventory and handler annotations change to advertise the new endpoint paths.

## Impact

- Public API routes in `src/api/mod.rs`.
- Utoipa route path annotations in `src/api/problems.rs` and OpenAPI inventory registration in `src/api/openapi.rs` if needed.
- Existing tests for difficulty/tag discovery routes and any generated documentation snapshots or route inventory checks.
- No dependency, database schema, crawler, or authentication changes.
