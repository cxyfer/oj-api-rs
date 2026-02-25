# luogu-source-registration Specification

## Purpose
TBD - created by archiving change luogu-crawler. Update Purpose after archive.
## Requirements
### Requirement: Luogu variant in CrawlerSource enum
The `CrawlerSource` enum in `src/models.rs` SHALL include a `Luogu` variant. `CrawlerSource::parse("luogu")` SHALL return `Ok(CrawlerSource::Luogu)`. Any other unrecognized string SHALL continue to return `Err`.

#### Scenario: Parse luogu source
- **WHEN** `CrawlerSource::parse("luogu")` is called
- **THEN** it SHALL return `Ok(CrawlerSource::Luogu)`

#### Scenario: Parse unknown source
- **WHEN** `CrawlerSource::parse("unknown")` is called
- **THEN** it SHALL return `Err` with message containing "invalid source"

### Requirement: Luogu script name mapping
`CrawlerSource::Luogu.script_name()` SHALL return `"luogu.py"`.

#### Scenario: Script name resolution
- **WHEN** `CrawlerSource::Luogu.script_name()` is called
- **THEN** it SHALL return `"luogu.py"`

### Requirement: LUOGU_ARGS whitelist
A static `LUOGU_ARGS: &[ArgSpec]` SHALL be defined with exactly 8 entries matching the Python CLI:

| Flag | Arity | ValueType | ui_exposed |
|---|---|---|---|
| `--sync` | 0 | None | true |
| `--sync-content` | 0 | None | true |
| `--fill-missing-content` | 0 | None | true |
| `--missing-content-stats` | 0 | None | true |
| `--status` | 0 | None | true |
| `--rate-limit` | 1 | Float | true |
| `--data-dir` | 1 | Str | false |
| `--db-path` | 1 | Str | false |

`CrawlerSource::Luogu.arg_specs()` SHALL return `LUOGU_ARGS`.

#### Scenario: Valid sync argument
- **WHEN** `validate_args(&CrawlerSource::Luogu, &["--sync".into()])` is called
- **THEN** it SHALL return `Ok` with the args vector

#### Scenario: Valid sync-content argument
- **WHEN** `validate_args(&CrawlerSource::Luogu, &["--sync-content".into()])` is called
- **THEN** it SHALL return `Ok` with the args vector

#### Scenario: Valid fill-missing-content argument
- **WHEN** `validate_args(&CrawlerSource::Luogu, &["--fill-missing-content".into()])` is called
- **THEN** it SHALL return `Ok` with the args vector

#### Scenario: Valid missing-content-stats argument
- **WHEN** `validate_args(&CrawlerSource::Luogu, &["--missing-content-stats".into()])` is called
- **THEN** it SHALL return `Ok` with the args vector

#### Scenario: Valid rate-limit argument
- **WHEN** `validate_args(&CrawlerSource::Luogu, &["--rate-limit".into(), "3.0".into()])` is called
- **THEN** it SHALL return `Ok`

#### Scenario: Invalid rate-limit value
- **WHEN** `validate_args(&CrawlerSource::Luogu, &["--rate-limit".into(), "-1.0".into()])` is called
- **THEN** it SHALL return `Err` containing "invalid positive float"

#### Scenario: Unknown argument rejected
- **WHEN** `validate_args(&CrawlerSource::Luogu, &["--fetch-all".into()])` is called
- **THEN** it SHALL return `Err` containing "unknown argument"

#### Scenario: data-dir path traversal rejected
- **WHEN** `validate_args(&CrawlerSource::Luogu, &["--data-dir".into(), "../etc".into()])` is called
- **THEN** it SHALL return `Err` containing "must not contain '..'"

#### Scenario: data-dir absolute path rejected
- **WHEN** `validate_args(&CrawlerSource::Luogu, &["--data-dir".into(), "/tmp/data".into()])` is called
- **THEN** it SHALL return `Err` containing "must be a relative path"

