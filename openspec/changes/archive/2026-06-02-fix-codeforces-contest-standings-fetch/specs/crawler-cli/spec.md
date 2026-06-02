## MODIFIED Requirements

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
