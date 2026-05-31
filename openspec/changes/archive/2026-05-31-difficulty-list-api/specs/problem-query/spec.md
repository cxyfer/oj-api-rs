## ADDED Requirements

### Requirement: Difficulty filter discoverability
The problem query API SHALL provide a discoverability endpoint for valid per-source difficulty filter values so clients can populate the existing `difficulty` query parameter without hard-coded platform lists.

#### Scenario: Discover then filter by difficulty
- **WHEN** client reads `GET /api/v1/difficulties/leetcode` and receives `Medium`
- **AND** client sends `GET /api/v1/problems/leetcode?difficulty=Medium`
- **THEN** system returns only LeetCode problems whose difficulty matches `Medium` case-insensitively

#### Scenario: Discovery endpoint shares problem source validation
- **WHEN** client sends `GET /api/v1/difficulties/invalid_source`
- **THEN** system rejects the request with the same invalid source behavior used by problem query endpoints
