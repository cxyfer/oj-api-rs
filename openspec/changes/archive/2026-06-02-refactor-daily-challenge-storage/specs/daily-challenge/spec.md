## ADDED Requirements

### Requirement: Daily challenge compact storage
The system SHALL store daily challenge records in `daily_challenge` with `date`, `source`, and `problems` columns. The `problems` column SHALL contain a JSON array of strings. Each string SHALL use `{problem_source}:{problem_id}` format and SHALL be parsed by splitting only the first colon.

#### Scenario: Store one LeetCode daily problem reference
- **WHEN** the crawler stores the leetcode.com daily challenge for `2026-01-01` with problem id `1234`
- **THEN** the database row has `date = "2026-01-01"`, `source = "leetcode.com"`, and `problems = ["leetcode:1234"]`

#### Scenario: Store multiple ordered problem references
- **WHEN** a daily source stores references `["leetcode:1234", "leetcode:1", "atcoder:abc321_a"]`
- **THEN** the system preserves that order when parsing and returning the daily response

#### Scenario: Parse problem id containing colon-like content
- **WHEN** a problem reference contains more than one colon
- **THEN** the system treats the text before the first colon as `problem_source` and the remaining text as `problem_id`

### Requirement: Daily challenge schema compatibility migration
The system SHALL detect the `daily_challenge` schema during Rust server initialization and Python crawler initialization. If the table is missing, the system SHALL create the compact schema. If the table already has compact `date`, `source`, and `problems` columns, initialization SHALL leave it unchanged. If the table has the legacy snapshot schema with `domain` and problem fields, initialization SHALL rebuild it into the compact schema while preserving convertible cached records.

#### Scenario: Create compact table in empty database
- **WHEN** initialization runs and `daily_challenge` does not exist
- **THEN** the system creates `daily_challenge(date TEXT NOT NULL, source TEXT NOT NULL, problems TEXT NOT NULL, PRIMARY KEY(date, source))`

#### Scenario: Leave compact table unchanged
- **WHEN** initialization runs and `daily_challenge` already has `date`, `source`, and `problems` columns
- **THEN** the system does not rename, drop, or rewrite the table

#### Scenario: Migrate legacy COM row
- **WHEN** initialization finds a legacy row with `date = "2026-01-01"`, `domain = "com"`, and `id = 1234`
- **THEN** the migrated row has `date = "2026-01-01"`, `source = "leetcode.com"`, and `problems = ["leetcode:1234"]`

#### Scenario: Migrate legacy CN row
- **WHEN** initialization finds a legacy row with `date = "2026-01-01"`, `domain = "cn"`, and `id = 1`
- **THEN** the migrated row has `date = "2026-01-01"`, `source = "leetcode.cn"`, and `problems = ["leetcode:1"]`

#### Scenario: Migrate legacy row by slug fallback
- **WHEN** initialization finds a legacy row with no `id` but with a `slug` that exists in `problems` for source `leetcode`
- **THEN** the system resolves the problem id from `problems` and migrates the row with a `leetcode:{id}` reference

#### Scenario: Skip unconvertible legacy row
- **WHEN** initialization finds a legacy row with neither a usable `id` nor a resolvable `slug`
- **THEN** the system skips that row and continues migrating other rows without panicking

### Requirement: Daily challenge problem localization projection
The system SHALL assemble daily response problems by resolving stored problem refs from the `problems` table and projecting localized display fields according to the daily challenge source. For `source = "leetcode.cn"`, the response problem `title` SHALL use `title_cn` when present and fallback to `title`, and `content` SHALL use `content_cn` when present and fallback to `content`. For `source = "leetcode.com"`, the response SHALL use default `title` and `content` fields.

#### Scenario: CN daily uses Chinese title and content
- **WHEN** `GET /api/v1/daily?source=leetcode.cn&date=2026-01-01` resolves `leetcode:1` whose problem row has both English and Chinese fields
- **THEN** the returned problem uses `title_cn` as `title` and `content_cn` as `content`

#### Scenario: CN daily falls back when Chinese fields are absent
- **WHEN** `GET /api/v1/daily?source=leetcode.cn&date=2026-01-01` resolves a problem whose `title_cn` or `content_cn` is empty
- **THEN** the returned problem falls back to the corresponding default `title` or `content`

#### Scenario: COM daily uses default fields
- **WHEN** `GET /api/v1/daily?source=leetcode.com&date=2026-01-01` resolves `leetcode:1`
- **THEN** the returned problem uses default `title` and `content`

#### Scenario: LeetCode link host follows daily source
- **WHEN** the daily source is `leetcode.cn` and a resolved problem source is `leetcode`
- **THEN** the returned problem link uses the `https://leetcode.cn` host

## MODIFIED Requirements

### Requirement: Daily challenge retrieval
The system SHALL return daily challenges via `GET /api/v1/daily?domain={com|cn}&date={YYYY-MM-DD}` or `GET /api/v1/daily?source={leetcode.com|leetcode.cn}&date={YYYY-MM-DD}`. The `domain` parameter SHALL be parsed as a `LeetCodeDomain` enum (`Com`, `Cn`) and mapped to canonical daily sources (`leetcode.com`, `leetcode.cn`). The response SHALL expose `date`, canonical `source`, and a `problems` array assembled from stored problem references. Each response problem SHALL include localized problem detail fields and `similar_questions` as a hydrated array of `ProblemSummary` objects resolved from the stored slug list.

#### Scenario: Today's daily (default)
- **WHEN** client sends `GET /api/v1/daily` without parameters
- **THEN** system returns today's (UTC) daily challenge for `leetcode.com` with source defaulting to `leetcode.com`

