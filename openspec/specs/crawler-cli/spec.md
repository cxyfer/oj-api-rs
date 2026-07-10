## Purpose

Define the supported crawler CLI operations and compatibility requirements for maintained crawler scripts.

## Requirements
### Requirement: Canonical crawler operations
Crawler scripts SHALL expose canonical operation flags for the shared crawler workflows they support. `--sync-problemset` SHALL mean fetching initial problem metadata while skipping existing problems. `--fetch-contest` SHALL mean fetching contest/archive problems and their content. `--fill-missing-content` SHALL mean filling content for existing problems whose metadata is already stored but whose content is missing.

#### Scenario: Shared operation vocabulary is available
- **WHEN** an operator inspects supported crawler CLI operations
- **THEN** metadata sync is represented by `--sync-problemset`
- **AND** contest/archive content fetching is represented by `--fetch-contest`
- **AND** missing content backfill is represented by `--fill-missing-content`

### Requirement: Single problem fetch support
The system SHALL support a source-scoped single-problem crawler operation for Codeforces, AtCoder, and Luogu. The operation SHALL fetch only the requested problem, SHALL persist the result through the existing `problems` table merge semantics, and SHALL be accepted only for crawler sources whose argument whitelist declares it. AtCoder single-problem fetch SHALL accept straightforward task IDs such as `abc321_a` and explicit contest paths in `contest/problem_id` or `contest/tasks/problem_id` form for task IDs whose contest cannot be inferred safely.

#### Scenario: Codeforces single problem fetch
- **WHEN** `codeforces.py` is invoked with `--problem 1988A`
- **THEN** the crawler fetches Codeforces problem `1988A` from the derived contest URL and stores it as source `codeforces`

#### Scenario: Codeforces gym single problem fetch
- **WHEN** `codeforces.py` is invoked with `--problem 102951A`
- **THEN** the crawler fetches Codeforces problem `102951A` from the derived gym URL and stores it as source `codeforces`

#### Scenario: AtCoder single problem fetch
- **WHEN** `atcoder.py` is invoked with `--problem abc321_a`
- **THEN** the crawler fetches AtCoder problem `abc321_a` from the derived task URL and stores it as source `atcoder`

#### Scenario: AtCoder explicit contest single problem fetch
- **WHEN** `atcoder.py` is invoked with `--problem ndpc/ndpc2026_m`
- **THEN** the crawler fetches AtCoder problem `ndpc2026_m` from contest `ndpc` and stores it as source `atcoder`

#### Scenario: AtCoder explicit tasks path single problem fetch
- **WHEN** `atcoder.py` is invoked with `--problem ndpc/tasks/ndpc2026_m`
- **THEN** the crawler fetches AtCoder problem `ndpc2026_m` from contest `ndpc` and stores it as source `atcoder`

#### Scenario: AtCoder ambiguous historical slug requires explicit contest for alternate contest lookup
- **WHEN** `atcoder.py` is invoked with `--problem abc042/arc058_abc042_a`
- **THEN** the crawler fetches AtCoder task `arc058_abc042_a` from contest `abc042` and stores it as source `atcoder`

#### Scenario: Luogu single problem fetch
- **WHEN** `luogu.py` is invoked with `--problem P1083`
- **THEN** the crawler fetches Luogu problem `P1083` from the derived problem URL and stores it as source `luogu`

#### Scenario: Unsupported source rejects single problem flag
- **WHEN** a crawler source without single-problem support is validated with `--problem <id>`
- **THEN** validation rejects the unsupported argument instead of silently running another operation

### Requirement: Problemset metadata sync support
The system SHALL support `--sync-problemset` for LeetCode, AtCoder, Codeforces, Luogu, and SPOJ crawler sources. The operation SHALL insert new problem metadata and SHALL skip existing problems unless a source-specific overwrite or metadata refresh behavior is explicitly supplied and supported by that source. LeetCode problemset sync SHALL refresh rating metadata for new and existing LeetCode problems while preserving existing detail-rich fields.

#### Scenario: LeetCode problemset sync
- **WHEN** `leetcode.py` is invoked with `--sync-problemset`
- **THEN** the crawler fetches LeetCode problem metadata using the same behavior as the legacy `--init` operation
- **AND** it attempts to merge available rating metadata into new and existing LeetCode problem records

#### Scenario: LeetCode rating source unavailable
- **WHEN** `leetcode.py` is invoked with `--sync-problemset`
- **AND** the external rating source is unavailable or returns no usable rating data
- **THEN** the crawler still persists LeetCode problem metadata from the problemset source
- **AND** existing positive ratings are not overwritten with zero or null placeholders

