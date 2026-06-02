## 1. Schema Migration

- [x] 1.1 Replace Rust `daily_challenge` creation with compact `date`, `source`, `problems` schema
- [x] 1.2 Add Rust schema detection for missing, compact, and legacy `daily_challenge` tables
- [x] 1.3 Add Rust legacy rebuild migration that converts `domain/id` rows to JSON problem refs
- [x] 1.4 Add Rust migration tests for empty DB, compact DB no-op, COM/CN legacy rows, slug fallback, and unconvertible rows
- [x] 1.5 Replace Python `DailyChallengeDatabaseManager._init_db` schema creation with matching compact schema
- [x] 1.6 Add Python schema detection and data-preserving legacy rebuild migration

## 2. Daily Persistence

- [x] 2.1 Update Python `update_daily` to write `date`, canonical daily `source`, and JSON `problems` refs only
- [x] 2.2 Map LeetCode crawler domain `com` to daily source `leetcode.com` and `cn` to `leetcode.cn`
- [x] 2.3 Ensure crawler still updates `problems` with detail data before storing daily refs
- [x] 2.4 Add Python tests or focused checks for compact daily rows and JSON ref normalization

## 3. Rust Daily Read Model

- [x] 3.1 Replace `DailyChallengeRecord` snapshot fields with compact daily record plus resolved problem list models
- [x] 3.2 Add helpers to parse `{problem_source}:{problem_id}` refs by splitting only the first colon
- [x] 3.3 Resolve parsed refs from `problems(source, id)` while preserving stored order
- [x] 3.4 Treat malformed JSON, malformed refs, and rows with no resolvable problems as unusable daily data
- [x] 3.5 Add Rust DB tests for ordered multi-ref resolution and unusable row behavior

## 4. API Response Refactor

- [x] 4.1 Replace `/api/v1/daily` response schema with `{ date, source, problems }`
- [x] 4.2 Preserve `domain=com|cn` and `source=leetcode.com|leetcode.cn` query aliases while using canonical daily source internally
- [x] 4.3 Project `leetcode.cn` daily problems with `title_cn` / `content_cn` fallback logic
- [x] 4.4 Project `leetcode.com` daily problems with default title/content and LeetCode COM links
- [x] 4.5 Rewrite LeetCode response links according to daily source host when problem source is `leetcode`
- [x] 4.6 Keep hydrated `similar_questions` on each response problem
- [x] 4.7 Update fallback runtime keys to use canonical daily source plus date
- [x] 4.8 Update OpenAPI component schemas to reflect the breaking daily response shape

## 5. Verification

- [x] 5.1 Update API integration tests for new daily response shape and absence of old top-level problem fields
- [x] 5.2 Add CN localization API test covering title/content fallback behavior
- [x] 5.3 Add fallback tests for canonical source runtime keys and compact row readback after crawler completion
- [x] 5.4 Run `cargo fmt`
- [x] 5.5 Run `cargo test`
- [x] 5.6 Run relevant Python checks for daily persistence if Python tests are available
