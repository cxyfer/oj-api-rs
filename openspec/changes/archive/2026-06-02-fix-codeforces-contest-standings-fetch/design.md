## Context

The Codeforces crawler discovers contest problems through `scripts/codeforces.py` by calling `contest.standings` inside `fetch_contest_problems`. The current URL appends `from=1&count=1`; affected contests now reject that shape, while `contestId` alone remains accepted and still returns the `result.problems` list needed by the crawler.

## Goals / Non-Goals

**Goals:**

- Use the accepted Codeforces standings URL shape for contest problem discovery.
- Preserve existing parsing, content fetching, database updates, and progress behavior.
- Keep the change limited to Codeforces crawler logic.

**Non-Goals:**

- Add a new Codeforces API client abstraction.
- Change crawler CLI flags, database schema, or Rust API behavior.
- Introduce retries or rate-limit behavior beyond the existing fetch helpers.

## Decisions

- Build standings URLs as `contest.standings?contestId=<id>` only.
  - Rationale: the crawler only needs contest metadata and problem definitions, and the accepted request shape returns those fields without pagination parameters.
  - Alternative considered: keep `from/count` but increase `count`; rejected because the reported failure is caused by using additional parameters at all.
- Keep `_fetch_json` and existing status checks unchanged.
  - Rationale: this is a request-shape fix, not an error-handling redesign.

## Risks / Trade-offs

- Codeforces could return larger standings payloads without `count` → Mitigation: contest problem lists are still parsed from the existing response structure, and no ranking rows are persisted.
- Some contests may still fail due to Codeforces availability or anti-bot controls → Mitigation: existing retry, throttling, and logging behavior remains in place.
