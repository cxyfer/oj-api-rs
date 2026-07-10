## Why

Problem detail lookups currently stop at the local database boundary: a missing row returns `404 problem not found` even when the source ID contains enough information to derive the canonical problem URL. Codeforces, AtCoder, and Luogu problem IDs can often be fetched directly, so the API can improve freshness without requiring a full crawler sync first.

## What Changes

- Add direct problem derivation for URL-addressable IDs:
  - `codeforces:1988A` → `https://codeforces.com/contest/1988/problem/A`
  - `codeforces:102951A` and `gym:102951A` → `https://codeforces.com/gym/102951/problem/A` when the contest ID has at least 6 digits
  - `atcoder:abc321_a` → `https://atcoder.jp/contests/abc321/tasks/abc321_a`
  - `luogu:P1083` → `https://www.luogu.com.cn/problem/P1083`
- Add safe single-problem crawler CLI operations for Codeforces, AtCoder, and Luogu so the Rust API can fetch one missing problem without scanning whole contests or problem lists.
- Update `GET /api/v1/problems/{source}/{id}` to keep the existing database-first behavior, then invoke the bounded single-problem crawler on supported misses, re-read the database, and return the fetched detail when successful.
- Preserve existing RFC 7807 `404` behavior for unsupported sources, malformed IDs, crawler failures, and truly missing problems.
- Add focused Rust and Python tests for derivation, argument validation, crawler single-problem mapping, and API fallback behavior.

## Capabilities

### New Capabilities
- `dynamic-problem-fetch`: Direct one-problem fetching from derived URLs when a supported problem detail lookup misses the database.

### Modified Capabilities
- `problem-query`: `GET /api/v1/problems/{source}/{id}` may resolve a supported missing problem dynamically before returning `404`.
- `crawler-cli`: Maintained crawler scripts expose source-scoped single-problem operations validated through the Rust argument whitelist.

## Impact

- Rust API: `src/api/problems.rs`, likely a small helper module for derivation/subprocess orchestration, and API tests.
- Rust models/validation: `src/models.rs` crawler argument specs.
- Python crawlers: `scripts/codeforces.py`, `scripts/atcoder.py`, `scripts/luogu.py`, and focused script tests.
- Runtime behavior: problem detail requests may perform bounded outbound HTTP through crawler subprocesses on supported database misses; all existing database-hit and unsupported-miss responses remain unchanged.
