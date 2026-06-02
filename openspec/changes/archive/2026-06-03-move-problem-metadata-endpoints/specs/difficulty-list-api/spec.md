## MODIFIED Requirements

### Requirement: Public difficulty discovery endpoint
The system SHALL expose `GET /api/v1/problems/difficulties/{source}` to return distinct difficulty values available for the requested supported source. The endpoint SHALL require the same public API Bearer authentication as other `/api/v1/*` routes and SHALL use the existing supported source validation rules.

#### Scenario: List LeetCode difficulties
- **WHEN** client sends `GET /api/v1/problems/difficulties/leetcode` with a valid Bearer token and stored LeetCode problems contain `Easy`, `Medium`, and `Hard` difficulties
- **THEN** system returns HTTP 200 with JSON array `["Easy", "Medium", "Hard"]`

#### Scenario: Invalid source
- **WHEN** client sends `GET /api/v1/problems/difficulties/invalid_source` with a valid Bearer token
- **THEN** system returns HTTP 400 with RFC 7807 error body indicating invalid source

#### Scenario: Source has no stored difficulties
- **WHEN** client sends `GET /api/v1/problems/difficulties/atcoder` with a valid Bearer token and no AtCoder rows have non-empty `difficulty`
- **THEN** system returns HTTP 200 with JSON array `[]`
