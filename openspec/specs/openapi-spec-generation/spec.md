# Specification: OpenAPI Spec Generation with Scalar UI

## Purpose
Define the runtime-generated OpenAPI 3.1 contract for `oj-api-rs`, including the public JSON spec at `/openapi.json`, the interactive docs at `/docs`, and the legacy redirect from `/docs/api`.

## Constraints

### Hard Constraints

1. **utoipa 5.x + utoipa-scalar 0.3.x** are used for spec generation and UI integration.
2. **`/openapi.json`** is public and MUST NOT require bearer auth.
3. **`/docs`** is the interactive API docs surface.
4. **`/docs/api`** MUST permanently redirect to `/docs` for backward compatibility.
5. **`/docs/mcp`** remains unchanged and is not part of the OpenAPI UI.
6. **All public `/api/v1/*` routes** and **all `/admin/api/*` JSON routes** are included in the generated spec.
7. **Admin HTML routes** under `/admin/*` are excluded from the spec.
8. **RFC 7807 error responses** MUST declare `application/problem+json` explicitly where applicable.
9. **Bearer auth** MUST be declared for public `/api/v1/*` routes.
10. **Admin auth** MUST support both `x-admin-secret` header and `oj_admin_session` cookie as alternative security requirements.

### Soft Constraints

11. Reuse descriptions from `src/home.rs` docs metadata where practical.
12. Keep the OpenAPI document English-first; localized prose belongs to the UI layer.
13. Concrete `ToSchema` aliases are acceptable for generic wrapper responses.

## Security Schemes

### Public API
- HTTP Bearer security scheme
- Applied to `/api/v1/*`

### Admin API
- API key in header (`x-admin-secret`)
- API key in cookie (`oj_admin_session`)
- Either scheme satisfies auth for `/admin/api/*`

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
| GET | `/api/v1/random` | `random_problems` |
| GET | `/api/v1/tags/{source}` | `list_tags` |
| GET | `/api/v1/resolve/{*query}` | `resolve` |
| GET | `/api/v1/daily` | `get_daily` |
| GET | `/api/v1/similar/{source}/{id}` | `similar_by_problem` |
| POST | `/api/v1/similar` | `similar_by_text` |
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

### Core Domain Models
- `Problem`
- `ProblemSummary`
- `ProblemRecord`
- `DailyChallenge`
- `DailyChallengeRecord`
- `ApiToken`
- `CrawlerJob`
- `CrawlerStatus`
- `CrawlerTrigger`
- `JobType`
- `JobArtifactMetadata`
- `EmbeddingJob`
- `CrawlerPhase`
- `CrawlerProgress`
- `EmbeddingProgress`

### API Response Types
- `ProblemDetailResponse`
- `ListResponse<T>` concrete aliases such as `ProblemListResponse` and `TagListResponse`
- `BatchResponse<T>` concrete alias such as `BatchProblemResponse`
- `ResolveResponse`
- `DailyChallengeResponse`
- `SimilarResponse`, `SimilarResult`
- `StatusResponse`

### API Query/Path Parameters
- `ListQuery`
- `BatchQuery`
- `DailyQuery`
- `SimilarByProblemQuery`
- `SimilarByTextQuery`

### Error Types
- `ProblemDetail`
- `FieldError`

## Edge Cases

1. **Wildcard path** `/api/v1/resolve/{*query}` should be represented in OpenAPI using a compatible path description.
2. **Conditional response schemas** for batch fetch should document both summary and detail modes.
3. **202 Accepted** from `get_daily` when the crawler fallback is triggered must be documented.
4. **Ad hoc JSON responses** for health/status/output endpoints may need explicit schema fragments.
5. **Custom deserializers** should not leak DB storage format into the wire schema.
6. **Query aliases** such as `SimilarByTextQuery.query` → `q` should be documented.
7. **Comma-separated source filters** should be described as CSV semantics.
8. **Raw identifier fields** such as `DailyQuery.r#async` should be documented with wire name `async`.

## Verification Properties

### Validity
- Generated output MUST be valid OpenAPI 3.1 JSON.

### Completeness
- Every registered route handler MUST have a corresponding path entry in the spec.

### Schema Coverage
- Every `ToSchema`-derived struct SHOULD appear in `components.schemas`.

### Security Consistency
- Security schemes in the spec MUST match the actual middleware behavior.

### Response Correctness
- RFC 7807 routes MUST declare `application/problem+json`.

### Determinism
- The same source code MUST produce identical spec output across builds.

### Redirect Safety
- `GET /docs/api` MUST return a permanent redirect to `/docs`.
