# problem-query Specification

## Purpose
TBD - created by archiving change oj-api-rs-v1. Update Purpose after archive.
## Requirements
### Requirement: Single problem retrieval
The system SHALL return the complete problem data when queried by source and ID via `GET /api/v1/problems/{source}/{id}`. The response SHALL include all columns from the `problems` table. Fields `tags` and `similar_questions` SHALL be deserialized as JSON arrays; if parsing fails, the system SHALL return an empty array instead of failing.

#### Scenario: Valid problem exists
- **WHEN** client sends `GET /api/v1/problems/leetcode/1` with a valid Bearer token
- **THEN** system returns HTTP 200 with the full problem object including `content`, `content_cn`, `tags` (as array), and `similar_questions` (as array)

#### Scenario: Problem not found
- **WHEN** client sends `GET /api/v1/problems/leetcode/999999` and no such problem exists
- **THEN** system returns HTTP 404 with RFC 7807 error body (`type`, `title`, `status`, `detail`)

#### Scenario: Invalid source
- **WHEN** client sends `GET /api/v1/problems/invalid_source/1`
- **THEN** system returns HTTP 400 with RFC 7807 error body indicating invalid source

#### Scenario: Malformed tags in DB
- **WHEN** the `tags` column contains `null`, empty string, or invalid JSON
- **THEN** system returns the problem with `tags` as an empty array `[]` without panicking

### Requirement: Problem list with pagination
The system SHALL return a paginated list of problems for a given source via `GET /api/v1/problems/{source}`. The response SHALL use a wrapped format: `{"data": [...], "meta": {"total", "page", "per_page", "total_pages"}}`. List items SHALL exclude `content`, `content_cn`, `similar_questions`, `category`, and `paid_only` fields.

#### Scenario: Default pagination
- **WHEN** client sends `GET /api/v1/problems/leetcode` without pagination params
- **THEN** system returns page 1 with `per_page=20`, `meta.total` matching the filtered count, and results ordered by `id`

#### Scenario: Custom pagination
- **WHEN** client sends `GET /api/v1/problems/leetcode?page=3&per_page=50`
- **THEN** system returns page 3 with up to 50 items and correct `meta.total_pages`

#### Scenario: per_page exceeds maximum
- **WHEN** client sends `GET /api/v1/problems/leetcode?per_page=200`
- **THEN** system clamps `per_page` to 100 and returns at most 100 items

#### Scenario: per_page below minimum
- **WHEN** client sends `GET /api/v1/problems/leetcode?per_page=0`
- **THEN** system clamps `per_page` to 1

#### Scenario: Page beyond available data
- **WHEN** client sends `GET /api/v1/problems/leetcode?page=9999`
- **THEN** system returns HTTP 200 with empty `data` array and correct `meta` (total unchanged)

### Requirement: Problem list filtering
The system SHALL support filtering by `tags` and `difficulty` query parameters on the list endpoint.

#### Scenario: Filter by single tag
- **WHEN** client sends `GET /api/v1/problems/leetcode?tags=Array`
- **THEN** system returns only problems whose `tags` JSON array contains "Array" (case-insensitive match, OR semantics)

#### Scenario: Filter by multiple tags
- **WHEN** client sends `GET /api/v1/problems/leetcode?tags=Array,Dynamic+Programming`
- **THEN** system returns problems matching ANY of the specified tags (OR semantics)

#### Scenario: Filter by difficulty
- **WHEN** client sends `GET /api/v1/problems/leetcode?difficulty=Easy`
- **THEN** system returns only problems with `difficulty = "Easy"` (case-insensitive)

#### Scenario: Combined filters
- **WHEN** client sends `GET /api/v1/problems/leetcode?tags=Array&difficulty=Medium`
- **THEN** system returns problems matching the tag filter AND difficulty filter, with `meta.total` reflecting the filtered count

### Requirement: Stable list ordering
The system SHALL order list results by `id` ascending to ensure deterministic pagination.

#### Scenario: Consistent ordering across pages
- **WHEN** client fetches page 1 and page 2 sequentially
- **THEN** no problem appears on both pages and the last item of page 1 precedes the first item of page 2 by `id` ordering

