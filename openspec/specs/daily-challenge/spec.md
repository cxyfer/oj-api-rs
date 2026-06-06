# daily-challenge Specification

## Purpose
TBD - created by archiving change oj-api-rs-v1. Update Purpose after archive.
## Requirements
### Requirement: Daily challenge retrieval
The system SHALL return daily challenges via `GET /api/v1/daily?domain={com|cn}&date={YYYY-MM-DD}` or `GET /api/v1/daily?source={daily-source}&date={YYYY-MM-DD}`. The `domain` parameter SHALL remain LeetCode-only, parsed as a `LeetCodeDomain` enum (`Com`, `Cn`), and mapped to canonical daily sources (`leetcode.com`, `leetcode.cn`). The `source` parameter SHALL accept LeetCode daily sources (`leetcode.com`, `leetcode.cn`) and known additional daily challenge sources (`sheep`, `0x3f`). The response SHALL expose `date`, canonical `source`, and a `problems` array assembled from stored problem references. Each response problem SHALL include problem detail fields and `similar_questions` as a hydrated array of `ProblemSummary` objects resolved from the stored slug list.

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
- **WHEN** a resolved daily problem has similar question slugs that exist in the problem table for that problem source
- **THEN** the response problem returns `similar_questions` as hydrated summary objects in the same order as the stored slug list

#### Scenario: Daily response shape
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.com&date=2026-01-01` and a daily record exists
- **THEN** system returns HTTP 200 with a JSON object containing `date`, `source`, and `problems`, and does not return top-level problem fields such as `id`, `slug`, or `title`

#### Scenario: Multiple daily problems preserve order
- **WHEN** the stored daily record has `problems = ["leetcode:1234", "leetcode:1", "atcoder:abc321_a"]`
- **THEN** the response `problems` array returns resolved problems in that same reference order

#### Scenario: Sheep daily source
- **WHEN** client sends `GET /api/v1/daily?source=sheep&date=2026-06-02` and a daily record exists
- **THEN** system returns HTTP 200 with `source = "sheep"` and resolved Codeforces problems

#### Scenario: 0x3f daily source
- **WHEN** client sends `GET /api/v1/daily?source=0x3f&date=2026-06-02` and a daily record exists
- **THEN** system returns HTTP 200 with `source = "0x3f"` and resolved Codeforces problems

### Requirement: Daily challenge date validation
The system SHALL validate the `date` parameter format as `YYYY-MM-DD` and enforce range `[2020-04-01, domain-aware today]`. For `domain=cn`, "today" SHALL be computed using UTC+8. For `domain=com`, "today" SHALL be computed using UTC.

#### Scenario: Date before lower bound
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2019-01-01`
- **THEN** system returns HTTP 400 with error detail indicating date must be >= 2020-04-01

#### Scenario: Future date (com)
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2099-01-01`
- **THEN** system returns HTTP 400 with error detail indicating date must be <= today (UTC)

#### Scenario: Future date (cn, timezone edge)
- **WHEN** client sends `GET /api/v1/daily?domain=cn` at 01:00 UTC (09:00 UTC+8) and the cn date has already advanced
- **THEN** system uses UTC+8 "today" as the default date and upper bound, not UTC

#### Scenario: Invalid date format
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=01-15-2024`
- **THEN** system returns HTTP 400 with error detail indicating invalid date format

#### Scenario: Invalid calendar date
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-02-30`
- **THEN** system returns HTTP 400 with error detail indicating invalid date

### Requirement: Daily challenge domain validation
The system SHALL only accept `com` or `cn` as valid domain values, validated via the `LeetCodeDomain` enum. The `source` parameter SHALL only accept supported daily source values. If both `domain` and `source` are provided, the system SHALL allow matching LeetCode pairs and SHALL return HTTP 400 for conflicts or for pairing a LeetCode-only `domain` with a non-LeetCode `source`.

#### Scenario: Invalid domain
- **WHEN** client sends `GET /api/v1/daily?domain=jp`
- **THEN** system returns HTTP 400 with error detail indicating invalid domain

#### Scenario: Invalid source
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.jp`
- **THEN** system returns HTTP 400 with error detail indicating invalid source value

#### Scenario: Conflicting domain and source
- **WHEN** client sends `GET /api/v1/daily?domain=com&source=leetcode.cn`
- **THEN** system returns HTTP 400 with error detail indicating domain and source conflict

#### Scenario: Domain conflicts with additional daily source
- **WHEN** client sends `GET /api/v1/daily?domain=com&source=sheep`
- **THEN** system returns HTTP 400 with error detail indicating domain and source conflict

### Requirement: Daily challenge not found
The system SHALL return HTTP 404 only when no usable daily challenge record exists in the DB AND no fallback behavior applies. A daily row with malformed JSON, malformed problem refs, or no resolvable problems SHALL be treated as unusable. When fallback behavior applies, the system SHALL return HTTP 202 instead. LeetCode sources SHALL keep spawning the LeetCode fallback crawler. Additional daily sources SHALL return the HTTP 202 fetching response without spawning a crawler from the API handler.

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

