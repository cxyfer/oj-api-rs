# authentication Specification

## Purpose
TBD - created by archiving change oj-api-rs-v1. Update Purpose after archive.
## Requirements
### Requirement: Bearer token authentication for public API
All `GET /api/v1/*` endpoints SHALL require a valid, active Bearer token in the `Authorization` header. The `/health` endpoint SHALL NOT require authentication.

#### Scenario: Valid active token
- **WHEN** client sends a request with `Authorization: Bearer <valid_active_token>`
- **THEN** system processes the request and returns 2xx

#### Scenario: Missing Authorization header
- **WHEN** client sends a request to `/api/v1/problems/leetcode/1` without Authorization header
- **THEN** system returns HTTP 401 with RFC 7807 error body

#### Scenario: Invalid token format
- **WHEN** client sends `Authorization: Bearer invalid_random_string`
- **THEN** system returns HTTP 401

#### Scenario: Inactive token
- **WHEN** client sends a valid token that has `is_active = 0` in the database
- **THEN** system returns HTTP 401

#### Scenario: Empty Bearer value
- **WHEN** client sends `Authorization: Bearer ` (empty value)
- **THEN** system returns HTTP 401

#### Scenario: Health check without auth
- **WHEN** client sends `GET /health` without any authentication
- **THEN** system returns HTTP 200

### Requirement: Admin secret authentication
All `/admin/*` endpoints SHALL require the `X-Admin-Secret` header matching the configured `ADMIN_SECRET`. Admin endpoints SHALL NOT accept Bearer tokens as an alternative.

#### Scenario: Valid admin secret
- **WHEN** client sends `X-Admin-Secret: <correct_secret>` to an admin endpoint
- **THEN** system processes the request

#### Scenario: Missing admin secret
- **WHEN** client sends a request to `/admin/api/tokens` without X-Admin-Secret header
- **THEN** system returns HTTP 401

#### Scenario: Wrong admin secret
- **WHEN** client sends `X-Admin-Secret: wrong_value` to an admin endpoint
- **THEN** system returns HTTP 401

#### Scenario: Bearer token on admin route
- **WHEN** client sends only `Authorization: Bearer <valid_token>` to an admin endpoint (no X-Admin-Secret)
- **THEN** system returns HTTP 401

### Requirement: ADMIN_SECRET mandatory at startup
The system SHALL refuse to start if `ADMIN_SECRET` environment variable is not set.

#### Scenario: Missing ADMIN_SECRET
- **WHEN** the application starts without `ADMIN_SECRET` env var
- **THEN** system logs an error and exits with non-zero code

### Requirement: Token last_used_at tracking
The system SHALL update `last_used_at` for a token upon each successful authentication.

#### Scenario: Token usage tracking
- **WHEN** a valid token is used for an API request
- **THEN** the token's `last_used_at` field is updated to the current Unix timestamp

