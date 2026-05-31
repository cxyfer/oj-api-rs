## ADDED Requirements

### Requirement: Public difficulty discovery endpoint
The system SHALL expose `GET /api/v1/difficulties/{source}` to return distinct difficulty values available for the requested supported source. The endpoint SHALL require the same public API Bearer authentication as other `/api/v1/*` routes and SHALL use the existing supported source validation rules.

#### Scenario: List LeetCode difficulties
- **WHEN** client sends `GET /api/v1/difficulties/leetcode` with a valid Bearer token and stored LeetCode problems contain `Easy`, `Medium`, and `Hard` difficulties
- **THEN** system returns HTTP 200 with JSON array `["Easy", "Medium", "Hard"]`

#### Scenario: Invalid source
- **WHEN** client sends `GET /api/v1/difficulties/invalid_source` with a valid Bearer token
- **THEN** system returns HTTP 400 with RFC 7807 error body indicating invalid source

#### Scenario: Source has no stored difficulties
- **WHEN** client sends `GET /api/v1/difficulties/atcoder` with a valid Bearer token and no AtCoder rows have non-empty `difficulty`
- **THEN** system returns HTTP 200 with JSON array `[]`

### Requirement: Difficulty values are clean and canonical
The system SHALL return only non-empty difficulty strings after trimming surrounding whitespace. Returned values SHALL preserve the canonical stored string casing and characters so clients can reuse a returned value as the `difficulty` query parameter.

#### Scenario: Empty difficulties are omitted
- **WHEN** stored problems for a source contain `NULL`, empty string, whitespace-only, and `Easy` difficulty values
- **THEN** system returns HTTP 200 with JSON array `["Easy"]`

#### Scenario: Luogu difficulty value round-trips
- **WHEN** stored Luogu problems contain the difficulty `普及−`
- **THEN** the response includes `普及−` exactly
- **AND** client can call `GET /api/v1/problems/luogu?difficulty=%E6%99%AE%E5%8F%8A%E2%88%92` using that returned value

### Requirement: Difficulty ordering
The system SHALL sort known platform difficulty values by platform difficulty progression and sort unknown values deterministically after known values.

#### Scenario: LeetCode difficulties use canonical order
- **WHEN** stored LeetCode difficulties are discovered in any database order
- **THEN** response order SHALL be `Easy`, `Medium`, then `Hard` for values that exist

#### Scenario: Luogu difficulties use canonical order
- **WHEN** stored Luogu difficulties include `NOI/NOI+/CTSC`, `入门`, and `暂无评定`
- **THEN** response order SHALL be `暂无评定`, `入门`, then `NOI/NOI+/CTSC`

#### Scenario: Unknown difficulties use fallback order
- **WHEN** stored difficulties include unknown values not covered by a platform ordering map
- **THEN** response SHALL order known values first and remaining unknown values by deterministic lexical order
