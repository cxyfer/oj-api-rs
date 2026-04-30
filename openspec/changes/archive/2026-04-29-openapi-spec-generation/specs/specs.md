# Specification: OpenAPI Spec Generation with Scalar UI

## Overview

Add runtime-generated OpenAPI 3.1 spec to oj-api-rs using `utoipa` 5.x macros, served as JSON at `/openapi.json`, with interactive Scalar UI replacing existing HTML docs at `/docs`.

## Constraints

### Hard Constraints

1. **utoipa 5.x + utoipa-axum 0.2.x + utoipa-scalar 0.3.x** (NOT 0.2.x as originally proposed — upstream has moved to 0.3.x with axum integration)
2. **Spec endpoint `/openapi.json`** is public — NOT behind bearer auth middleware
3. **Scalar UI at `/docs`** replaces existing HTML API docs
4. **`/docs/mcp`** remains unchanged (Askama template, not OpenAPI-appropriate)
5. **All admin JSON API endpoints** (`/admin/api/*`) included in same spec, tagged as "Admin"
6. **Admin HTML page routes** (`/admin/login`, `/admin/`, `/admin/problems`, `/admin/tokens`, `/admin/crawlers`, `/admin/embeddings`) are EXCLUDED from OpenAPI spec
7. **RFC 7807 error responses** — `application/problem+json` content type must be explicitly declared per operation
8. **Bearer token security scheme** applied to public `/api/v1/*` routes
9. **Admin dual-mode security** — both `x-admin-secret` header AND `oj_admin_session` cookie as alternative security requirements
10. **`/docs/api` redirect** — permanent redirect to `/docs` for backward compatibility

### Soft Constraints

11. Reuse descriptions from existing `DocsRegistry` in `src/home.rs` where applicable
12. OpenAPI spec is English-primary; Scalar UI has its own i18n
13. `pub(crate)` response types are fine for in-crate utoipa macros

## Security Schemes

### Public API
- **Type**: HTTP Bearer
- **Scheme**: bearer
- **Applied to**: All `/api/v1/*` routes

### Admin API
- **Type A**: API Key in header (`x-admin-secret`)
- **Type B**: API Key in cookie (`oj_admin_session`)
- **Security requirement**: Alternative (either one satisfies auth)
- **Applied to**: All `/admin/api/*` routes

## Endpoint Inventory

### Infrastructure (no auth)
| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| GET | `/openapi.json` | OpenAPI 3.1 spec |
| GET | `/docs` | Scalar UI |

### Public API (bearer auth)
| Method | Path | Handler |
|--------|------|---------|
| GET | `/api/v1/problems/{source}/{id}` | `get_problem` |
| POST | `/api/v1/problems/batch` | `batch_problems` |
| GET | `/api/v1/problems/{source}` | `list_problems` |
| GET | `/api/v1/tags/{source}` | `list_tags` |
| GET | `/api/v1/resolve/{*query}` | `resolve` |
| GET | `/api/v1/daily` | `get_daily` |
| GET | `/api/v1/similar/{source}/{id}` | `similar_by_problem` |
| GET | `/api/v1/similar` | `similar_by_text` |
| GET | `/status` | `get_status` |

### Admin API (admin auth, JSON only)
| Method | Path | Handler |
|--------|------|---------|
| POST | `/admin/api/problems` | `create_problem` |
| GET | `/admin/api/problems/{source}` | `admin_list_problems` |
| GET | `/admin/api/tags/{source}` | `admin_list_tags` |
| GET | `/admin/api/problems/{source}/{id}` | `admin_get_problem` |
| PUT | `/admin/api/problems/{source}/{id}` | `update_problem` |
| DELETE | `/admin/api/problems/{source}/{id}` | `delete_problem` |
| GET | `/admin/api/tokens` | `list_tokens` |
| POST | `/admin/api/tokens` | `create_token` |
| DELETE | `/admin/api/tokens/{token}` | `delete_token` |
| GET | `/admin/api/settings/token-auth` | `get_token_auth_setting` |
| PUT | `/admin/api/settings/token-auth` | `toggle_token_auth` |
| POST | `/admin/api/crawlers/trigger` | `trigger_crawler` |
| POST | `/admin/api/crawlers/cancel` | `cancel_crawler` |
| GET | `/admin/api/crawlers/status` | `crawler_status` |
| GET | `/admin/api/crawlers/{job_id}/output` | `crawler_output` |
| GET | `/admin/api/crawlers/{job_id}/progress` | `crawler_progress` |
| GET | `/admin/api/embeddings/stats` | `embedding_stats` |
| POST | `/admin/api/embeddings/trigger` | `trigger_embedding` |
| POST | `/admin/api/embeddings/cancel` | `cancel_embedding` |
| GET | `/admin/api/embeddings/status` | `embedding_status` |
| GET | `/admin/api/embeddings/{job_id}/output` | `embedding_output` |
| GET | `/admin/api/embeddings/{job_id}/progress` | `embedding_progress` |

