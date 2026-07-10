## Why

AtCoder dynamic single-problem fetching currently relies on task-id heuristics that can overfit historical slugs and still cannot resolve contest slugs that are not encoded in the task id. This change makes AtCoder lookup deterministic: infer only the straightforward task-id cases, and require an explicit contest path when the contest cannot be derived safely.

## What Changes

- Support explicit AtCoder single-problem identifiers in both `contest/problem_id` and `contest/tasks/problem_id` forms.
- Keep straightforward AtCoder task-id derivation for IDs where the contest slug is the prefix before the first underscore, such as `abc042_a` and `arc001_1`.
- Keep the known `pastYYYYMM_*` mapping to `pastYYYYMM-open` because it is a stable platform convention.
- Remove automatic `arc*_abcNNN_* -> abcNNN` inference; callers that need historical or non-obvious task slugs must provide the contest explicitly, such as `abc042/arc058_abc042_a`.
- Document that ambiguous task ids such as `ndpc2026_m` require explicit input like `ndpc/ndpc2026_m` or `ndpc/tasks/ndpc2026_m`.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `crawler-cli`: clarify accepted AtCoder `--problem` formats and explicit contest-path behavior.
- `dynamic-problem-fetch`: refine AtCoder URL derivation requirements for inferred versus explicit contest identifiers.

## Impact

- Rust dynamic fetch planning in `src/dynamic_problem.rs`.
- AtCoder crawler parsing and URL derivation in `scripts/atcoder.py`.
- Rust and Python derivation tests.
- OpenSpec requirements for crawler CLI and dynamic problem fetching.
