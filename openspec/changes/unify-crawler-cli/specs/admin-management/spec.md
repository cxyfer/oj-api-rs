## ADDED Requirements

### Requirement: Admin crawler trigger validates unified flags
The admin crawler trigger SHALL accept canonical crawler operation flags for the sources that support them and SHALL reject canonical operation flags for unsupported sources. Validation SHALL continue to enforce per-source allowlists, value arity, and value types before spawning Python subprocesses.

#### Scenario: Supported canonical operation accepted
- **WHEN** an admin sends `POST /admin/api/crawlers/trigger` with source `codeforces` and args `["--fetch-contest"]`
- **THEN** the system accepts the request and starts the Codeforces crawler subprocess

#### Scenario: Unsupported canonical operation rejected
- **WHEN** an admin sends `POST /admin/api/crawlers/trigger` with source `leetcode` and args `["--fetch-contest"]`
- **THEN** the system returns HTTP 400 without spawning a crawler subprocess

#### Scenario: No-resume validation
- **WHEN** an admin sends `POST /admin/api/crawlers/trigger` with source `atcoder` and args `["--fetch-contest", "--no-resume"]`
- **THEN** the system accepts both flags as zero-arity AtCoder crawler arguments

### Requirement: Admin crawler UI exposes unified operations
The `/admin/crawlers` page SHALL present canonical operation flags for routine crawler workflows. Source-specific legacy operation flags MAY remain accepted by the API, but the UI SHALL prefer canonical operation names for new manual crawler launches.

#### Scenario: AtCoder UI uses canonical operations
- **WHEN** an admin selects the AtCoder crawler source
- **THEN** the argument controls include `--sync-problemset`, `--fetch-contest`, `--no-resume`, and `--fill-missing-content`

#### Scenario: Codeforces UI uses canonical operations
- **WHEN** an admin selects the Codeforces crawler source
- **THEN** the argument controls include `--sync-problemset`, `--fetch-contest`, `--no-resume`, and `--fill-missing-content`

#### Scenario: Luogu UI uses canonical operations
- **WHEN** an admin selects the Luogu crawler source
- **THEN** the argument controls include `--sync-problemset` and `--fill-missing-content`

#### Scenario: SPOJ UI uses canonical operations
- **WHEN** an admin selects the SPOJ crawler source
- **THEN** the argument controls include `--sync-problemset` and `--fill-missing-content`

### Requirement: Admin crawler UI retains auxiliary workflows
The `/admin/crawlers` page SHALL keep maintained auxiliary controls for workflows outside the canonical crawler operations, including LeetCode daily challenge flags, single-contest fetches, rate limits, batch size, overwrite behavior, status/debug operations, Luogu training lists, and diagnostics.

#### Scenario: LeetCode daily controls remain visible
- **WHEN** an admin selects the LeetCode crawler source
- **THEN** the UI includes controls for daily challenge workflows such as `--daily`, `--date`, `--monthly`, and `--domain`

#### Scenario: Contest source auxiliary controls remain visible
- **WHEN** an admin selects AtCoder or Codeforces
- **THEN** the UI includes `--contest` and source-specific rate limit controls

#### Scenario: Diagnostic source remains available
- **WHEN** an admin selects the diagnostic crawler source
- **THEN** the UI continues to expose the diagnostic test target selector
