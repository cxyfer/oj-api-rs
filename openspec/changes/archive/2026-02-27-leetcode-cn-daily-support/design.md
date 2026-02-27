## Context

The `/api/v1/daily` endpoint accepts `?domain=com|cn` and queries the DB by `(domain, date)` composite key. However, when DB misses for `domain=cn`, the handler returns 404 immediately — the fallback crawler path is hardcoded for `domain=com` only (`src/api/daily.rs:65-73`).

The Python crawler (`scripts/leetcode.py`) has partial cn support: `fetch_daily_challenge(domain="cn")` works for today via the `todayRecord` GraphQL query, but the CLI always instantiates `LeetCodeClient(domain="com")` with no `--domain` argument.

Key discovery during analysis: leetcode.cn **does** have a monthly API — `dailyQuestionRecords(year, month)` and `weeklyQuestionRecords(year, month)` — with different operation names than com's `dailyCodingChallengeV2`. This means cn and com can have fully symmetric fallback behavior.

Affected modules: `src/api/daily.rs`, `src/models.rs`, `scripts/leetcode.py`.

## Goals / Non-Goals

**Goals:**
- Enable cn fallback in the Rust handler, symmetric with com (today + historical via monthly)
- Add `--domain` CLI argument to the Python crawler
- Implement cn monthly fetch using `dailyQuestionRecords` GraphQL query
- Add `source=leetcode.cn` query param alias
- Introduce `LeetCodeDomain` enum to replace string comparisons
- Fix timezone handling: cn uses UTC+8, com uses UTC
- Fix Python `get_daily_challenge()` bug where `self.domain` is used instead of the `domain` parameter

**Non-Goals:**
- Bulk historical cn data backfill tooling
- Authentication/cookie-based cn API access
- `weeklyQuestionRecords` integration (weekly challenges are a separate feature)
- Multi-instance distributed locking for fallback deduplication

## Decisions

### D1 — Parameterize existing fallback (Option A)

Modify the existing fallback flow in `daily.rs` to accept `domain` as a variable, rather than creating a separate cn-specific function.

**Rationale:** The only differences between cn and com fallback are: (1) timezone for "today" calculation, (2) fallback key prefix, (3) `--domain` CLI arg. The TOCTOU guard, cooldown, background task, and timeout logic are identical. Duplicating the entire flow would create drift risk.

**Alternative rejected:** Separate `fallback_cn_daily()` function — too much copy-paste for minimal behavioral difference.

### D2 — Domain-aware fallback key: `{domain}:{date}`

Change from hardcoded `com:{date}` to `format!("{domain}:{date}")`.

**Rationale:** Prevents cooldown collision between cn and com requests for the same date. The key is generated after `source` → `domain` normalization, so aliases cannot produce different keys.

### D3 — `LeetCodeDomain` enum

Introduce a Rust enum `LeetCodeDomain { Com, Cn }` with `Display`, `FromStr`, `Deserialize` impls. Used in `DailyQuery`, fallback key generation, timezone resolution, and CLI arg construction.

**Rationale:** Eliminates scattered `if domain != "com" && domain != "cn"` string comparisons. Type-safe, exhaustive match ensures new variants (if ever added) are handled everywhere.

**Location:** `src/models.rs` alongside existing model types.

### D4 — Domain-aware timezone for "today" and upper bound

- `Com` → `chrono::Utc` (offset 0)
- `Cn` → `chrono::FixedOffset::east_opt(8 * 3600)` (UTC+8)

Applied to: default date when `?date` is omitted, upper bound validation (`date <= today`), and `--daily` vs `--date` determination in fallback arg construction.

**Rationale:** leetcode.cn resets daily challenge at midnight Beijing time (UTC+8). Without this, requests between 00:00-08:00 UTC would use the wrong date or reject valid cn "today" requests as future dates.

**No new dependency:** `chrono::FixedOffset` is already available in the existing `chrono` dependency.

