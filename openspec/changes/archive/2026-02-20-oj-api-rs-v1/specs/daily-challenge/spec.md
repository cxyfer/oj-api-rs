## ADDED Requirements

### Requirement: Daily challenge retrieval
The system SHALL return the LeetCode daily challenge via `GET /api/v1/daily?domain={com|cn}&date={YYYY-MM-DD}`.

#### Scenario: Today's daily (default)
- **WHEN** client sends `GET /api/v1/daily?domain=com` without `date` parameter
- **THEN** system returns today's (UTC) daily challenge for leetcode.com

#### Scenario: Specific date
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-01-15`
- **THEN** system returns the daily challenge for that specific date

#### Scenario: CN domain
- **WHEN** client sends `GET /api/v1/daily?domain=cn`
- **THEN** system returns the daily challenge from leetcode.cn

### Requirement: Daily challenge date validation
The system SHALL validate the `date` parameter format as `YYYY-MM-DD` and enforce range `[2020-04-01, today UTC]`.

#### Scenario: Date before lower bound
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2019-01-01`
- **THEN** system returns HTTP 400 with error detail indicating date must be >= 2020-04-01

#### Scenario: Future date
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2099-01-01`
- **THEN** system returns HTTP 400 with error detail indicating date must be <= today

#### Scenario: Invalid date format
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=01-15-2024`
- **THEN** system returns HTTP 400 with error detail indicating invalid date format

#### Scenario: Invalid calendar date
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-02-30`
- **THEN** system returns HTTP 400 with error detail indicating invalid date

### Requirement: Daily challenge domain validation
The system SHALL only accept `com` or `cn` as valid domain values.

#### Scenario: Invalid domain
- **WHEN** client sends `GET /api/v1/daily?domain=jp`
- **THEN** system returns HTTP 400 with error detail indicating invalid domain

### Requirement: Daily challenge not found
The system SHALL return 404 when no daily challenge record exists for the given date and domain.

#### Scenario: No data for date
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2020-04-01` but no record exists
- **THEN** system returns HTTP 404 with RFC 7807 error body
