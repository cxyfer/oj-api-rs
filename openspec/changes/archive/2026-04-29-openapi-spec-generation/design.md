# Design: OpenAPI Spec Generation with Scalar UI

## Architecture

### Before
```
/             → home::public_router() (Askama HTML)
/docs/api     → Askama HTML template (hand-written docs)
/docs/mcp     → Askama HTML template (MCP docs)
/api/v1/*     → api::public_router() (bearer auth)
/admin/*      → admin::admin_router() (admin auth)
/health       → health::health_check()
```

### After
```
/             → home::public_router() (Askama HTML, links updated)
/docs         → Scalar UI (interactive API docs, NEW)
/docs/api     → 301 redirect → /docs
/docs/mcp     → Askama HTML template (UNCHANGED)
/openapi.json → OpenAPI 3.1 JSON spec (NEW, public)
/api/v1/*     → api::public_router() (bearer auth, documented via utoipa)
/admin/api/*  → admin JSON API (admin auth, documented via utoipa)
/admin/*      → admin HTML pages (NOT documented)
/health       → health::health_check() (documented via utoipa)
```

## Module Structure

```
src/api/openapi.rs    ← NEW: OpenApi derive, tags, security schemes, spec assembly
src/api/mod.rs        ← MODIFIED: add OpenApiRouter for public routes
src/admin/mod.rs      ← MODIFIED: add OpenApiRouter for admin JSON routes
src/main.rs           ← MODIFIED: merge specs, mount /openapi.json and /docs
src/home.rs           ← MODIFIED: update links, redirect /docs/api
```

## Implementation Strategy

### Step 1: Add Dependencies
Add to `Cargo.toml`:
```toml
utoipa = { version = "5", features = ["axum_extras", "chrono", "uuid"] }
utoipa-axum = "0.2"
utoipa-scalar = { version = "0.3", features = ["axum"] }
```

### Step 2: Create `src/api/openapi.rs`
Central module for OpenAPI assembly:
- `#[derive(OpenApi)]` with security schemes, tags, info
- Bearer auth security scheme (HTTP Bearer)
- Admin auth security schemes (API Key header + cookie, alternative)
- Tags: "Problems", "Tags", "Resolve", "Daily", "Similar", "Status", "Health", "Admin"
- Helper type aliases for generic wrappers (`ListResponse<Problem>` → `ProblemListResponse`)

### Step 3: Add `ToSchema` Derives
Add `#[derive(utoipa::ToSchema)]` to all DTOs in inventory (see specs.md Schema Inventory).
For generic `ListResponse<T>` and `BatchResponse<T>`, create concrete type aliases:
```rust
type ProblemListResponse = ListResponse<ProblemSummary>;
type TagListResponse = ListResponse<String>;
type BatchProblemResponse = BatchResponse<ProblemDetailResponse>;
```

### Step 4: Annotate Handlers with `#[utoipa::path]`
Each handler gets a `#[utoipa::path]` attribute specifying:
- `method`, `path`, `operation_id`
- `params` (query/path parameters)
- `responses` (success + RFC 7807 errors)
- `security` (bearer or admin)
- `tag` (grouping)

Key annotations for edge cases:
- `batch_problems`: document both `detail=true` and `detail=false` response variants
- `get_daily`: document both 200 and 202 responses
- `resolve`: use `/{query}` path with description about slash capture
- `health`: document as public (no security)

### Step 5: Migrate to OpenApiRouter (Incremental)
**Public routes** (`src/api/mod.rs`):
- Wrap documented handlers with `OpenApiRouter::new().routes(routes!(...))`
- Call `split_for_parts()` to get `(Router, OpenApi)`
- Apply `bearer_auth` and `CorsLayer` on the returned `axum::Router`

**Admin routes** (`src/admin/mod.rs`):
- Create separate `OpenApiRouter` for `/admin/api/*` JSON endpoints only
- HTML page routes stay on plain `Router`
- Call `split_for_parts()` to get `(Router, OpenApi)`

### Step 6: Merge and Mount in `main.rs`
```rust
// In src/main.rs
let (api_router, api_docs) = api::openapi_parts();
let (admin_router, admin_docs) = admin::openapi_parts();

let mut openapi = api_docs;
openapi.merge(admin_docs);

// Mount public spec endpoint
let app = Router::new()
    .route("/openapi.json", get(|| async { Json(openapi) }))
    .merge(Scalar::new("/openapi.json").url("/docs", "/docs"))
    .merge(home::public_router())  // includes /docs/mcp redirect
    .merge(api_router)             // bearer-authed public API
    .merge(admin_router)           // admin-authed admin API
    .route("/health", get(health::health_check));
```

### Step 7: Update Home Links and Redirects
- Add 301 redirect from `/docs/api` → `/docs`
- Update homepage card links from `/docs/api` → `/docs`
- Keep `/docs/mcp` unchanged

### Step 8: Cleanup (Post-Verification)
After spec parity is verified:
- Remove REST-only entries from `DocsRegistry`
- Remove `templates/docs_api.html` (or repurpose for redirect)
- Keep `templates/docs_base.html` if still used by `/docs/mcp`

## Migration Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| `/openapi.json` behind bearer auth | Mount spec endpoint OUTSIDE the bearer-authed router |
| Admin auth not representable | Manually declare alternative security requirements |
| `impl IntoResponse` hides response types | Manually annotate every response variant in `#[utoipa::path]` |
| Generic wrappers need concrete schemas | Create type aliases for each concrete usage |
| Wildcard path syntax mismatch | Use `/{query}` with descriptive docs |
| `serde_json::Value` responses | Stabilize as named structs or manually author schemas |
| Stale `/docs/api` links | Add permanent redirect |
| Binary size increase | ~200KB gzipped for Scalar UI assets, acceptable |

## Files Modified

| File | Change Type | Description |
|------|------------|-------------|
| `Cargo.toml` | MODIFY | Add utoipa, utoipa-axum, utoipa-scalar deps |
| `src/api/openapi.rs` | CREATE | OpenApi derive, tags, security, assembly |
| `src/api/mod.rs` | MODIFY | Add OpenApiRouter for public routes |
| `src/api/problems.rs` | MODIFY | Add ToSchema, IntoParams, utoipa::path |
| `src/api/daily.rs` | MODIFY | Add ToSchema, IntoParams, utoipa::path |
| `src/api/resolve.rs` | MODIFY | Add ToSchema, utoipa::path |
| `src/api/similar.rs` | MODIFY | Add ToSchema, IntoParams, utoipa::path |
| `src/api/status.rs` | MODIFY | Add ToSchema, utoipa::path |
| `src/api/error.rs` | MODIFY | Add ToSchema to ProblemDetail, FieldError |
| `src/models.rs` | MODIFY | Add ToSchema to ~15 structs/enums |
| `src/admin/mod.rs` | MODIFY | Add OpenApiRouter for admin JSON routes |
| `src/admin/handlers.rs` | MODIFY | Add utoipa::path to ~19 admin handlers |
| `src/main.rs` | MODIFY | Merge specs, mount /openapi.json and /docs |
| `src/home.rs` | MODIFY | Update links, add /docs/api redirect |
| `src/health.rs` | MODIFY | Add utoipa::path |
| `templates/home.html` | MODIFY | Update docs link from /docs/api to /docs |
