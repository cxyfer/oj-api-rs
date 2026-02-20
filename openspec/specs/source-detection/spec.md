# source-detection Specification

## Purpose
TBD - created by archiving change oj-api-rs-v1. Update Purpose after archive.
## Requirements
### Requirement: URL-based source detection
The system SHALL parse known OJ URLs and extract source and problem ID. Supported domains: `atcoder.jp`, `leetcode.com`, `leetcode.cn`, `codeforces.com`, `luogu.com.cn`.

#### Scenario: LeetCode URL
- **WHEN** input is `https://leetcode.com/problems/two-sum/`
- **THEN** system returns `(source: "leetcode", id: "two-sum")`

#### Scenario: LeetCode CN URL
- **WHEN** input is `https://leetcode.cn/problems/two-sum/`
- **THEN** system returns `(source: "leetcode", id: "two-sum")`

#### Scenario: LeetCode contest URL
- **WHEN** input is `https://leetcode.com/contest/weekly-contest-400/problems/two-sum/`
- **THEN** system returns `(source: "leetcode", id: "two-sum")`

#### Scenario: Codeforces contest URL
- **WHEN** input is `https://codeforces.com/contest/2000/problem/A`
- **THEN** system returns `(source: "codeforces", id: "2000A")`

#### Scenario: Codeforces problemset URL
- **WHEN** input is `https://codeforces.com/problemset/problem/2000/A`
- **THEN** system returns `(source: "codeforces", id: "2000A")`

#### Scenario: AtCoder URL
- **WHEN** input is `https://atcoder.jp/contests/abc321/tasks/abc321_a`
- **THEN** system returns `(source: "atcoder", id: "abc321_a")`

#### Scenario: Luogu URL
- **WHEN** input is `https://www.luogu.com.cn/problem/P1001`
- **THEN** system returns `(source: "luogu", id: "P1001")`

### Requirement: Prefix-based source detection
The system SHALL recognize `source:id` prefix format and split accordingly.

#### Scenario: Explicit prefix
- **WHEN** input is `atcoder:abc321_a`
- **THEN** system returns `(source: "atcoder", id: "abc321_a")`

#### Scenario: Codeforces prefix
- **WHEN** input is `codeforces:2179A`
- **THEN** system returns `(source: "codeforces", id: "2179A")`

### Requirement: Pattern-based source inference
The system SHALL infer source from ID patterns when no URL or prefix is provided. Priority: CF pattern > AtCoder pattern > pure numeric (LeetCode) > default slug (LeetCode).

#### Scenario: Pure numeric defaults to LeetCode
- **WHEN** input is `2000`
- **THEN** system returns `(source: "leetcode", id: "2000")`

#### Scenario: Digit-letter pattern infers Codeforces
- **WHEN** input is `2000A`
- **THEN** system returns `(source: "codeforces", id: "2000A")`

#### Scenario: CF prefix pattern
- **WHEN** input is `CF2000A`
- **THEN** system returns `(source: "codeforces", id: "2000A")` (strips CF prefix)

#### Scenario: Codeforces with sub-problem
- **WHEN** input is `1999B1`
- **THEN** system returns `(source: "codeforces", id: "1999B1")`

#### Scenario: AtCoder contest pattern
- **WHEN** input is `abc321_a`
- **THEN** system returns `(source: "atcoder", id: "abc321_a")`

#### Scenario: Unknown input defaults to LeetCode slug
- **WHEN** input is `two-sum`
- **THEN** system returns `(source: "leetcode", id: "two-sum")`

### Requirement: Resolve endpoint
The system SHALL expose `GET /api/v1/resolve/{query}` that applies source detection and returns the identified source, ID, and problem data if available. The `{query}` path parameter SHALL support URL-encoded values.

#### Scenario: Resolve with existing problem
- **WHEN** client sends `GET /api/v1/resolve/2000` and LeetCode problem 2000 exists in DB
- **THEN** system returns HTTP 200 with `{"source": "leetcode", "id": "2000", "problem": {...}}`

#### Scenario: Resolve with non-existent problem
- **WHEN** client sends `GET /api/v1/resolve/abc999_z` and that AtCoder problem does not exist
- **THEN** system returns HTTP 200 with `{"source": "atcoder", "id": "abc999_z", "problem": null}`

#### Scenario: Resolve URL-encoded input
- **WHEN** client sends `GET /api/v1/resolve/https%3A%2F%2Fleetcode.com%2Fproblems%2Ftwo-sum`
- **THEN** system URL-decodes the input and returns the correct detection result

#### Scenario: Resolve non-supported source
- **WHEN** detection identifies source as "luogu" (no data in DB)
- **THEN** system returns HTTP 200 with `{"source": "luogu", "id": "P1001", "problem": null}`

### Requirement: URL-encoded resolve equivalence
The system SHALL produce identical results for `resolve(url_encode(query))` and `resolve(query)`.

#### Scenario: Encoded vs decoded parity
- **WHEN** the same query is sent both URL-encoded and raw
- **THEN** both requests return the same `source` and `id`

