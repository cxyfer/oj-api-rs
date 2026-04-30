# Tasks: OpenAPI Spec Generation with Scalar UI

## Phase 1: Foundation

- [x] 1.1 Add utoipa dependencies to Cargo.toml
- [x] 1.2 Create `src/api/openapi.rs` with OpenApi derive and security schemes
- [x] 1.3 Add `ToSchema` derives to core domain models

## Phase 2: API Response & Parameter Types

- [x] 2.1 Add `ToSchema` to API response types
- [x] 2.2 Create concrete type aliases for generic wrappers
- [x] 2.3 Add `IntoParams` to query/path parameter structs

## Phase 3: Handler Annotations (Public API)

- [x] 3.1 Annotate problems handlers
- [x] 3.2 Annotate resolve handler
- [x] 3.3 Annotate daily handler
- [x] 3.4 Annotate similar handlers
- [x] 3.5 Annotate status and health handlers

## Phase 4: Handler Annotations (Admin API)

- [x] 4.1 Annotate admin problem CRUD handlers
- [x] 4.2 Annotate admin token handlers
- [x] 4.3 Annotate admin settings handlers
- [x] 4.4 Annotate admin crawler handlers
- [x] 4.5 Annotate admin embedding handlers

## Phase 5: Router Migration

- [x] 5.1 Migrate public routes to OpenApiRouter
- [x] 5.2 Create admin OpenApiRouter for JSON endpoints

## Phase 6: Assembly & Mounting

- [x] 6.1 Merge specs and mount endpoints in main.rs
- [x] 6.2 Add redirect and update links

## Phase 7: Verification & Cleanup

- [x] 7.1 Verify spec validity and completeness
- [x] 7.2 Verify endpoint coverage
- [ ] 7.3 Cleanup old docs (post-verification)
  - Remove REST-only entries from `DocsRegistry` in `src/home.rs`
  - Remove or repurpose `templates/docs_api.html`
  - Keep `templates/docs_base.html` if still used by `/docs/mcp`
