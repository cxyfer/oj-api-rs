# Proposal: LeetCode CN Daily Challenge Support

## Context

The `/api/v1/daily` endpoint currently supports `?domain=com|cn` at the Rust handler level,
but the fallback crawler (`src/api/daily.rs:65-70`) only triggers for `domain=com`.
When `domain=cn` data is absent from the DB, it returns 404 immediately without attempting
to fetch from leetcode.cn.

The Python crawler (`scripts/leetcode.py`) already supports `domain=cn` for today's challenge
via `fetch_daily_challenge(domain="cn")`, but:
1. The CLI has no `--domain` argument — `LeetCodeClient` is always instantiated with default `domain="com"` (line 1458)
2. `fetch_monthly_daily_challenges()` explicitly rejects `domain != "com"` (line 992)
3. `get_daily_challenge()` for historical dates falls back to monthly fetch, which is com-only

---

## Constraint Set

### Hard Constraints

| # | Constraint | Source |
|---|---|---|
| H1 | `fetch_monthly_daily_challenges` is com-only; leetcode.cn uses a different GraphQL query `dailyQuestionRecords(year, month)` instead of `dailyCodingChallengeV2` | `scripts/leetcode.py:992`, `design.md:D6` |
| H2 | leetcode.cn today's challenge uses `todayRecord` query; com uses `activeDailyCodingChallengeQuestion` | `scripts/leetcode.py:564-618` |
| H3 | leetcode.cn `acRate` is returned as a fraction (0–1), must multiply ×100; com returns percentage directly | `scripts/leetcode.py:661-663` |
| H4 | Rust fallback key is hardcoded as `"com:{date}"` — cn fallback path is entirely absent | `src/api/daily.rs:73` |
| H5 | CLI instantiates `LeetCodeClient(domain="com")` unconditionally; no `--domain` arg exists | `scripts/leetcode.py:1458` |
| H6 | `DailyQuery` struct already has `domain: Option<String>` field; Rust handler already validates `com|cn` | `src/api/daily.rs:12-24` |
| H7 | DB schema uses `(domain, date)` composite key in `daily_challenge` table | `src/db/daily.rs:14` |
| H8 | `source` query param alias is NOT currently implemented — only `domain` exists | `src/api/daily.rs:11-15` |

### Soft Constraints

| # | Constraint | Source |
|---|---|---|
| S1 | Fallback for cn should mirror com fallback pattern: TOCTOU guard + cooldown + background spawn | `src/api/daily.rs:76-107` |
| S2 | ~~Superseded~~ — leetcode.cn does have a monthly API (`dailyQuestionRecords`); historical cn dates can be batch-fetched per month, symmetric with com | `design.md:D6` |
| S3 | `source=leetcode.cn` / `source=leetcode.com` aliases are desired but lower priority than `domain=cn/com` | Requirements |
| S4 | Crawler args must pass through `validate_args` whitelist in `src/models.rs` | `CLAUDE.md` architecture note |

### Open Questions Resolved

| Question | Answer |
|---|---|
| Does leetcode.cn have a historical monthly API? | Yes — `dailyQuestionRecords(year, month)` query exists on `leetcode.cn/graphql/` with different operation name than com's `dailyCodingChallengeV2`. See `design.md:D6`. |
| Is `--domain` CLI arg needed? | Yes — Rust fallback must pass `--domain cn` to spawn a cn-aware crawler instance |
| Should `source=leetcode.cn` be supported? | Yes, as alias mapping to `domain=cn` |

---

## Requirements

### R1 — Add `--domain` CLI argument to `scripts/leetcode.py`

**Scenario:** Rust fallback spawns `uv run python3 leetcode.py --daily --domain cn`

- Add `--domain` arg (choices: `com`, `cn`, default: `com`)
- Pass it to `LeetCodeClient(domain=args.domain)`
- Apply to `--daily`, `--date`, `--monthly` execution paths

**Acceptance:** `python3 leetcode.py --daily --domain cn` fetches from leetcode.cn and writes `domain=cn` row to DB

### R2 — Enable cn fallback in Rust handler

**Scenario:** `GET /api/v1/daily?domain=cn` with no DB data triggers background crawler

- Fallback key: `"cn:{date}"` (mirrors existing `"com:{date}"` pattern)
- Spawn args: `["--daily", "--domain", "cn"]` for today; `["--date", date, "--domain", "cn"]` for historical
- Same TOCTOU guard, cooldown, background task pattern as com

**Acceptance:** First request returns HTTP 202; subsequent request after crawler completes returns 200 with cn data

### R3 — Historical cn date fetch (via monthly batch)

**Scenario:** `GET /api/v1/daily?domain=cn&date=2024-11-15`

- Crawler fetches the entire month via `--monthly 2024 11 --domain cn`, using `dailyQuestionRecords` GraphQL query
- Fallback behavior is symmetric with com: triggers monthly fetch, then retries DB lookup
- `get_daily_challenge()` routes to `fetch_monthly_daily_challenges_cn()` when `domain == "cn"`

**Acceptance:** Historical cn dates trigger monthly batch fetch and return 200 after crawler completes

### R4 — Add `source` query param alias (optional, lower priority)

**Scenario:** `GET /api/v1/daily?source=leetcode.cn`

- Map `source=leetcode.cn` → `domain=cn`, `source=leetcode.com` → `domain=com`
- `domain` takes precedence if both provided
- Invalid `source` values return 400

**Acceptance:** Both `?domain=cn` and `?source=leetcode.cn` return identical responses

### R5 — Validate `--domain` in crawler arg whitelist

**Scenario:** Rust spawns crawler with `--domain cn`

- Add `--domain` to `ArgSpec` whitelist in `src/models.rs` (or wherever `validate_args` is defined)

**Acceptance:** `validate_args` accepts `["--daily", "--domain", "cn"]` without error

---

## Success Criteria

1. `GET /api/v1/daily?domain=cn` (today) → HTTP 202 on first call, HTTP 200 with cn data after crawler completes
2. `GET /api/v1/daily?domain=cn&date=<past>` → HTTP 202 on first call (triggers monthly batch fetch), HTTP 200 after crawler completes
3. `GET /api/v1/daily` (no params) → unchanged behavior (com, today)
4. `GET /api/v1/daily?source=leetcode.cn` → same as `?domain=cn`
5. `python3 leetcode.py --daily --domain cn` → writes row with `domain='cn'` to `daily_challenge` table
6. No regression on existing `domain=com` paths

---

## Implementation Scope

| File | Change |
|---|---|
| `scripts/leetcode.py` | Add `--domain` CLI arg; pass to `LeetCodeClient` constructor; implement cn monthly fetch |
| `src/api/daily.rs` | Add cn fallback path (R2); add `source` param alias (R4) |
| `src/models.rs` | Add `--domain` to crawler arg whitelist (R5) |

Out of scope: bulk historical cn data backfill tooling, `weeklyQuestionRecords` integration.
