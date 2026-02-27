## 1. Rust: LeetCodeDomain Enum

- [x] 1.1 Add `LeetCodeDomain` enum (`Com`, `Cn`) to `src/models.rs` with `Display`, `FromStr`, `Deserialize` impls
- [x] 1.2 Add `LeetCodeDomain::today()` method returning domain-aware date string (UTC for Com, UTC+8 for Cn)
- [x] 1.3 Add `--domain` to `LEETCODE_ARGS` whitelist in `src/models.rs` (`arity=1`, `value_type=Str`, `ui_exposed=true`)

## 2. Rust: DailyQuery + Source Alias

- [x] 2.1 Add `source: Option<String>` field to `DailyQuery` struct in `src/api/daily.rs`
- [x] 2.2 Implement `resolve_domain(domain, source) -> Result<LeetCodeDomain, ProblemDetail>` normalization function: `leetcode.com` → `Com`, `leetcode.cn` → `Cn`, conflict → 400, default → `Com`
- [x] 2.3 Replace string-based domain validation in `get_daily` handler with `resolve_domain` call

## 3. Rust: Domain-Aware Fallback

- [x] 3.1 Replace hardcoded `today` (UTC) with `domain.today()` for default date and upper bound validation
- [x] 3.2 Change fallback key from `format!("com:{}", date)` to `format!("{}:{}", domain, date)`
- [x] 3.3 Remove `None if domain != "com" => 404` guard — allow cn to enter fallback path
- [x] 3.4 Add `--domain` arg to fallback CLI args: `["--daily", "--domain", domain]` for today, `["--date", date, "--domain", domain]` for historical
- [x] 3.5 Use `domain.today()` instead of `Utc::now()` for `--daily` vs `--date` determination in fallback arg construction

## 4. Python: CLI --domain Argument

- [x] 4.1 Add `--domain` argument to argparse (`choices=["com", "cn"]`, `default="com"`)
- [x] 4.2 Change `LeetCodeClient` instantiation from `LeetCodeClient(data_dir=..., db_path=...)` to `LeetCodeClient(domain=args.domain, data_dir=..., db_path=...)`
- [x] 4.3 Add `sys.exit(2)` when `--daily` or `--date` result is `None` (with stderr message)

## 5. Python: CN Monthly Fetch

- [x] 5.1 Implement `fetch_monthly_daily_challenges_cn(year, month)` method using `dailyQuestionRecords` GraphQL query with `operation-name: dailyQuestionRecords` header against `leetcode.cn/graphql/`
- [x] 5.2 Normalize cn monthly response to match the internal data structure used by `get_daily_challenge` (apply `acRate * 100` for cn)
- [x] 5.3 Remove `if domain == "com"` guard in `get_daily_challenge()` line 761 — route to cn monthly method when `domain == "cn"`

## 6. Python: Bug Fixes

- [x] 6.1 Fix `get_daily_challenge()` line 698-702: change `self.domain` to `domain` for timezone resolution
- [x] 6.2 Fix `get_daily_challenge()` line 757: change `self.fetch_daily_challenge(self.domain)` to `self.fetch_daily_challenge(domain)`

## 7. Verification

- [x] 7.1 `cargo build --release` compiles without errors
- [x] 7.2 `cargo clippy` passes without warnings
- [x] 7.3 Manual test: `GET /api/v1/daily?domain=cn` returns 202 on first call (fallback triggered)
- [x] 7.4 Manual test: `GET /api/v1/daily?source=leetcode.cn` returns same as `?domain=cn`
- [x] 7.5 Manual test: `GET /api/v1/daily?domain=com&source=leetcode.cn` returns 400 (conflict)
- [x] 7.6 Manual test: `python3 leetcode.py --daily --domain cn` fetches from leetcode.cn and writes to DB
- [x] 7.7 Manual test: `python3 leetcode.py --monthly 2026 2 --domain cn` fetches cn monthly data
- [x] 7.8 Verify no regression: `GET /api/v1/daily` (no params) returns com daily as before
