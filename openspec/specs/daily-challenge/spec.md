# daily-challenge Specification

## Purpose
TBD - created by archiving change oj-api-rs-v1. Update Purpose after archive.
## Requirements
### Requirement: Daily challenge retrieval
The system SHALL return the LeetCode daily challenge via `GET /api/v1/daily?domain={com|cn}&date={YYYY-MM-DD}`. The `domain` parameter SHALL be parsed as a `LeetCodeDomain` enum (`Com`, `Cn`). An optional `source` parameter SHALL be accepted as an alias (`leetcode.com` → `Com`, `leetcode.cn` → `Cn`).

#### Scenario: Today's daily (default)
- **WHEN** client sends `GET /api/v1/daily` without parameters
- **THEN** system returns today's (UTC) daily challenge for leetcode.com with domain defaulting to `Com`

#### Scenario: Today's CN daily
- **WHEN** client sends `GET /api/v1/daily?domain=cn` without `date` parameter
- **THEN** system returns today's (UTC+8) daily challenge for leetcode.cn

#### Scenario: Specific date
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-01-15`
- **THEN** system returns the daily challenge for that specific date from leetcode.com

#### Scenario: CN domain
- **WHEN** client sends `GET /api/v1/daily?domain=cn`
- **THEN** system returns the daily challenge from leetcode.cn

#### Scenario: Source alias leetcode.cn
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.cn`
- **THEN** system returns the same response as `?domain=cn`

#### Scenario: Source alias leetcode.com
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.com`
- **THEN** system returns the same response as `?domain=com`

#### Scenario: Domain takes precedence over source when equal
- **WHEN** client sends `GET /api/v1/daily?domain=cn&source=leetcode.cn`
- **THEN** system returns the daily challenge from leetcode.cn (no conflict)

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
The system SHALL only accept `com` or `cn` as valid domain values, validated via the `LeetCodeDomain` enum. The `source` parameter SHALL only accept `leetcode.com` or `leetcode.cn`. If both `domain` and `source` are provided with conflicting values, the system SHALL return HTTP 400.

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
The system SHALL return HTTP 404 only when no daily challenge record exists in the DB AND no fallback crawler can be triggered. When a fallback crawler is triggered, the system SHALL return HTTP 202 instead.

#### Scenario: No data, fallback triggered (com)
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-06-15` and no DB record exists and no fallback is running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` and spawns background crawler

#### Scenario: No data, fallback triggered (cn)
- **WHEN** client sends `GET /api/v1/daily?domain=cn` and no DB record exists for today (UTC+8) and no fallback is running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` and spawns background crawler with `--domain cn`

### Requirement: CN daily challenge fallback
The system SHALL trigger a background crawler fallback for `domain=cn` when no DB record exists, using the same TOCTOU guard, cooldown, and background task pattern as `domain=com`. The fallback key SHALL be `{domain}:{date}` to prevent cross-domain cooldown collision. The crawler SHALL be spawned with `--domain cn` argument.

#### Scenario: CN today fallback
- **WHEN** client sends `GET /api/v1/daily?domain=cn` with no DB data and no active fallback
- **THEN** system inserts fallback entry with key `cn:{today_utc8}`, spawns `uv run python3 leetcode.py --daily --domain cn`, and returns HTTP 202

#### Scenario: CN historical fallback
- **WHEN** client sends `GET /api/v1/daily?domain=cn&date=2024-11-15` with no DB data and no active fallback
- **THEN** system inserts fallback entry with key `cn:2024-11-15`, spawns `uv run python3 leetcode.py --date 2024-11-15 --domain cn`, and returns HTTP 202

#### Scenario: CN fallback already running
- **WHEN** client sends `GET /api/v1/daily?domain=cn` and a fallback for `cn:{today}` is already Running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` without spawning a new crawler

#### Scenario: CN and COM fallback independent
- **WHEN** fallback for `com:2024-11-15` is Running and client sends `GET /api/v1/daily?domain=cn&date=2024-11-15`
- **THEN** system spawns a separate cn fallback (key `cn:2024-11-15`) because the keys are independent

#### Scenario: Fallback completed, data available
- **WHEN** client sends `GET /api/v1/daily?domain=cn` after the cn fallback crawler has completed successfully
- **THEN** system returns HTTP 200 with the cn daily challenge data from DB

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
The system SHALL accept an optional `?wait=true` query parameter on `GET /api/v1/daily`.
When `wait=true`, the handler SHALL await the background crawler's completion (up to 10 s)
before responding. If the crawler completes within 10 s and the DB row exists, the system
SHALL return HTTP 200 with the challenge data. If the crawler fails, times out, or the DB
row is still absent after notification, the system SHALL return HTTP 202.

When `wait` is omitted or `false`, existing behavior is unchanged.

#### Scenario: Wait succeeds — new crawler finishes in time
- **WHEN** client sends `GET /api/v1/daily?wait=true` and no DB row exists
- **THEN** system spawns the crawler, awaits notification (≤10 s), reads DB, and returns HTTP 200 with challenge data

#### Scenario: Wait succeeds — joins existing crawler
- **WHEN** a crawler for the same key is already `Running` and client sends `GET /api/v1/daily?wait=true`
- **THEN** system joins the existing `Notify` (no second crawler spawned), awaits notification (≤10 s), and returns HTTP 200 if DB row exists

#### Scenario: Wait times out
- **WHEN** client sends `GET /api/v1/daily?wait=true` and the crawler does not complete within 10 s
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}`

#### Scenario: Crawler fails during wait
- **WHEN** client sends `GET /api/v1/daily?wait=true` and the crawler exits with non-zero status
- **THEN** system receives the notification, reads DB (finds nothing), and returns HTTP 202

#### Scenario: Spawn failure during wait
- **WHEN** client sends `GET /api/v1/daily?wait=true` and `uv run python3 leetcode.py` fails to spawn
- **THEN** system calls `notify_waiters()` in the failure path, and the waiting handler returns HTTP 202

#### Scenario: No-wait behavior unchanged
- **WHEN** client sends `GET /api/v1/daily` without `?wait` parameter
- **THEN** system returns HTTP 202 immediately upon triggering crawler, identical to pre-change behavior

#### Scenario: Two concurrent wait requests share one crawler
- **WHEN** two concurrent requests both send `GET /api/v1/daily?wait=true` for the same key
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

