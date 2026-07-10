## MODIFIED Requirements

### Requirement: Single problem retrieval
The system SHALL return the complete problem data when queried by source and ID via `GET /api/v1/problems/{source}/{id}`. The response SHALL include all columns from the `problems` table except that `similar_questions` SHALL be exposed as a hydrated array of `ProblemSummary` objects resolved from the stored slug list. The `tags` column SHALL be deserialized as a JSON string array; if parsing fails, the system SHALL return an empty array instead of failing. Legacy `similar_questions` payloads stored as JSON object arrays SHALL be normalized to slug lists during read, and any unresolved slug SHALL be omitted from the hydrated response rather than causing request failure. When no database row exists for a supported URL-derivable source and ID, the system SHALL attempt a bounded single-problem dynamic fetch before returning not found.

#### Scenario: Valid problem exists
- **WHEN** client sends `GET /api/v1/problems/leetcode/1` with a valid Bearer token
- **THEN** system returns HTTP 200 with the full problem object including `content`, `content_cn`, `tags` (as array), and `similar_questions` as an array of hydrated summary objects

#### Scenario: Supported problem missing from database can be fetched dynamically
- **WHEN** client sends `GET /api/v1/problems/luogu/P1083` and no such problem exists locally
- **AND** the direct single-problem crawler fetch succeeds and stores the problem
- **THEN** system returns HTTP 200 with the full problem object from the stored crawler result

#### Scenario: Problem not found
- **WHEN** client sends `GET /api/v1/problems/leetcode/999999` and no such problem exists
- **THEN** system returns HTTP 404 with RFC 7807 error body (`type`, `title`, `status`, `detail`)

#### Scenario: Dynamic fetch failure remains not found
- **WHEN** client sends `GET /api/v1/problems/codeforces/1988A` and no such problem exists locally
- **AND** the direct single-problem crawler fails or does not store the problem
- **THEN** system returns HTTP 404 with RFC 7807 error body (`type`, `title`, `status`, `detail`)

#### Scenario: Invalid source
- **WHEN** client sends `GET /api/v1/problems/invalid_source/1`
- **THEN** system returns HTTP 400 with RFC 7807 error body indicating invalid source

#### Scenario: Malformed tags in DB
- **WHEN** the `tags` column contains `null`, empty string, or invalid JSON
- **THEN** system returns the problem with `tags` as an empty array `[]` without panicking

#### Scenario: Legacy similar question object payload in DB
- **WHEN** the `similar_questions` column stores legacy LeetCode objects such as `{"titleSlug":"two-sum"}`
- **THEN** system normalizes them to slug values and returns hydrated summary objects for the slugs that can be resolved from the same source

#### Scenario: Missing similar question slug in DB
- **WHEN** a stored similar question slug has no matching problem row
- **THEN** system omits that entry from the hydrated `similar_questions` response and still returns HTTP 200