### D5 — `source` query param alias with conflict detection

Add `source: Option<String>` field to `DailyQuery`. Normalize before validation:

| `domain` | `source` | Result |
|---|---|---|
| None | None | `Com` |
| Some(d) | None | validate(d) |
| None | Some(s) | map + validate(s) |
| Some(d) | Some(s) | if equivalent → validate; if conflict → 400 |

Mapping: `leetcode.com` → `Com`, `leetcode.cn` → `Cn`. Invalid source values → 400.

**Rationale:** `domain` takes precedence. Explicit conflict detection (400) prevents silent bugs in API consumers. The normalization happens once at the top of the handler, before any other logic.

### D6 — CN monthly fetch via `dailyQuestionRecords`

Add `fetch_monthly_daily_challenges_cn(year, month)` method to `LeetCodeClient` using:

```graphql
query dailyQuestionRecords($year: Int!, $month: Int!) {
  dailyQuestionRecords(year: $year, month: $month) {
    date
    userStatus
    question {
      questionFrontendId
      title
      titleSlug
      translatedTitle
    }
  }
}
```

Endpoint: `https://leetcode.cn/graphql/`, with `operation-name: dailyQuestionRecords` header.

**Rationale:** This is the actual API leetcode.cn uses on its problemset page. The response structure differs from com's `dailyCodingChallengeV2` (flat list vs nested `challenges`/`weeklyChallenges`), so a separate method is cleaner than over-parameterizing the existing one.

The existing `get_daily_challenge()` flow (DB check → file check → today fetch → monthly fallback) will route to the cn method when `domain == "cn"`, removing the current `if domain == "com"` guard at line 761.

### D7 — `--domain` in ArgSpec whitelist

Add to `LEETCODE_ARGS` in `src/models.rs`:

```rust
ArgSpec {
    flag: "--domain",
    arity: 1,
    value_type: ValueType::Str, // validated as "com"|"cn" at CLI level
    ui_exposed: true,
}
```

**Rationale:** Even though `daily.rs` fallback spawns the crawler directly (bypassing `validate_args`), the admin UI crawler trigger does go through `validate_args`. Consistency requires the whitelist entry.

### D8 — CLI `None` result → non-zero exit

When `--daily` or `--date` returns `None`, the CLI must `sys.exit(2)` instead of printing `null` and exiting 0.

**Rationale:** Rust fallback checks `output.status.success()` to determine `Completed` vs `Failed`. An exit-0 with no DB write causes the fallback to think the job succeeded, preventing retry.

## Risks / Trade-offs

**[R1] CN GraphQL API stability** → The `dailyQuestionRecords` query is undocumented and could change without notice. Mitigation: schema guard in Python (check response shape before accessing nested fields); non-zero exit on unexpected structure so Rust marks as Failed with cooldown.

**[R2] Timezone edge case at midnight** → Between 00:00-00:01 UTC+8, the cn "today" just flipped but leetcode.cn might not have published the new challenge yet. Mitigation: this is the same race condition that exists for com at UTC midnight — the existing cooldown + retry pattern handles it.

**[R3] `source` alias adds API surface** → More ways to specify the same thing increases testing surface. Mitigation: normalization happens once at handler entry; all downstream code only sees `LeetCodeDomain` enum.

**[R4] Python `self.domain` / `domain` parameter confusion** → `get_daily_challenge()` uses `self.domain` for timezone at line 698-702 and `self.domain` for fetch at line 757, ignoring the `domain` parameter. Mitigation: fix both to use the local `domain` variable. This is a pre-existing bug that affects correctness regardless of this change.

**[R5] CN `acRate` fraction vs percentage** → Already handled in existing code (`scripts/leetcode.py:661-663`), but the monthly fetch path will need the same `×100` normalization. Mitigation: apply the same transform in the new `fetch_monthly_daily_challenges_cn` method.

## Open Questions

None — all ambiguities resolved during multi-model analysis and user consultation.
