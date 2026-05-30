## ADDED Requirements

### Requirement: Batch fetch endpoint accepts POST with problem list
The system SHALL expose `POST /api/v1/problems/batch` accepting a JSON array of objects with `source` and `id` string fields. The endpoint SHALL be protected by the same bearer auth middleware as other `/api/v1/*` routes.

#### Scenario: Successful batch fetch in summary mode
- **WHEN** a client POSTs `[{"source":"leetcode","id":"1"},{"source":"codeforces","id":"1A"}]` to `/api/v1/problems/batch`
- **THEN** the response is HTTP 200 with `{"results":[...], "not_found":[]}` where each result is a `ProblemSummary` (12 fields: id, source, slug, title, title_cn, difficulty, ac_rate, rating, contest, problem_index, tags, link)

#### Scenario: Successful batch fetch in detail mode
- **WHEN** a client POSTs with `?detail=true` and the same body
- **THEN** the response is HTTP 200 where each result is a `ProblemDetailResponse` (17 fields including content, content_cn, and hydrated `similar_questions` array)

#### Scenario: Partial not-found
- **WHEN** some (source, id) pairs do not exist in the database
- **THEN** the response is HTTP 200 with found items in `results[]` and missing items in `not_found[]` as `{source, id}` objects

### Requirement: Batch size validation
The system SHALL reject requests with an empty array or exceeding 50 items with HTTP 400.

#### Scenario: Empty array
- **WHEN** a client POSTs `[]` to the batch endpoint
- **THEN** the response is HTTP 400 with detail "request body must not be empty"

#### Scenario: Exceeds max size
- **WHEN** a client POSTs an array with more than 50 items
- **THEN** the response is HTTP 400 with detail indicating the batch size exceeds maximum

### Requirement: Source validation
The system SHALL validate all `source` values in the request body against the allowed source list before executing any database queries.

#### Scenario: Invalid source
- **WHEN** any item in the batch has an invalid `source` value
- **THEN** the response is HTTP 400 with detail indicating the invalid source

### Requirement: API documentation reflects batch endpoint
The API docs page at `/docs` SHALL document the batch endpoint under the "Problems" group. The legacy `/docs/api` path SHALL redirect to `/docs`.

#### Scenario: Docs page renders batch card
- **WHEN** a user visits `/docs`
- **THEN** a card with method `POST`, path `/api/v1/problems/batch`, and title "Batch fetch" appears in the Problems group

#### Scenario: Legacy docs path redirects
- **WHEN** a user visits `/docs/api`
- **THEN** the request is redirected to `/docs`
