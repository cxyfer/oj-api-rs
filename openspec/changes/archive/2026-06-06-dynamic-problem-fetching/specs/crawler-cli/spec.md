## ADDED Requirements

### Requirement: Single problem fetch support
The system SHALL support a source-scoped single-problem crawler operation for Codeforces, AtCoder, and Luogu. The operation SHALL fetch only the requested problem, SHALL persist the result through the existing `problems` table merge semantics, and SHALL be accepted only for crawler sources whose argument whitelist declares it.

#### Scenario: Codeforces single problem fetch
- **WHEN** `codeforces.py` is invoked with `--problem 1988A`
- **THEN** the crawler fetches Codeforces problem `1988A` from the derived contest URL and stores it as source `codeforces`

#### Scenario: Codeforces gym single problem fetch
- **WHEN** `codeforces.py` is invoked with `--problem 102951A`
- **THEN** the crawler fetches Codeforces problem `102951A` from the derived gym URL and stores it as source `codeforces`

#### Scenario: AtCoder single problem fetch
- **WHEN** `atcoder.py` is invoked with `--problem abc321_a`
- **THEN** the crawler fetches AtCoder problem `abc321_a` from the derived task URL and stores it as source `atcoder`

#### Scenario: Luogu single problem fetch
- **WHEN** `luogu.py` is invoked with `--problem P1083`
- **THEN** the crawler fetches Luogu problem `P1083` from the derived problem URL and stores it as source `luogu`

#### Scenario: Unsupported source rejects single problem flag
- **WHEN** a crawler source without single-problem support is validated with `--problem <id>`
- **THEN** validation rejects the unsupported argument instead of silently running another operation
