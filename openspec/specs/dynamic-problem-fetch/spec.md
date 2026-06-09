# dynamic-problem-fetch Specification

## Purpose

Define bounded on-demand fetching for supported problem sources when a requested problem is missing from local storage.

## Requirements

### Requirement: Direct problem URL derivation
The system SHALL derive a canonical problem URL for supported source and ID pairs before attempting a dynamic single-problem fetch. Derivation SHALL accept only known source-specific ID formats and SHALL reject malformed IDs without invoking a crawler. AtCoder derivation SHALL infer the contest slug from the task ID prefix only for straightforward task IDs, SHALL map `pastYYYYMM_*` task IDs to `pastYYYYMM-open`, and SHALL accept explicit AtCoder contest paths in `contest/problem_id` and `contest/tasks/problem_id` forms when the contest cannot be inferred safely.

#### Scenario: Codeforces contest URL derivation
- **WHEN** the system derives a URL for source `codeforces` and ID `1988A`
- **THEN** it returns `https://codeforces.com/contest/1988/problem/A`

#### Scenario: Codeforces gym URL derivation from long contest ID
- **WHEN** the system derives a URL for source `codeforces` and ID `102951A`
- **THEN** it returns `https://codeforces.com/gym/102951/problem/A`

#### Scenario: Explicit gym URL derivation
- **WHEN** the system derives a URL for source `gym` and ID `102951A`
- **THEN** it returns `https://codeforces.com/gym/102951/problem/A`

#### Scenario: AtCoder straightforward URL derivation
- **WHEN** the system derives a URL for source `atcoder` and ID `abc321_a`
- **THEN** it returns `https://atcoder.jp/contests/abc321/tasks/abc321_a`

#### Scenario: AtCoder PAST URL derivation
- **WHEN** the system derives a URL for source `atcoder` and ID `past201912_a`
- **THEN** it returns `https://atcoder.jp/contests/past201912-open/tasks/past201912_a`

#### Scenario: AtCoder explicit contest path derivation
- **WHEN** the system derives a URL for source `atcoder` and ID `ndpc/ndpc2026_m`
- **THEN** it returns `https://atcoder.jp/contests/ndpc/tasks/ndpc2026_m`

#### Scenario: AtCoder explicit tasks path derivation
- **WHEN** the system derives a URL for source `atcoder` and ID `ndpc/tasks/ndpc2026_m`
- **THEN** it returns `https://atcoder.jp/contests/ndpc/tasks/ndpc2026_m`

#### Scenario: AtCoder ambiguous historical slug is not reinterpreted
- **WHEN** the system derives a URL for source `atcoder` and ID `arc058_abc042_a`
- **THEN** it returns `https://atcoder.jp/contests/arc058/tasks/arc058_abc042_a`

#### Scenario: AtCoder historical slug can be fetched with explicit contest
- **WHEN** the system derives a URL for source `atcoder` and ID `abc042/arc058_abc042_a`
- **THEN** it returns `https://atcoder.jp/contests/abc042/tasks/arc058_abc042_a`

#### Scenario: Luogu URL derivation
- **WHEN** the system derives a URL for source `luogu` and ID `P1083`
- **THEN** it returns `https://www.luogu.com.cn/problem/P1083`

#### Scenario: Malformed ID is rejected
- **WHEN** the system derives a URL for a supported source with an ID that does not match that source format
- **THEN** it returns no dynamic fetch plan

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