#### Scenario: Missing additional daily source does not spawn API fallback crawler
- **WHEN** client sends `GET /api/v1/daily?source=sheep&date=2026-06-02` and no usable DB record exists
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}`
- **AND** the API handler does not spawn `leetcode.py` or `codeforces.py`

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

### Requirement: CN monthly daily challenges fetch
The Python crawler SHALL support fetching monthly daily challenges for leetcode.cn using the `dailyQuestionRecords(year, month)` GraphQL query at `https://leetcode.cn/graphql/`. The request SHALL include the `operation-name: dailyQuestionRecords` header.

#### Scenario: CN monthly fetch for current month
- **WHEN** crawler is invoked with `--monthly 2026 2 --domain cn`
- **THEN** crawler sends `dailyQuestionRecords` query with `{"year": 2026, "month": 2}` to `leetcode.cn/graphql/` and stores all returned daily challenges with `domain=cn`

#### Scenario: CN monthly fetch for historical month
- **WHEN** crawler is invoked with `--monthly 2024 11 --domain cn`
- **THEN** crawler fetches and stores all daily challenges for November 2024 from leetcode.cn

#### Scenario: CN historical date triggers monthly fetch
- **WHEN** `get_daily_challenge(date_str="2024-11-15", domain="cn")` finds no DB record and no file
- **THEN** crawler calls `fetch_monthly_daily_challenges_cn(2024, 11)` to batch-fetch the month, then returns the requested date's challenge

### Requirement: Crawler --domain CLI argument
The Python crawler CLI SHALL accept a `--domain` argument with choices `com` and `cn` (default: `com`). The argument SHALL be passed to `LeetCodeClient(domain=args.domain)` at initialization. The `--domain` flag SHALL be included in the Rust `LEETCODE_ARGS` whitelist with `arity=1` and `value_type=Str`.

#### Scenario: CLI with --domain cn --daily
- **WHEN** `python3 leetcode.py --daily --domain cn` is executed
- **THEN** `LeetCodeClient` is instantiated with `domain="cn"` and fetches today's challenge from leetcode.cn

#### Scenario: CLI with --domain cn --date
- **WHEN** `python3 leetcode.py --date 2024-11-15 --domain cn` is executed
- **THEN** `LeetCodeClient` is instantiated with `domain="cn"` and fetches the challenge for 2024-11-15

#### Scenario: CLI with --domain cn --monthly
- **WHEN** `python3 leetcode.py --monthly 2024 11 --domain cn` is executed
- **THEN** crawler fetches monthly daily challenges from leetcode.cn for November 2024

#### Scenario: CLI default domain
- **WHEN** `python3 leetcode.py --daily` is executed without `--domain`
- **THEN** `LeetCodeClient` is instantiated with `domain="com"` (backward compatible)

#### Scenario: CLI result None exits non-zero
- **WHEN** `--daily` or `--date` returns `None` from the crawler
- **THEN** CLI exits with code 2 and prints error to stderr

#### Scenario: Rust arg whitelist accepts --domain
- **WHEN** `validate_args` is called with `["--daily", "--domain", "cn"]` against `LEETCODE_ARGS`
- **THEN** validation passes without error

### Requirement: LeetCodeDomain enum
The Rust codebase SHALL define a `LeetCodeDomain` enum with variants `Com` and `Cn` in `src/models.rs`. The enum SHALL implement `Display` (outputting `"com"` / `"cn"`), `FromStr`, and `Deserialize`. All domain string comparisons in `daily.rs` SHALL be replaced with enum matching.

#### Scenario: Deserialize from query param
- **WHEN** query string contains `domain=cn`
- **THEN** `LeetCodeDomain::Cn` is produced

#### Scenario: Display for fallback key
- **WHEN** `format!("{}:{}", LeetCodeDomain::Cn, "2024-11-15")` is called
- **THEN** result is `"cn:2024-11-15"`

#### Scenario: Invalid value rejected
- **WHEN** `"jp".parse::<LeetCodeDomain>()` is called
- **THEN** an error is returned

### Requirement: Domain-aware timezone resolution
The Rust handler SHALL compute "today" using UTC+8 for `domain=cn` and UTC for `domain=com`. This SHALL affect: default date when `?date` is omitted, upper bound for date validation, and `--daily` vs `--date` determination in fallback arg construction.

#### Scenario: CN today at UTC midnight
- **WHEN** current time is 2024-11-16 01:00 UTC (2024-11-16 09:00 UTC+8) and client sends `GET /api/v1/daily?domain=cn`
- **THEN** system uses `2024-11-16` as the default date (UTC+8 today)

