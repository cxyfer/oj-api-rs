## MODIFIED Requirements

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
