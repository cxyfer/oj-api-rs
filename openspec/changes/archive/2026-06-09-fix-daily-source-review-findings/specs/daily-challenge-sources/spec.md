## MODIFIED Requirements

### Requirement: Daily source problem snapshots
The crawler SHALL upsert minimal Codeforces problem snapshots and write the compact daily row in a single SQLite transaction so the API can resolve every stored daily reference and partial daily-source writes cannot be committed. When a daily-source snapshot refers to an existing Codeforces problem, the crawler SHALL merge non-empty curated metadata into sparse existing fields without clearing richer existing data.

#### Scenario: Store metadata before daily refs
- **WHEN** a daily source parser extracts Codeforces problems
- **THEN** the crawler upserts those problems into `problems` with `source = "codeforces"` before writing the `daily_challenge` row

#### Scenario: Preserve source order
- **WHEN** a daily source lists multiple problems in a specific order
- **THEN** the stored `problems` JSON array preserves that order

#### Scenario: Daily source storage is atomic
- **WHEN** daily-source problem snapshot persistence succeeds but writing the `daily_challenge` row fails
- **THEN** the crawler rolls back the transaction
- **AND** no new daily-source problem snapshots from that ingestion are committed

#### Scenario: Curated metadata fills sparse existing problem
- **WHEN** a daily source lists a Codeforces problem that already exists with missing title, rating, difficulty, tags, link, or source URL fields
- **THEN** the crawler updates the missing fields from the curated daily-source snapshot
- **AND** writes the `daily_challenge` row in the same transaction

#### Scenario: Curated metadata preserves richer existing problem
- **WHEN** a daily source lists a Codeforces problem that already has non-empty detail-rich metadata
- **THEN** the crawler does not clear or replace those richer fields with empty or placeholder daily-source snapshot values
