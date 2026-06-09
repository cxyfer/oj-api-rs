## Context

The dynamic problem fetch path derives source-specific crawler arguments when an API detail lookup misses the local database. AtCoder task IDs are usually derivable from the prefix before the first underscore, but some task IDs require an explicit contest slug that is not safely recoverable from the task ID alone.

The current implementation includes heuristic handling for historical `arc*_abcNNN_*` slugs. That behavior is surprising for user-facing input and risks overfitting ambiguous task IDs. The intended behavior is to keep deterministic inference for straightforward IDs and require explicit contest input for non-obvious task slugs.

## Goals / Non-Goals

**Goals:**

- Parse AtCoder single-problem IDs consistently in Rust dynamic fetch planning and the Python AtCoder crawler.
- Support explicit AtCoder contest paths in `contest/problem_id` and `contest/tasks/problem_id` forms.
- Keep simple inference for IDs like `abc042_a`, `arc001_1`, and `dp_a`.
- Keep stable `pastYYYYMM_* -> pastYYYYMM-open` handling.
- Add regression tests for explicit paths and ambiguous IDs.

**Non-Goals:**

- Build a complete AtCoder contest metadata resolver.
- Query Kenkoooo or AtCoder archives during URL derivation.
- Add broad search or fallback scanning for ambiguous task IDs.
- Change Codeforces, Luogu, or LeetCode dynamic fetch behavior.

## Decisions

1. Explicit paths take precedence.
   - Accept `contest/problem_id` and `contest/tasks/problem_id` for AtCoder only.
   - Validate both segments with the same conservative ASCII rules used for existing IDs.
   - Rationale: this lets callers provide `ndpc/ndpc2026_m` or `ndpc/tasks/ndpc2026_m` when the contest cannot be inferred.

2. Simple inference remains prefix-based.
   - For `problem_id` without slashes, use the prefix before `_` as the contest slug.
   - Keep rejecting values without `_`.
   - Rationale: this matches common AtCoder IDs and keeps behavior predictable.

3. Keep only stable platform convention special-casing.
   - Continue mapping `pastYYYYMM_*` to `pastYYYYMM-open`.
   - Remove automatic `arc*_abcNNN_* -> abcNNN` inference.
   - Rationale: `arc058_abc042_a` is a historical task slug, but users can specify `abc042/arc058_abc042_a` explicitly. Avoid guessing for all future ambiguous IDs.

4. Keep Python and Rust derivation rules aligned.
   - Rust `derive_direct_fetch_plan` and Python `AtCoderClient.problem_url_for_id` must produce equivalent URLs for all accepted forms.
   - Rationale: Rust uses derivation for dynamic fetch planning, Python uses it for actual crawler fetches.

## Risks / Trade-offs

- Ambiguous IDs require caller knowledge → Mitigation: document explicit forms and add examples in specs/tests.
- Removing `arc*_abcNNN_*` inference may change behavior from the current PR branch → Mitigation: explicit `abc042/arc058_abc042_a` remains supported and avoids hidden guesswork.
- Slash-bearing problem args pass through crawler arg validation → Mitigation: update tests to cover accepted explicit values and ensure validation rejects malformed path traversal patterns.
