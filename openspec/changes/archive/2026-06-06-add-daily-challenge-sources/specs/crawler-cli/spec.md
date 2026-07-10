## MODIFIED Requirements

### Requirement: Auxiliary crawler flags remain maintained
The system SHALL retain auxiliary crawler flags for workflows outside the three canonical operations, including daily challenge fetches, single-contest fetches, external daily challenge source ingestion, Luogu training list sync, diagnostic runs, rate limiting, batch sizing, overwrite behavior, status/debug output, and internal source selection where needed.

#### Scenario: LeetCode daily flags remain supported
- **WHEN** `leetcode.py` is invoked with `--daily`, `--date`, `--monthly`, or `--domain`
- **THEN** the daily challenge workflow remains available

#### Scenario: Additional daily source flags are supported
- **WHEN** `codeforces.py` is invoked with `--daily-source sheep --date 2026-06-02`
- **THEN** the daily challenge source ingestion workflow runs for that source and date

#### Scenario: 0x3f daily file flag is supported
- **WHEN** `codeforces.py` is invoked with `--daily-source 0x3f --date 2026-06-02 --daily-file 0x3f.csv`
- **THEN** the daily challenge source ingestion workflow reads the provided local file

#### Scenario: Single contest flags remain supported
- **WHEN** AtCoder or Codeforces is invoked with `--contest <id>`
- **THEN** only the requested contest is fetched

#### Scenario: Luogu training list remains supported
- **WHEN** `luogu.py` is invoked with `--training-list <url-or-id>`
- **THEN** the Luogu training list sync workflow remains available

## ADDED Requirements

### Requirement: Additional daily source CLI argument validation
The Rust crawler argument whitelist SHALL accept the daily-source flags needed by `codeforces.py` and SHALL reject invalid source names or unsafe local file paths.

#### Scenario: Accept Sheep daily args
- **WHEN** `validate_args` is called for Codeforces with `--daily-source sheep --date 2026-06-02`
- **THEN** validation passes

#### Scenario: Accept 0x3f daily file args
- **WHEN** `validate_args` is called for Codeforces with `--daily-source 0x3f --date 2026-06-02 --daily-file data/0x3f.csv`
- **THEN** validation passes

#### Scenario: Reject invalid daily source
- **WHEN** `validate_args` is called for Codeforces with `--daily-source unknown`
- **THEN** validation fails with an invalid daily source error

#### Scenario: Reject unsafe daily file path
- **WHEN** `validate_args` is called for Codeforces with `--daily-file ../secret.csv`
- **THEN** validation fails with a path safety error
