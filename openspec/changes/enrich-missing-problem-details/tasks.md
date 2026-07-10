## 1. Daily-source enrichment coordination

- [x] 1.1 Select candidates from pre-write database state when the row is absent, both stored title and content are blank, or both fields still match the current curated snapshot
- [x] 1.2 Store problem snapshots and ordered daily references atomically before starting enrichment
- [x] 1.3 Attempt candidates sequentially in parsed order and isolate false results or exceptions without rolling back stored daily data

## 2. Source-specific detail dispatch

- [x] 2.1 Dispatch Codeforces, AtCoder, and Luogu candidates to their existing single-problem crawlers while preserving AtCoder contest context
- [x] 2.2 Dispatch LeetCode candidates through the existing detail path with domain selection from the parsed problem URL
- [x] 2.3 Preserve Codeforces Gym storage keys while fetching normalized Gym problem paths
- [x] 2.4 Treat whitespace-only LeetCode text as missing and replace it only with non-blank fetched detail
- [x] 2.5 Prefer non-empty source-fetched title and content without clearing curated metadata omitted by the detail response
- [x] 2.6 Parse public Codeforces Gym statements even when page navigation contains a normal sign-in link
- [x] 2.7 Parse Codeforces statement titles, remove redundant index prefixes, replace ID placeholders, and retry existing Gym placeholder rows
- [x] 2.8 Populate Codeforces tags from contest metadata and retry existing rows whose tags remain empty

## 3. Verification

- [x] 3.1 Add focused tests for candidate selection, post-commit ordering, source dispatch, failure isolation, source-detail precedence, Codeforces tags, Gym title/content parsing and identity, and LeetCode whitespace replacement
- [x] 3.2 Run the focused Python test suite and full Rust test suite
- [x] 3.3 Run formatting, lint, OpenSpec strict validation, and diff whitespace checks

## 4. Review corrections

- [x] 4.1 Resolve LeetCode slugs to a local numeric ID before candidate selection and storage, retain the slug fallback, and enrich by the exact stored ID
- [x] 4.2 Prefer non-empty Codeforces API tags while preserving stored tags as the fallback when metadata is missing or empty
- [x] 4.3 Add regression coverage for LeetCode numeric/fallback identities and Codeforces tag precedence/preservation, then rerun focused and repository verification
