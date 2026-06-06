## ADDED Requirements

### Requirement: Sheep daily source ingestion
The Codeforces crawler SHALL ingest Sheep daily Codeforces problems from the public GitHub raw Markdown path for a requested date and SHALL store the compact daily row with source `sheep`.

#### Scenario: Ingest Sheep daily markdown
- **WHEN** `codeforces.py` is invoked with `--daily-source sheep --date 2026-06-02`
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

### Requirement: 0x3f stable export ingestion
The Codeforces crawler SHALL ingest 0x3f daily problems only from stable downloaded/exported local tabular input and SHALL store the compact daily row with source `0x3f`.

#### Scenario: Ingest 0x3f local CSV export
- **WHEN** `codeforces.py` is invoked with `--daily-source 0x3f --date 2026-06-02 --daily-file <path>`
- **THEN** the crawler reads the local tabular file
- **AND** stores a `daily_challenge` row with `date = "2026-06-02"` and `source = "0x3f"`

#### Scenario: Extract Codeforces URLs from 0x3f rows
- **WHEN** a 0x3f row for the requested date contains one or more Codeforces problem URLs
- **THEN** the crawler extracts every supported Codeforces URL and stores ordered `codeforces:<id>` refs

#### Scenario: Reject unstable 0x3f scraping
- **WHEN** no `--daily-file` is provided for `--daily-source 0x3f`
- **THEN** the crawler fails with a clear error instead of scraping Tencent Docs UI or private endpoints

#### Scenario: Invalid 0x3f input
- **WHEN** the 0x3f local file has no requested-date rows or no Codeforces URLs
- **THEN** the crawler does not write an empty daily row

### Requirement: Daily source problem snapshots
The crawler SHALL upsert minimal Codeforces problem snapshots before writing daily refs so the API can resolve every stored daily reference.

#### Scenario: Store metadata before daily refs
- **WHEN** a daily source parser extracts Codeforces problems
- **THEN** the crawler upserts those problems into `problems` with `source = "codeforces"` before writing the `daily_challenge` row

#### Scenario: Preserve source order
- **WHEN** a daily source lists multiple problems in a specific order
- **THEN** the stored `problems` JSON array preserves that order