#### Scenario: LeetCode problemset sync preserves detail fields
- **WHEN** `leetcode.py` is invoked with `--sync-problemset`
- **AND** an existing LeetCode problem has stored content, tags, or similar questions
- **THEN** the metadata refresh does not clear those stored detail fields

#### Scenario: AtCoder problemset sync
- **WHEN** `atcoder.py` is invoked with `--sync-problemset`
- **THEN** the crawler fetches AtCoder problem metadata using the same behavior as the legacy Kenkoooo sync operation

#### Scenario: Codeforces problemset sync
- **WHEN** `codeforces.py` is invoked with `--sync-problemset`
- **THEN** the crawler fetches Codeforces problemset API metadata and skips existing stored problems

#### Scenario: Luogu problemset sync
- **WHEN** `luogu.py` is invoked for the Luogu source with `--sync-problemset`
- **THEN** the crawler fetches Luogu problem metadata using the same behavior as the legacy `--sync` operation

#### Scenario: SPOJ problemset sync
- **WHEN** `luogu.py` is invoked for the SPOJ source with `--sync-problemset`
- **THEN** the crawler fetches SPOJ metadata using the same behavior as the legacy `--sync-spoj` operation

### Requirement: Contest fetching support
The system SHALL support `--fetch-contest` for AtCoder and Codeforces. The operation SHALL fetch contest/archive problem metadata and SHALL fetch content for each problem discovered from those contests. Codeforces contest fetching SHALL request `contest.standings` with only the `contestId` query parameter.

#### Scenario: AtCoder contest fetch
- **WHEN** `atcoder.py` is invoked with `--fetch-contest`
- **THEN** the crawler fetches contest tasks from the AtCoder contest archive
- **AND** stores fetched problem content for those contest tasks

#### Scenario: Codeforces contest fetch
- **WHEN** `codeforces.py` is invoked with `--fetch-contest`
- **THEN** the crawler fetches contest problems from Codeforces contest APIs
- **AND** requests standings discovery without `from`, `count`, or any other pagination query parameter
- **AND** stores fetched problem content for those contest problems

#### Scenario: Unsupported contest fetch is rejected
- **WHEN** a source that does not support contest/archive crawling is invoked with `--fetch-contest`
- **THEN** the crawler CLI rejects the unsupported argument instead of silently running another operation

### Requirement: Contest fetch resumes by default
AtCoder and Codeforces `--fetch-contest` SHALL resume by default using each source's existing JSON progress file. Already-fetched contests recorded in progress SHALL be skipped. `--no-resume` SHALL disable progress-based skipping for that invocation.

#### Scenario: Default contest fetch skips completed contests
- **WHEN** AtCoder or Codeforces progress JSON records a contest as fetched
- **AND** the crawler is invoked with `--fetch-contest`
- **THEN** that contest is skipped

#### Scenario: No-resume contest fetch ignores progress
- **WHEN** AtCoder or Codeforces progress JSON records a contest as fetched
- **AND** the crawler is invoked with `--fetch-contest --no-resume`
- **THEN** the crawler does not use the progress record to skip that contest

#### Scenario: Legacy resume remains accepted
- **WHEN** AtCoder or Codeforces is invoked with the legacy `--fetch-all --resume`
- **THEN** the command remains accepted and performs resume-mode contest fetching

### Requirement: Missing content backfill support
The system SHALL support `--fill-missing-content` for every maintained crawler source. The operation SHALL only target existing problem records with missing content for the selected source.

#### Scenario: Backfill LeetCode content
- **WHEN** `leetcode.py` is invoked with `--fill-missing-content`
- **THEN** the crawler fetches details for LeetCode problems whose content is missing

#### Scenario: Backfill contest-source content
- **WHEN** `atcoder.py` or `codeforces.py` is invoked with `--fill-missing-content`
- **THEN** the crawler fetches content for stored AtCoder or Codeforces problems whose content is missing

#### Scenario: Backfill Luogu-family content
- **WHEN** `luogu.py` is invoked with `--fill-missing-content` for either the Luogu or SPOJ source
- **THEN** the crawler fetches content for stored problems whose content is missing for the selected source

### Requirement: Legacy operation aliases
The system SHALL keep legacy operation flags accepted as compatibility aliases. Legacy aliases SHALL map to the canonical operation behavior and SHALL NOT be the primary operation names in new documentation.

