## Context

`GET /api/v1/problems/{source}/{id}` currently reads from SQLite and returns RFC 7807 `404` on miss. The Python crawlers already know how to fetch and normalize problem content for Codeforces, AtCoder, and Luogu, but their CLIs only expose broad sync/backfill operations. This change adds a narrow database-miss path for sources whose IDs can deterministically produce a canonical problem URL.

The API must remain database-first: stored records should return without subprocess work, and unsupported or failed dynamic fetches must preserve existing not-found semantics.

## Goals / Non-Goals

**Goals:**
- Derive canonical URLs and metadata for supported single problem IDs.
- Fetch exactly one missing Codeforces, AtCoder, or Luogu problem through existing crawler code paths.
- Persist fetched results into the existing `problems` table and return the existing `ProblemDetailResponse` shape.
- Keep crawler invocation bounded by existing crawler timeout configuration and safe argument validation.
- Cover derivation and single-problem behavior with focused tests that do not require live network access.

**Non-Goals:**
- No dynamic fetching for LeetCode, SPOJ, UVa, or other sources.
- No full contest/problemset scan during a problem detail request.
- No new database schema or migration.
- No guarantee that remote pages bypass rate limits or authentication; those failures still return `404`.
- No background job tracking UI for this synchronous miss fallback.

## Decisions

### Use a Rust derivation helper as the routing gate

Create a small Rust helper module or functions that parse `(source, id)` into a direct-fetch plan: canonical source, normalized ID, URL, crawler source, and single-problem argument. This keeps malformed IDs from reaching subprocess execution and gives Rust unit tests full coverage for all examples.

Alternatives considered:
- Let Python parse all IDs. Rejected because Rust needs a clear safe/unsafe gate before spawning external processes.
- Hard-code URL strings inside `get_problem`. Rejected because it would mix request handling with parsing rules and make tests noisy.

### Add a single `--problem <id>` crawler CLI operation per supported crawler

Each crawler gets a source-scoped `--problem` flag. The crawler derives the URL/metadata internally using the same rules as Rust and writes a single row via `ProblemsDatabaseManager.update_problem`. Rust `ArgSpec` validates `--problem` as `Str` for the supported sources.

Alternatives considered:
- Pass `--url` from Rust. Rejected because validating arbitrary URLs in the Rust whitelist is broader than needed and risks SSRF-like expansion.
- Reuse existing `--contest` flags. Rejected because fetching an entire contest on a detail lookup violates the single-problem scope.

### Synchronous miss fallback with timeout and re-read

On DB miss, `get_problem` invokes the crawler subprocess with `uv run python3 <script> --problem <id> --db-path <configured db path>`. After successful process exit, Rust re-reads the database and returns the record if present. Any process failure, timeout, or absent row falls through to the existing `404`.

Alternatives considered:
- Return `202 Accepted` and fetch in background. Rejected for this endpoint because the user asked to fetch and return content directly where possible, and daily challenge already owns the async fallback pattern.
- Fetch HTML directly from Rust. Rejected because crawler scripts already handle sessions, rate limits, Cloudflare workarounds, parsing, and content normalization.

### Minimal API surface change

Only the single detail endpoint changes behavior. `resolve` may continue returning `problem: null` on misses unless implementation reuse makes dynamic fetch trivial without changing its response contract. Batch/list endpoints remain database-only.

Alternatives considered:
- Add dynamic fetch to batch and resolve simultaneously. Rejected as broader scope with more rate-limit and latency risk.

## Risks / Trade-offs

- Remote fetch can increase request latency → Use existing crawler timeout configuration and only run on supported DB misses.
- Crawler subprocess can fail due to rate limiting, network, or page changes → Preserve existing `404` behavior and log failures for diagnosis.
- Duplicate parsing rules in Rust and Python can drift → Keep derivation rules small and cover both sides with tests for the required examples.
- Codeforces gym heuristic may classify six-digit contest IDs as gym → This follows the requested rule; normal contest IDs below six digits keep `/contest/`.
- Synchronous subprocess execution adds resource cost under repeated misses → Scope to single-problem requests and no scan operations; future throttling/caching can be added if production traffic requires it.
