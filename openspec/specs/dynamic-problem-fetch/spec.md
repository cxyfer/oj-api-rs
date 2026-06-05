# dynamic-problem-fetch Specification

## Purpose

Define bounded on-demand fetching for supported problem sources when a requested problem is missing from local storage.

## Requirements

### Requirement: Direct problem URL derivation
The system SHALL derive a canonical problem URL for supported source and ID pairs before attempting a dynamic single-problem fetch. Derivation SHALL accept only known source-specific ID formats and SHALL reject malformed IDs without invoking a crawler.

#### Scenario: Codeforces contest URL derivation
- **WHEN** the system derives a URL for source `codeforces` and ID `1988A`
- **THEN** it returns `https://codeforces.com/contest/1988/problem/A`

#### Scenario: Codeforces gym URL derivation from long contest ID
- **WHEN** the system derives a URL for source `codeforces` and ID `102951A`
- **THEN** it returns `https://codeforces.com/gym/102951/problem/A`

#### Scenario: Explicit gym URL derivation
- **WHEN** the system derives a URL for source `gym` and ID `102951A`
- **THEN** it returns `https://codeforces.com/gym/102951/problem/A`

#### Scenario: AtCoder URL derivation
- **WHEN** the system derives a URL for source `atcoder` and ID `abc321_a`
- **THEN** it returns `https://atcoder.jp/contests/abc321/tasks/abc321_a`

#### Scenario: Luogu URL derivation
- **WHEN** the system derives a URL for source `luogu` and ID `P1083`
- **THEN** it returns `https://www.luogu.com.cn/problem/P1083`

#### Scenario: Malformed ID is rejected
- **WHEN** the system derives a URL for a supported source with an ID that does not match that source format
- **THEN** it returns no dynamic fetch plan

### Requirement: Dynamic single-problem fetch on database miss
The system SHALL fetch a supported missing problem through a single-problem crawler operation, persist the crawler result, and return the persisted problem detail if the row becomes available. The system SHALL NOT perform broad sync, contest scan, or batch operations during this fallback.

#### Scenario: Supported miss is fetched and returned
- **WHEN** `GET /api/v1/problems/codeforces/1988A` has no matching database row
- **AND** the single-problem crawler successfully persists `codeforces:1988A`
- **THEN** the API returns HTTP 200 with the existing problem detail response shape

#### Scenario: Unsupported miss stays not found
- **WHEN** `GET /api/v1/problems/leetcode/999999` has no matching database row
- **THEN** the API returns the existing RFC 7807 HTTP 404 response without invoking a dynamic crawler

#### Scenario: Crawler failure stays not found
- **WHEN** a supported missing problem triggers a dynamic crawler
- **AND** the crawler fails, times out, or does not persist a matching row
- **THEN** the API returns the existing RFC 7807 HTTP 404 response
