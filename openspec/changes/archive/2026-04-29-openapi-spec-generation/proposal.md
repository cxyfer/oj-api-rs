# Proposal: OpenAPI Spec Generation with Scalar UI

## Problem Statement

The oj-api-rs project has 10+ public API endpoints, 30+ admin API endpoints, plus /health and /status, but no machine-readable OpenAPI specification. Current documentation is hand-written HTML rendered via Askama templates at `/docs/api` and `/docs/mcp`. This prevents:

- Automated client SDK generation
- Postman/Insomnia collection import
- Contract testing
- Machine-readable API discovery

## Proposed Solution

Add runtime-generated OpenAPI 3.1 spec using `utoipa` 5.x macros, served as JSON at `/api/v1/openapi.json`, with interactive Scalar UI replacing the existing HTML docs at `/docs`.

### Technology Stack

| Component | Crate | Version | Purpose |
|-----------|-------|---------|---------|
| Spec generation | `utoipa` | 5.x | `#[derive(ToSchema)]`, `#[utoipa::path]` macros |
| Axum integration | `utoipa-axum` | 0.2.x | `OpenApiRouter` for typed route collection |
| UI | `utoipa-scalar` | 0.2.x | Scalar UI at `/docs` (replaces HTML docs) |

### Scope

**All endpoints** will be documented:

1. **Public API** (`/api/v1/*`): 10 endpoints — problems, tags, resolve, daily, similar, status
2. **Admin API** (`/admin/api/*`): ~30 endpoints — problem CRUD, tokens, crawlers, embeddings, settings
3. **Infrastructure**: `/health`, `/status`

### Architecture Changes

```
Before:
  /docs/api  → Askama HTML template (hand-written)
  /docs/mcp  → Askama HTML template (hand-written)

After:
  /docs          → Scalar UI (interactive API docs)
  /openapi.json  → OpenAPI 3.1 spec (auto-generated)
  /docs/mcp      → Keep as-is (MCP is not REST, not OpenAPI-appropriate)
```

## Discovered Constraints

### Hard Constraints

1. **utoipa 5.x required**: utoipa 4.x does not support axum 0.8. Must use utoipa 5.x + utoipa-axum 0.2.x
2. **Bearer auth is middleware-applied**: OpenAPI security scheme must be manually declared; cannot be inferred from `route_layer(from_fn(bearer_auth))`
3. **RFC 7807 error responses**: `application/problem+json` content type must be explicitly declared in each operation's error responses
4. **`impl IntoResponse` return types**: Cannot auto-infer response schemas; must manually annotate responses in `#[utoipa::path]`
5. **Generic wrappers**: `ListResponse<T>` and `BatchResponse<T>` need concrete schema aliases for OpenAPI
6. **Admin auth dual-mode**: Admin endpoints accept both `x-admin-secret` header and `oj_admin_session` cookie — both must be declared as security schemes

### Soft Constraints

7. **Existing `src/home.rs` DocsRegistry**: Route descriptions, curl examples, and field docs already exist and should be reused as OpenAPI descriptions where possible
8. **i18n**: Current docs use client-side i18n (en, zh-TW, zh-CN). OpenAPI spec will be English-primary; Scalar UI has its own i18n
9. **`pub(crate)` response types**: Most API DTOs are crate-internal, which is fine for in-crate utoipa macros but means schemas can't be exported externally

## Dependencies

| Dependency | Impact | Files Affected |
|-----------|--------|----------------|
| `utoipa` 5.x | New dep + `ToSchema` derives on ~15 structs | `Cargo.toml`, `src/models.rs`, `src/api/*.rs`, `src/api/error.rs`, `src/db/problems.rs` |
| `utoipa-axum` 0.2.x | New dep, optional `OpenApiRouter` usage | `Cargo.toml`, `src/api/mod.rs` |
| `utoipa-scalar` 0.2.x | New dep, mounts Scalar UI route | `Cargo.toml`, `src/main.rs` |
| `src/home.rs` | HTML docs replaced/removed, DocsRegistry may be deprecated | `src/home.rs`, `templates/docs_api.html`, `templates/docs_base.html` |
| `src/api/mod.rs` | Route registration may migrate to `OpenApiRouter` | `src/api/mod.rs` |

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Annotation boilerplate (~10 handlers + ~15 structs) | Medium | Reuse descriptions from existing DocsRegistry |
| Drift between code and spec | Low | utoipa generates spec from macros — spec is always in sync |
| Binary size increase (Scalar UI assets) | Low | ~200KB gzipped, acceptable for a server binary |
| Breaking existing `/docs/api` links | Medium | Add redirect from `/docs/api` → `/docs` |
| Admin endpoint exposure in spec | Low | Can group/hide admin endpoints behind auth in spec |

## Success Criteria

1. `GET /openapi.json` returns valid OpenAPI 3.1 JSON
2. `GET /docs` renders Scalar UI with all documented endpoints
3. All 10 public API endpoints appear with correct paths, methods, parameters, and response schemas
4. All admin API endpoints appear with correct auth requirements
5. Bearer token security scheme is declared and applied to public routes
6. Admin secret security scheme is declared and applied to admin routes
7. RFC 7807 error response schema is shared across all operations
8. `cargo build --release` compiles without errors
9. `cargo clippy` passes without new warnings
10. Existing `/docs/mcp` page remains functional

## User Confirmations

- **Scope**: All endpoints (public + admin + health + status)
- **UI**: Scalar (modern, dark-mode friendly)
- **Existing docs**: Replace HTML docs with Scalar UI; keep MCP docs