## Schema Inventory

### Core Domain Models (`src/models.rs`)
- `Problem` — full problem detail
- `ProblemSummary` — lightweight problem listing item
- `ProblemRecord` — DB record with custom deserializer
- `DailyChallenge` — daily challenge detail
- `DailyChallengeRecord` — DB record
- `ApiToken` — token metadata
- `CrawlerJob` — crawler job info
- `CrawlerStatus` — enum: Idle/Running/Completed/Failed
- `CrawlerTrigger` — enum: Luogu/LeetCode/Codeforces/...
- `JobType` — enum: Crawler/Embedding
- `JobArtifactMetadata` — artifact info
- `EmbeddingJob` — embedding job info
- `CrawlerPhase` — enum: phase tracking
- `CrawlerProgress` — progress detail
- `EmbeddingProgress` — progress detail

### API Response Types
- `ProblemDetailResponse` (`src/api/problems.rs`)
- `ListResponse<T>` → needs concrete aliases (e.g., `ProblemListResponse`, `TagListResponse`)
- `BatchResponse<T>` → needs concrete alias (`BatchProblemResponse`)
- `ResolveResponse` (`src/api/resolve.rs`)
- `DailyChallengeResponse` (`src/api/daily.rs`)
- `SimilarResponse`, `SimilarResult` (`src/api/similar.rs`)
- `StatusResponse` (`src/api/status.rs`)

### API Query/Path Parameters
- `ListQuery` — problems list filters
- `BatchQuery` — batch request params
- `DailyQuery` — daily challenge query (note: `r#async` field → wire name `async`)
- `SimilarByProblemQuery` — similar by problem ID
- `SimilarByTextQuery` — similar by text (note: `query` alias `q`)

### Error Types
- `ProblemDetail` — RFC 7807 error body
- `FieldError` — validation error detail

## Edge Cases

1. **Wildcard path** `/api/v1/resolve/{*query}` — axum catch-all does not map 1:1 to OpenAPI path syntax. Use `/{query}` with description noting it captures slashes.
2. **Conditional response schemas** — `batch_problems` returns `ProblemSummary[]` or `ProblemDetailResponse[]` based on `detail=true`. Use `oneOf` or document as separate response variants.
3. **202 Accepted responses** — `get_daily` returns 202 with `{status, retry_after}` when data is missing and crawler is triggered. Must be explicitly documented.
4. **Raw `serde_json::Value` responses** — `/health`, crawler status/output, embedding status/output return ad hoc JSON. Either stabilize as named structs or manually author schema fragments.
5. **Custom serde deserializers** — `Problem.tags` and `Problem.similar_questions` use custom deserializers for DB storage. OpenAPI schema should reflect the wire format (JSON arrays), not the DB format (JSON strings).
6. **Query parameter aliases** — `SimilarByTextQuery.query` accepts alias `q`. Document both in parameter description.
7. **CSV semantics** — `ListQuery.tags`, `SimilarByProblemQuery.source`, `SimilarByTextQuery.source` are comma-separated strings, not repeated params.
8. **Raw identifier** — `DailyQuery.r#async` maps to wire name `async`.

## PBT Properties

### Validity
- **INVARIANT**: Generated spec MUST be valid OpenAPI 3.1 JSON
- **FALSIFICATION**: Parse spec with `serde_json` → validate against OpenAPI 3.1 JSON Schema

### Completeness
- **INVARIANT**: Every registered route handler MUST have a corresponding path entry in the spec
- **FALSIFICATION**: Collect all routes from axum Router → compare against spec paths → report missing

### Schema Coverage
- **INVARIANT**: Every `ToSchema`-derived struct MUST appear in `components.schemas`
- **FALSIFICATION**: Collect all `ToSchema` types → compare against spec schemas → report missing

### Security Consistency
- **INVARIANT**: Security schemes in spec MUST match actual middleware behavior
- **FALSIFICATION**: For each route, verify its security requirement matches the middleware applied in code

### Response Correctness
- **INVARIANT**: Documented response content types MUST match actual handler return types
- **FALSIFICATION**: For RFC 7807 routes, verify `application/problem+json` is declared

### Determinism
- **INVARIANT**: Same source code → identical spec output (stable across builds)
- **FALSIFICATION**: Build twice, diff the JSON output

### Redirect Safety
- **INVARIANT**: `GET /docs/api` MUST return 301/308 redirect to `/docs`
- **FALSIFICATION**: HTTP request to `/docs/api` → verify redirect response
