# Spec: R1 — GET /status endpoint

## Requirement
Authenticated `/status` endpoint returning version + per-platform statistics.

## Scenarios

### S1.1: Authenticated request (auth enabled)
- **Given**: token_auth_enabled = true, valid Bearer token
- **When**: GET /status
- **Then**: 200 with `{ version: string, platforms: [...] }`

### S1.2: Unauthenticated request (auth enabled)
- **Given**: token_auth_enabled = true, no Bearer token
- **When**: GET /status
- **Then**: 401 ProblemDetail "missing or invalid token"

### S1.3: Request with auth disabled
- **Given**: token_auth_enabled = false
- **When**: GET /status (no token)
- **Then**: 200 with status data

### S1.4: Empty database
- **Given**: problems table has zero rows
- **When**: GET /status
- **Then**: 200 with `{ version: "...", platforms: [] }`

### S1.5: Platform stats accuracy
- **Given**: leetcode has 100 problems, 5 with NULL/empty content, 10 without problem_embeddings row
- **When**: GET /status
- **Then**: leetcode entry shows `total: 100, missing_content: 5, not_embedded: 10`

## PBT Properties

### P1.1: Sum invariant
- `sum(platforms[*].total)` == `SELECT COUNT(*) FROM problems`

### P1.2: Bound invariant
- For each platform: `missing_content <= total` AND `not_embedded <= total`

### P1.3: Idempotency
- Two consecutive GET /status with no DB writes between them return identical responses.

### P1.4: Version constancy
- `version` field always equals `env!("CARGO_PKG_VERSION")` at compile time.
