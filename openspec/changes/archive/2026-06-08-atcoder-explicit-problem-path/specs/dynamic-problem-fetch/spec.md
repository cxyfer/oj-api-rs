## MODIFIED Requirements

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
