## 1. Daily-source enrichment coordination

- [x] 1.1 Select candidates from pre-write database state when the row is absent or both stored title and content are blank after trimming whitespace
- [x] 1.2 Store problem snapshots and ordered daily references atomically before starting enrichment
- [x] 1.3 Attempt candidates sequentially in parsed order and isolate false results or exceptions without rolling back stored daily data

## 2. Source-specific detail dispatch

- [x] 2.1 Dispatch Codeforces, AtCoder, and Luogu candidates to their existing single-problem crawlers while preserving AtCoder contest context
- [x] 2.2 Dispatch LeetCode candidates through the existing detail path with domain selection from the parsed problem URL
- [x] 2.3 Preserve Codeforces Gym storage keys while fetching normalized Gym problem paths
- [x] 2.4 Treat whitespace-only LeetCode text as missing and replace it only with non-blank fetched detail

## 3. Verification

- [x] 3.1 Add focused tests for candidate selection, post-commit ordering, source dispatch, failure isolation, Gym identity, and LeetCode whitespace replacement
- [x] 3.2 Run the focused Python test suite and full Rust test suite
- [x] 3.3 Run formatting, lint, OpenSpec strict validation, and diff whitespace checks