#### Scenario: COM today at UTC midnight
- **WHEN** current time is 2024-11-16 01:00 UTC and client sends `GET /api/v1/daily?domain=com`
- **THEN** system uses `2024-11-16` as the default date (UTC today)

### Requirement: Python get_daily_challenge domain parameter fix
The `get_daily_challenge()` method SHALL use the local `domain` parameter (not `self.domain`) for timezone resolution and for calling `fetch_daily_challenge()`. This fixes the existing bug where the method ignores its `domain` argument.

#### Scenario: Timezone uses domain parameter
- **WHEN** `LeetCodeClient(domain="com")` calls `get_daily_challenge(domain="cn")`
- **THEN** timezone is resolved as UTC+8 (from the `domain="cn"` parameter), not UTC (from `self.domain="com"`)

#### Scenario: Fetch uses domain parameter
- **WHEN** `get_daily_challenge(domain="cn")` needs to fetch today's challenge
- **THEN** it calls `fetch_daily_challenge(domain="cn")`, not `fetch_daily_challenge(self.domain)`

### Requirement: Daily challenge wait-for-result
The system SHALL accept an optional `?async=true` query parameter on `GET /api/v1/daily`.
By default (when `async` is omitted or `false`), the handler SHALL await the background crawler's
completion (up to 10 s) before responding. If the crawler completes within 10 s and the DB row
exists, the system SHALL return HTTP 200 with the challenge data. If the crawler fails, times out,
or the DB row is still absent after notification, the system SHALL return HTTP 202.

When `async=true`, the handler returns HTTP 202 immediately upon triggering the crawler without waiting.

#### Scenario: Default behavior — waits for crawler
- **WHEN** client sends `GET /api/v1/daily` (no `?async` parameter) and no DB row exists
- **THEN** system spawns the crawler, awaits notification (≤10 s), reads DB, and returns HTTP 200 with challenge data

#### Scenario: Wait succeeds — joins existing crawler
- **WHEN** a crawler for the same key is already `Running` and client sends `GET /api/v1/daily`
- **THEN** system joins the existing `Notify` (no second crawler spawned), awaits notification (≤10 s), and returns HTTP 200 if DB row exists

#### Scenario: Wait times out
- **WHEN** client sends `GET /api/v1/daily` and the crawler does not complete within 10 s
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}`

#### Scenario: Crawler fails during wait
- **WHEN** client sends `GET /api/v1/daily` and the crawler exits with non-zero status
- **THEN** system receives the notification, reads DB (finds nothing), and returns HTTP 202

#### Scenario: Spawn failure during wait
- **WHEN** client sends `GET /api/v1/daily` and `uv run python3 leetcode.py` fails to spawn
- **THEN** system calls `notify_waiters()` in the failure path, and the waiting handler returns HTTP 202

#### Scenario: Async mode — immediate return
- **WHEN** client sends `GET /api/v1/daily?async=true`
- **THEN** system returns HTTP 202 immediately upon triggering crawler without waiting

#### Scenario: Two concurrent default requests share one crawler
- **WHEN** two concurrent requests both send `GET /api/v1/daily` for the same key
- **THEN** only one crawler is spawned; both requests await the same `Notify` and both receive the result

### Requirement: DailyFallbackEntry notify field
`DailyFallbackEntry` in `src/models.rs` SHALL include a `notify: Arc<tokio::sync::Notify>`
field. The field SHALL be initialised with `Arc::new(Notify::new())` at entry creation.
All completion paths (success, failure, timeout, spawn error) SHALL call
`entry.notify.notify_waiters()` to unblock any waiting handlers.

#### Scenario: Notify initialised at entry creation
- **WHEN** a new `DailyFallbackEntry` is inserted into `state.daily_fallback`
- **THEN** its `notify` field is a fresh `Arc<Notify>` (not shared from a previous entry)

#### Scenario: notify_waiters called on crawler success
- **WHEN** the background crawler exits with status 0
- **THEN** `entry.notify.notify_waiters()` is called before cleanup sleep

#### Scenario: notify_waiters called on spawn failure
- **WHEN** `spawn_with_pgid(cmd)` returns `Err(_)`
- **THEN** `entry.notify.notify_waiters()` is called within the error handler

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
The system SHALL assemble daily response problems by resolving stored problem refs from the `problems` table and projecting display fields according to the daily challenge source. For `source = "leetcode.cn"`, the response problem `title` SHALL use `title_cn` when present and fallback to `title`, and `content` SHALL use `content_cn` when present and fallback to `content`. For all other daily sources, including `leetcode.com`, `sheep`, and `0x3f`, the response SHALL use default `title` and `content` fields. LeetCode host rewriting SHALL apply only when the resolved problem source is `leetcode` and the daily source is `leetcode.com` or `leetcode.cn`.

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

#### Scenario: additional daily source preserves Codeforces link
- **WHEN** the daily source is `sheep` and a resolved problem source is `codeforces`
- **THEN** the returned problem link remains the stored Codeforces link

