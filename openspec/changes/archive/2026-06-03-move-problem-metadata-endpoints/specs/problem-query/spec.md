## MODIFIED Requirements

### Requirement: Difficulty filter discoverability
The problem query API SHALL provide a discoverability endpoint for valid per-source difficulty filter values at `GET /api/v1/problems/difficulties/{source}` so clients can populate the existing `difficulty` query parameter without hard-coded platform lists.

#### Scenario: Discover then filter by difficulty
- **WHEN** client reads `GET /api/v1/problems/difficulties/leetcode` and receives `Medium`
- **AND** client sends `GET /api/v1/problems/leetcode?difficulty=Medium`
- **THEN** system returns only LeetCode problems whose difficulty matches `Medium` case-insensitively

#### Scenario: Discovery endpoint shares problem source validation
- **WHEN** client sends `GET /api/v1/problems/difficulties/invalid_source`
- **THEN** system rejects the request with the same invalid source behavior used by problem query endpoints

## ADDED Requirements

### Requirement: Tag filter discoverability
The problem query API SHALL provide a discoverability endpoint for valid per-source tag filter values at `GET /api/v1/problems/tags/{source}` so clients can populate the existing `tags` query parameter without hard-coded platform lists.

#### Scenario: Discover then filter by tag
- **WHEN** client reads `GET /api/v1/problems/tags/leetcode` and receives `Array`
- **AND** client sends `GET /api/v1/problems/leetcode?tags=Array`
- **THEN** system returns only LeetCode problems whose tags include `Array` according to the existing tag filter semantics

#### Scenario: Tag discovery endpoint shares problem source validation
- **WHEN** client sends `GET /api/v1/problems/tags/invalid_source`
- **THEN** system rejects the request with the same invalid source behavior used by problem query endpoints
