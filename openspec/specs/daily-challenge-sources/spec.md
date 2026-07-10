# daily-challenge-sources Specification

## Purpose

Define additional daily challenge source ingestion requirements.

## Requirements
### Requirement: Sheep daily source ingestion
The dedicated daily-source crawler SHALL ingest Sheep daily Codeforces problems from the public GitHub raw Markdown path for a requested date and SHALL store the compact daily row with source `sheep`.

#### Scenario: Ingest Sheep daily markdown
- **WHEN** `daily_source.py` is invoked with `--daily-source sheep --date 2026-06-02`
- **THEN** the crawler fetches `daily_problems/2026/06/0602/problems.md` from `Yawn-Sean/Daily_CF_Problems`
- **AND** stores a `daily_challenge` row with `date = "2026-06-02"` and `source = "sheep"`

#### Scenario: Parse regular Codeforces links
- **WHEN** the Sheep Markdown table contains `https://codeforces.com/contest/1930/problem/A` or `https://codeforces.com/problemset/problem/1930/A`
- **THEN** the crawler stores a Codeforces problem with `id = "1930A"`, `source = "codeforces"`, and a daily ref `codeforces:1930A`

#### Scenario: Parse Gym Codeforces links
- **WHEN** the Sheep Markdown table contains `https://codeforces.com/gym/106539/problem/D`
- **THEN** the crawler stores a Codeforces problem with `id = "GYM106539D"`, `source = "codeforces"`, and a daily ref `codeforces:GYM106539D`

#### Scenario: Missing Sheep daily file
- **WHEN** the requested Sheep raw Markdown file returns 404 or contains no parseable problems
- **THEN** the crawler exits without writing an empty `daily_challenge` row

### Requirement: 0x3f online source ingestion
The dedicated daily-source crawler SHALL ingest 0x3f daily problems from the fixed Tencent Docs online sheet `DWGFoRGVZRmxNaXFz` / `BB08J2` (`算法趣题`) by default and SHALL store the compact daily row with source `0x3f`. The crawler SHALL use Tencent Docs MCP `sheet.get_cell_data` with an Authorization token loaded from a non-empty direct `config.toml` value, falling back to the configured environment variable. A local `--daily-file` CSV/TSV input SHALL remain available as an explicit offline/debug fallback.

#### Scenario: Ingest 0x3f online Tencent Docs sheet
- **WHEN** `daily_source.py` is invoked with `--daily-source 0x3f --date 2026-06-09`
- **AND** a non-empty direct Tencent Docs token in `config.toml` or its configured environment-variable fallback is available
- **THEN** the crawler calls Tencent Docs MCP `sheet.get_cell_data` for file `DWGFoRGVZRmxNaXFz` and sheet `BB08J2`
- **AND** stores a `daily_challenge` row with `date = "2026-06-09"` and `source = "0x3f"`

#### Scenario: 0x3f token missing
- **WHEN** `daily_source.py` is invoked with `--daily-source 0x3f --date 2026-06-09` without `--daily-file`
- **AND** the direct `config.toml` token and its configured environment-variable fallback are both missing or empty
- **THEN** the crawler fails with a clear token configuration error
- **AND** it does not write an empty `daily_challenge` row

#### Scenario: 0x3f local file fallback
- **WHEN** `daily_source.py` is invoked with `--daily-source 0x3f --date 2026-06-09 --daily-file <path>`
- **THEN** the crawler reads the local tabular file instead of calling Tencent Docs MCP
- **AND** stores a `daily_challenge` row with `date = "2026-06-09"` and `source = "0x3f"`

#### Scenario: Extract problem URLs from 0x3f rows
- **WHEN** a 0x3f row for the requested date contains one or more supported OJ problem URLs
- **THEN** the crawler extracts every supported URL that can be represented by stored problem refs
- **AND** supported URLs include LeetCode, AtCoder, Codeforces contest/problemset/Gym, and Luogu problem URLs
- **AND** stores ordered refs for the extracted problems

#### Scenario: Invalid 0x3f input
- **WHEN** the online sheet or local file has no requested-date rows or no supported problem URLs
- **THEN** the crawler does not write an empty daily row

### Requirement: Daily source problem snapshots
The crawler SHALL upsert minimal problem snapshots and write the compact daily row in a single SQLite transaction so the API can resolve every stored daily reference and partial daily-source writes cannot be committed. When a daily-source snapshot refers to an existing problem, the crawler SHALL merge non-empty curated metadata into sparse existing fields without clearing richer existing data.

#### Scenario: Store metadata before daily refs
- **WHEN** a daily source parser extracts problems
- **THEN** the crawler upserts those problems into `problems` before writing the `daily_challenge` row

#### Scenario: Preserve source order
- **WHEN** a daily source lists multiple problems in a specific order
- **THEN** the stored `problems` JSON array preserves that order

#### Scenario: Daily source storage is atomic
- **WHEN** daily-source problem snapshot persistence succeeds but writing the `daily_challenge` row fails
- **THEN** the crawler rolls back the transaction
- **AND** no new daily-source problem snapshots from that ingestion are committed

#### Scenario: Curated metadata fills sparse existing problem
- **WHEN** a daily source lists a problem that already exists with missing title, rating, difficulty, tags, link, or source URL fields
- **THEN** the crawler updates the missing fields from the curated daily-source snapshot
- **AND** writes the `daily_challenge` row in the same transaction

#### Scenario: Curated metadata preserves richer existing problem
- **WHEN** a daily source lists a problem that already has non-empty detail-rich metadata
- **THEN** the crawler does not clear or replace those richer fields with empty or placeholder daily-source snapshot values

### Requirement: Additional daily source scheduled refresh
The system SHALL refresh configured additional daily sources at UTC+8 08:00, 10:00, and 12:00 every day. Each refresh SHALL target the current UTC+8 date and SHALL avoid spawning duplicate source/date jobs when an equivalent job is already running.

#### Scenario: Scheduled refresh launches Sheep
- **WHEN** the server reaches UTC+8 08:00, 10:00, or 12:00
- **THEN** the system launches a crawler job with `--daily-source sheep --date <utc8-today>` unless that source/date job is already running

#### Scenario: Scheduled refresh launches 0x3f when token is configured
- **WHEN** the server reaches UTC+8 08:00, 10:00, or 12:00
- **AND** a direct local Tencent Docs token or its configured environment-variable fallback resolves to a non-empty value
- **THEN** the system launches a crawler job with `--daily-source 0x3f --date <utc8-today>` unless that source/date job is already running

#### Scenario: Scheduled refresh skips 0x3f when token is absent
- **WHEN** the server reaches UTC+8 08:00, 10:00, or 12:00
- **AND** the direct `config.toml` token and its configured environment-variable fallback are both missing or empty
- **THEN** the system does not launch the `0x3f` scheduled job
- **AND** it does not write an empty `daily_challenge` row
