## ADDED Requirements

### Requirement: Canonical crawler operations
Crawler scripts SHALL expose canonical operation flags for the shared crawler workflows they support. `--sync-problemset` SHALL mean fetching initial problem metadata while skipping existing problems. `--fetch-contest` SHALL mean fetching contest/archive problems and their content. `--fill-missing-content` SHALL mean filling content for existing problems whose metadata is already stored but whose content is missing.

#### Scenario: Shared operation vocabulary is available
- **WHEN** an operator inspects supported crawler CLI operations
- **THEN** metadata sync is represented by `--sync-problemset`
- **AND** contest/archive content fetching is represented by `--fetch-contest`
- **AND** missing content backfill is represented by `--fill-missing-content`

### Requirement: Problemset metadata sync support
The system SHALL support `--sync-problemset` for LeetCode, AtCoder, Codeforces, Luogu, and SPOJ crawler sources. The operation SHALL insert new problem metadata and SHALL skip existing problems unless a source-specific overwrite option is explicitly supplied and supported by that source.

#### Scenario: LeetCode problemset sync
- **WHEN** `leetcode.py` is invoked with `--sync-problemset`
- **THEN** the crawler fetches LeetCode problem metadata using the same behavior as the legacy `--init` operation

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
The system SHALL support `--fetch-contest` for AtCoder and Codeforces. The operation SHALL fetch contest/archive problem metadata and SHALL fetch content for each problem discovered from those contests.

#### Scenario: AtCoder contest fetch
- **WHEN** `atcoder.py` is invoked with `--fetch-contest`
- **THEN** the crawler fetches contest tasks from the AtCoder contest archive
- **AND** stores fetched problem content for those contest tasks

#### Scenario: Codeforces contest fetch
- **WHEN** `codeforces.py` is invoked with `--fetch-contest`
- **THEN** the crawler fetches contest problems from Codeforces contest APIs
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
The system SHALL retain auxiliary crawler flags for workflows outside the three canonical operations, including daily challenge fetches, single-contest fetches, Luogu training list sync, diagnostic runs, rate limiting, batch sizing, overwrite behavior, status/debug output, and internal source selection where needed.

#### Scenario: LeetCode daily flags remain supported
- **WHEN** `leetcode.py` is invoked with `--daily`, `--date`, `--monthly`, or `--domain`
- **THEN** the daily challenge workflow remains available

#### Scenario: Single contest flags remain supported
- **WHEN** AtCoder or Codeforces is invoked with `--contest <id>`
- **THEN** only the requested contest is fetched

#### Scenario: Luogu training list remains supported
- **WHEN** `luogu.py` is invoked with `--training-list <url-or-id>`
- **THEN** the Luogu training list sync workflow remains available
