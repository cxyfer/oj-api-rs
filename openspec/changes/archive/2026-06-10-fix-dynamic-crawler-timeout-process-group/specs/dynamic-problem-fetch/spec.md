## MODIFIED Requirements

### Requirement: Dynamic single-problem fetch on database miss
The system SHALL fetch a supported missing problem through a single-problem crawler operation, persist the crawler result, and return the persisted problem detail if the row becomes available. The system SHALL NOT perform broad sync, contest scan, or batch operations during this fallback. AtCoder explicit API paths SHALL support `/api/v1/problems/atcoder/{contest}/{problem_id}` and `/api/v1/problems/atcoder/{contest}/tasks/{problem_id}`. For explicit AtCoder paths, the database lookup ID SHALL be the normalized `problem_id` segment, while the crawler argument SHALL preserve the explicit contest path. The dynamic crawler operation SHALL be bounded by the configured crawler timeout and SHALL terminate the spawned crawler process group before returning a timeout failure.

#### Scenario: Supported miss is fetched and returned
- **WHEN** `GET /api/v1/problems/codeforces/1988A` has no matching database row
- **AND** the single-problem crawler successfully persists `codeforces:1988A`
- **THEN** the API returns HTTP 200 with the existing problem detail response shape

#### Scenario: AtCoder explicit contest path uses normalized database ID
- **WHEN** `GET /api/v1/problems/atcoder/abc042/aaabbb_aaabbb_ccc` is requested
- **THEN** the system looks up source `atcoder` and ID `aaabbb_aaabbb_ccc`
- **AND** a dynamic crawler invocation, if needed, receives `--problem abc042/aaabbb_aaabbb_ccc`

#### Scenario: AtCoder explicit tasks path uses normalized database ID
- **WHEN** `GET /api/v1/problems/atcoder/abc042/tasks/aaabbb_aaabbb_ccc` is requested
- **THEN** the system looks up source `atcoder` and ID `aaabbb_aaabbb_ccc`
- **AND** a dynamic crawler invocation, if needed, receives `--problem abc042/tasks/aaabbb_aaabbb_ccc`

#### Scenario: AtCoder normalized hit does not invoke crawler
- **WHEN** `GET /api/v1/problems/atcoder/abc042/aaabbb_aaabbb_ccc` is requested
- **AND** source `atcoder` and ID `aaabbb_aaabbb_ccc` already exists in the database
- **THEN** the API returns the existing problem detail without invoking the crawler

#### Scenario: Unsupported miss stays not found
- **WHEN** `GET /api/v1/problems/leetcode/999999` has no matching database row
- **THEN** the API returns the existing RFC 7807 HTTP 404 response without invoking a dynamic crawler

#### Scenario: Crawler failure stays not found
- **WHEN** a supported missing problem triggers a dynamic crawler
- **AND** the crawler fails, times out, or does not persist a matching row
- **THEN** the API returns the existing RFC 7807 HTTP 404 response

#### Scenario: Timed-out crawler process group is terminated
- **WHEN** a supported missing problem triggers a dynamic crawler
- **AND** the crawler exceeds the configured timeout
- **THEN** the system terminates the spawned crawler process group
- **AND** the API returns the existing RFC 7807 HTTP 404 response
