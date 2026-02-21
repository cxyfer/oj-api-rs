# admin-api Specification

## Purpose
TBD - created by archiving change admin-i18n-and-problems-browse. Update Purpose after archive.
## Requirements
### Requirement: Admin Problems List Endpoint
The system SHALL provide an admin-scoped API endpoint `GET /admin/api/problems/{source}` that returns paginated problem summaries with session authentication.

#### Scenario: Successful list request
- **WHEN** authenticated admin requests `GET /admin/api/problems/leetcode?page=1&per_page=20`
- **THEN** the system SHALL return `200 OK` with body `{data: [ProblemSummary], meta: {page: 1, per_page: 20, total: N, total_pages: M}}`

#### Scenario: List request with difficulty filter
- **WHEN** authenticated admin requests `GET /admin/api/problems/leetcode?difficulty=hard`
- **THEN** the system SHALL return only problems where `difficulty = "hard"`

#### Scenario: List request with tags filter
- **WHEN** authenticated admin requests `GET /admin/api/problems/leetcode?tags=array,dp`
- **THEN** the system SHALL return only problems that have ALL specified tags

#### Scenario: Invalid source parameter
- **WHEN** admin requests `GET /admin/api/problems/invalid-source`
- **THEN** the system SHALL return `400 Bad Request` with RFC7807 Problem JSON: `{type: "about:blank", title: "Invalid Source", status: 400, detail: "Source must be one of: atcoder, leetcode, codeforces"}`

#### Scenario: Unauthenticated list request
- **WHEN** request is made without valid admin session cookie
- **THEN** the system SHALL return `401 Unauthorized` with RFC7807 Problem JSON

#### Scenario: Database error during list fetch
- **WHEN** database query fails or times out
- **THEN** the system SHALL return `500 Internal Server Error` with RFC7807 Problem JSON and SHALL NOT expose internal error details

### Requirement: Admin Problem Detail Endpoint
The system SHALL provide an admin-scoped API endpoint `GET /admin/api/problems/{source}/{id}` that returns a single problem's details with session authentication.

#### Scenario: Successful detail request
- **WHEN** authenticated admin requests `GET /admin/api/problems/leetcode/1`
- **THEN** the system SHALL return `200 OK` with a Problem object excluding `content` and `content_cn` fields

#### Scenario: Detail response excludes content fields
- **WHEN** the system fetches a problem from the database that has non-empty `content` and `content_cn`
- **THEN** the API response SHALL NOT include `content` or `content_cn` fields in the JSON

#### Scenario: Problem not found
- **WHEN** admin requests a problem ID that does not exist for the given source
- **THEN** the system SHALL return `404 Not Found` with RFC7807 Problem JSON: `{type: "about:blank", title: "Problem Not Found", status: 404, detail: "Problem with id X not found in source Y"}`

#### Scenario: Invalid source parameter
- **WHEN** admin requests `GET /admin/api/problems/invalid-source/123`
- **THEN** the system SHALL return `400 Bad Request` with RFC7807 Problem JSON

#### Scenario: Unauthenticated detail request
- **WHEN** request is made without valid admin session cookie
- **THEN** the system SHALL return `401 Unauthorized` with RFC7807 Problem JSON

### Requirement: Source Validation
The system SHALL validate that the `source` parameter is one of the allowed values: `["atcoder", "leetcode", "codeforces"]`.

#### Scenario: Valid source values
- **WHEN** admin requests any endpoint with `source` in `["atcoder", "leetcode", "codeforces"]`
- **THEN** the system SHALL process the request normally

#### Scenario: Case sensitivity
- **WHEN** admin requests with `source="LeetCode"` (mixed case)
- **THEN** the system SHALL return `400 Bad Request` (source validation is case-sensitive)

### Requirement: Session Authentication
The system SHALL authenticate admin API requests using the `admin_session` cookie, consistent with other admin routes.

#### Scenario: Valid session cookie
- **WHEN** request includes a valid `admin_session` cookie
- **THEN** the system SHALL process the request

#### Scenario: Expired session cookie
- **WHEN** request includes an expired `admin_session` cookie
- **THEN** the system SHALL return `401 Unauthorized` with RFC7807 Problem JSON

#### Scenario: Missing session cookie
- **WHEN** request does not include `admin_session` cookie
- **THEN** the system SHALL return `401 Unauthorized` with RFC7807 Problem JSON

### Requirement: RFC7807 Error Responses
The system SHALL return all error responses in RFC7807 Problem JSON format.

#### Scenario: RFC7807 structure
- **WHEN** any error occurs
- **THEN** the response SHALL include: `type` (URI reference), `title` (short summary), `status` (HTTP status code), `detail` (human-readable explanation)

#### Scenario: Content-Type header
- **WHEN** returning an RFC7807 error response
- **THEN** the `Content-Type` header SHALL be `application/problem+json`

### Requirement: Database Access Pattern
The system SHALL use read-only connection pool (`ro_pool`) with `spawn_blocking` for all admin API read operations.

#### Scenario: Read-only pool usage
- **WHEN** admin API endpoint fetches data from database
- **THEN** it SHALL use `ro_pool` (not `rw_pool`) to avoid write lock contention

#### Scenario: Blocking task execution
- **WHEN** admin API endpoint performs database query
- **THEN** it SHALL wrap the query in `spawn_blocking` to avoid blocking the async runtime