#### Scenario: Today's CN daily
- **WHEN** client sends `GET /api/v1/daily?domain=cn` without `date` parameter
- **THEN** system returns today's (UTC+8) daily challenge for `leetcode.cn`

#### Scenario: Specific date
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-01-15`
- **THEN** system returns the daily challenge for that specific date from `leetcode.com`

#### Scenario: CN domain
- **WHEN** client sends `GET /api/v1/daily?domain=cn`
- **THEN** system returns the daily challenge from `leetcode.cn`

#### Scenario: Source alias leetcode.cn
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.cn`
- **THEN** system returns the same response as `?domain=cn`

#### Scenario: Source alias leetcode.com
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.com`
- **THEN** system returns the same response as `?domain=com`

#### Scenario: Domain takes precedence over source when equal
- **WHEN** client sends `GET /api/v1/daily?domain=cn&source=leetcode.cn`
- **THEN** system returns the daily challenge from `leetcode.cn` without conflict

#### Scenario: Daily response includes hydrated similar questions
- **WHEN** a resolved daily problem has similar question slugs that exist in the LeetCode problem table
- **THEN** the response problem returns `similar_questions` as hydrated summary objects in the same order as the stored slug list

#### Scenario: Daily response shape
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.com&date=2026-01-01` and a daily record exists
- **THEN** system returns HTTP 200 with a JSON object containing `date`, `source`, and `problems`, and does not return top-level problem fields such as `id`, `slug`, or `title`

#### Scenario: Multiple daily problems preserve order
- **WHEN** the stored daily record has `problems = ["leetcode:1234", "leetcode:1", "atcoder:abc321_a"]`
- **THEN** the response `problems` array returns resolved problems in that same reference order

### Requirement: Daily challenge domain validation
The system SHALL only accept `com` or `cn` as valid domain values, validated via the `LeetCodeDomain` enum. The `source` parameter SHALL only accept `leetcode.com` or `leetcode.cn` for the LeetCode daily endpoint. If both `domain` and `source` are provided with conflicting values, the system SHALL return HTTP 400.

#### Scenario: Invalid domain
- **WHEN** client sends `GET /api/v1/daily?domain=jp`
- **THEN** system returns HTTP 400 with error detail indicating invalid domain

#### Scenario: Invalid source
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.jp`
- **THEN** system returns HTTP 400 with error detail indicating invalid source value

#### Scenario: Conflicting domain and source
- **WHEN** client sends `GET /api/v1/daily?domain=com&source=leetcode.cn`
- **THEN** system returns HTTP 400 with error detail indicating domain and source conflict

### Requirement: Daily challenge not found
The system SHALL return HTTP 404 only when no usable daily challenge record exists in the DB AND no fallback crawler can be triggered. A daily row with malformed JSON, malformed problem refs, or no resolvable problems SHALL be treated as unusable. When a fallback crawler is triggered, the system SHALL return HTTP 202 instead.

#### Scenario: No data, fallback triggered (com)
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-06-15` and no DB record exists and no fallback is running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` and spawns background crawler

#### Scenario: No data, fallback triggered (cn)
- **WHEN** client sends `GET /api/v1/daily?domain=cn` and no DB record exists for today (UTC+8) and no fallback is running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` and spawns background crawler with `--domain cn`

#### Scenario: Malformed stored refs treated as unusable
- **WHEN** the DB has a daily row whose `problems` column is invalid JSON or contains no valid problem references
- **THEN** the system treats the row as unusable and follows the existing no-data fallback behavior

#### Scenario: Missing referenced problems treated as unusable when none resolve
- **WHEN** the DB has a daily row but none of its problem references resolve from the `problems` table
- **THEN** the system treats the row as unusable and follows the existing no-data fallback behavior

### Requirement: CN daily challenge fallback
The system SHALL trigger a background crawler fallback for `domain=cn` or `source=leetcode.cn` when no usable DB record exists, using the same TOCTOU guard, cooldown, and background task pattern as `domain=com`. The fallback key SHALL use the canonical daily source and date to prevent cross-source cooldown collision. The crawler SHALL be spawned with `--domain cn` argument.

#### Scenario: CN today fallback
- **WHEN** client sends `GET /api/v1/daily?domain=cn` with no usable DB data and no active fallback
- **THEN** system inserts fallback entry with key `leetcode.cn:{today_utc8}`, spawns `uv run python3 leetcode.py --daily --domain cn`, and returns HTTP 202

#### Scenario: CN historical fallback
- **WHEN** client sends `GET /api/v1/daily?domain=cn&date=2024-11-15` with no usable DB data and no active fallback
- **THEN** system inserts fallback entry with key `leetcode.cn:2024-11-15`, spawns `uv run python3 leetcode.py --date 2024-11-15 --domain cn`, and returns HTTP 202

#### Scenario: CN fallback already running
- **WHEN** client sends `GET /api/v1/daily?domain=cn` and a fallback for `leetcode.cn:{today}` is already Running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` without spawning a new crawler

#### Scenario: CN and COM fallback independent
- **WHEN** fallback for `leetcode.com:2024-11-15` is Running and client sends `GET /api/v1/daily?domain=cn&date=2024-11-15`
- **THEN** system spawns a separate cn fallback with key `leetcode.cn:2024-11-15` because the keys are independent

#### Scenario: Fallback completed, data available
- **WHEN** client sends `GET /api/v1/daily?domain=cn` after the cn fallback crawler has completed successfully and written compact daily data
- **THEN** system returns HTTP 200 with `{ date, source, problems }` from DB
