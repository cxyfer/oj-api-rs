## Context

The public API currently exposes problem metadata discovery as top-level endpoints: `GET /api/v1/tags/{source}` and `GET /api/v1/difficulties/{source}`. Both handlers already live in `src/api/problems.rs`, use the same bearer-auth protected `/api/v1/*` router, validate source through `VALID_SOURCES`, and read from the existing problem database tables.

The requested change is a namespace relocation, not a behavior change: tag and difficulty discovery should be grouped under the problem resource namespace while keeping the existing source-specific response semantics.

## Goals / Non-Goals

**Goals:**

- Route tag discovery at `GET /api/v1/problems/tags/{source}`.
- Route difficulty discovery at `GET /api/v1/problems/difficulties/{source}`.
- Update utoipa path annotations and OpenAPI inventory so generated docs match the runtime router.
- Keep response bodies, auth, validation, database queries, and error behavior unchanged.
- Add or adjust tests so the new endpoint paths are exercised.

**Non-Goals:**

- Keep compatibility aliases for the old top-level paths.
- Change list filtering query parameters such as `difficulty` or `tags`.
- Change database schema, crawler output, authentication, or source registration.
- Introduce redirects for API clients.

## Decisions

- Use plural nested resource paths: `/api/v1/problems/tags/{source}` and `/api/v1/problems/difficulties/{source}`.
  - Rationale: this preserves the existing plural resource names and follows the current API style.
  - Alternative considered: `/api/v1/problems/tag/{source}`. Rejected because the current endpoint is plural and the response is a collection.

- Remove the old top-level route registrations instead of adding aliases.
  - Rationale: the proposal marks the relocation as a breaking API change and avoids maintaining duplicate public contracts.
  - Alternative considered: keep old routes as temporary compatibility aliases. Rejected because the requested outcome is to move the endpoints, and aliases would keep advertising two contracts.

- Reuse the existing handlers and DB functions.
  - Rationale: handler behavior is already correct; only the path binding and documentation change.
  - Alternative considered: create a separate nested metadata module. Rejected as unnecessary for a small route relocation.

- Register static nested metadata routes before dynamic problem routes if needed by axum route matching.
  - Rationale: the new paths overlap the shape of `/api/v1/problems/{source}/{id}` if `tags` or `difficulties` are interpreted as `{source}`. Keeping explicit static routes visible in the router reduces ambiguity during implementation review.
  - Alternative considered: move metadata to `/api/v1/problems/{source}/tags`; rejected because the requested examples place metadata directly under `/problems`.

## Risks / Trade-offs

- Old clients calling `/api/v1/tags/{source}` or `/api/v1/difficulties/{source}` will fail after deployment → Update OpenAPI docs and release notes to show the new paths clearly.
- Nested static paths may conflict with dynamic problem detail routes if route registration is incorrect → Run routing-focused tests and `cargo test` after implementation.
- Documentation can drift from runtime routes → Update both `#[utoipa::path]` annotations and OpenAPI capability specs in the same change.