#### Scenario: Metadata sync aliases remain accepted
- **WHEN** a crawler is invoked with `--init`, `--sync`, `--sync-spoj`, `--sync-kenkoooo`, or `--sync-history` on a source that historically supported that flag
- **THEN** the command remains accepted as a metadata sync operation

#### Scenario: Contest fetch alias remains accepted
- **WHEN** AtCoder or Codeforces is invoked with `--fetch-all`
- **THEN** the command remains accepted as a contest fetch operation

#### Scenario: Unsupported legacy aliases remain source scoped
- **WHEN** a legacy flag is passed to a source that did not historically support it
- **THEN** the crawler CLI rejects the flag for that source

### Requirement: Auxiliary crawler flags remain maintained
The system SHALL retain auxiliary crawler flags for workflows outside the three canonical operations, including daily challenge fetches, single-contest fetches, external daily challenge source ingestion, Luogu training list sync, diagnostic runs, rate limiting, batch sizing, overwrite behavior, status/debug output, and internal source selection where needed.

#### Scenario: LeetCode daily flags remain supported
- **WHEN** `leetcode.py` is invoked with `--daily`, `--date`, `--monthly`, or `--domain`
- **THEN** the daily challenge workflow remains available

#### Scenario: Sheep daily source flags are supported
- **WHEN** `daily_source.py` is invoked with `--daily-source sheep --date 2026-06-02`
- **THEN** the daily challenge source ingestion workflow runs for that source and date

#### Scenario: 0x3f online daily source flags are supported
- **WHEN** `daily_source.py` is invoked with `--daily-source 0x3f --date 2026-06-09`
- **THEN** the daily challenge source ingestion workflow resolves the configured Tencent Docs token, preferring its direct local value over the environment fallback, and requests the fixed online sheet

#### Scenario: 0x3f daily file flag remains supported
- **WHEN** `daily_source.py` is invoked with `--daily-source 0x3f --date 2026-06-09 --daily-file 0x3f.csv`
- **THEN** the daily challenge source ingestion workflow reads the provided local file

#### Scenario: Single contest flags remain supported
- **WHEN** AtCoder or Codeforces is invoked with `--contest <id>`
- **THEN** only the requested contest is fetched

#### Scenario: Luogu training list remains supported
- **WHEN** `luogu.py` is invoked with `--training-list <url-or-id>`
- **THEN** the Luogu training list sync workflow remains available

### Requirement: Additional daily source CLI argument validation
The Rust crawler argument whitelist SHALL accept the daily-source flags needed by the dedicated daily-source crawler and SHALL reject invalid source names or unsafe local file paths. `--daily-file` SHALL remain optional and, when present, SHALL accept only relative safe paths and SHALL reject absolute paths and parent-directory traversal.

#### Scenario: Accept Sheep daily args
- **WHEN** `validate_args` is called for the daily-source crawler with `--daily-source sheep --date 2026-06-02`
- **THEN** validation passes

#### Scenario: Accept 0x3f online daily args
- **WHEN** `validate_args` is called for the daily-source crawler with `--daily-source 0x3f --date 2026-06-09`
- **THEN** validation passes

#### Scenario: Accept 0x3f daily file args
- **WHEN** `validate_args` is called for the daily-source crawler with `--daily-source 0x3f --date 2026-06-09 --daily-file data/0x3f.csv`
- **THEN** validation passes

#### Scenario: Reject incomplete daily-source args
- **WHEN** `validate_args` is called for the daily-source crawler without `--daily-source` or without `--date`
- **THEN** validation fails before a crawler job is created

#### Scenario: Reject daily file for Sheep
- **WHEN** `validate_args` is called for the daily-source crawler with `--daily-source sheep --date 2026-06-09 --daily-file data/sheep.csv`
- **THEN** validation fails because `--daily-file` is only supported for `0x3f`

#### Scenario: Reject invalid daily source
- **WHEN** `validate_args` is called for the daily-source crawler with `--daily-source unknown`
- **THEN** validation fails with an invalid daily source error

#### Scenario: Reject parent traversal daily file path
- **WHEN** `validate_args` is called for the daily-source crawler with `--daily-file ../secret.csv`
- **THEN** validation fails with a path safety error

#### Scenario: Reject absolute daily file path
- **WHEN** `validate_args` is called for the daily-source crawler with `--daily-file /tmp/0x3f.csv`
- **THEN** validation fails with a path safety error
