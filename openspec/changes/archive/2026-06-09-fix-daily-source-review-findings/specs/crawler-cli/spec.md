## MODIFIED Requirements

### Requirement: Additional daily source CLI argument validation
The Rust crawler argument whitelist SHALL accept the daily-source flags needed by `codeforces.py` and SHALL reject invalid source names or unsafe local file paths. `--daily-file` SHALL accept only relative safe paths and SHALL reject absolute paths and parent-directory traversal.

#### Scenario: Accept Sheep daily args
- **WHEN** `validate_args` is called for Codeforces with `--daily-source sheep --date 2026-06-02`
- **THEN** validation passes

#### Scenario: Accept 0x3f daily file args
- **WHEN** `validate_args` is called for Codeforces with `--daily-source 0x3f --date 2026-06-02 --daily-file data/0x3f.csv`
- **THEN** validation passes

#### Scenario: Reject invalid daily source
- **WHEN** `validate_args` is called for Codeforces with `--daily-source unknown`
- **THEN** validation fails with an invalid daily source error

#### Scenario: Reject parent traversal daily file path
- **WHEN** `validate_args` is called for Codeforces with `--daily-file ../secret.csv`
- **THEN** validation fails with a path safety error

#### Scenario: Reject absolute daily file path
- **WHEN** `validate_args` is called for Codeforces with `--daily-file /tmp/0x3f.csv`
- **THEN** validation fails with a path safety error
