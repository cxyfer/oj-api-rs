## 1. Rust direct-fetch planning

- [x] 1.1 Add a Rust helper that parses supported `(source, id)` pairs into a single-problem fetch plan with canonical URL and crawler arguments
- [x] 1.2 Cover Codeforces contest, Codeforces gym, explicit gym, AtCoder, Luogu, unsupported source, and malformed ID derivation with Rust unit tests
- [x] 1.3 Add `--problem` to the Rust crawler argument whitelist only for Codeforces, AtCoder, and Luogu

## 2. Python crawler single-problem operations

- [x] 2.1 Add single-problem ID parsing/URL derivation helpers to `scripts/codeforces.py`, including `/contest/` vs `/gym/` selection
- [x] 2.2 Add a `--problem` operation to `scripts/codeforces.py` that fetches one problem and persists it
- [x] 2.3 Add single-problem ID parsing/URL derivation helpers and `--problem` operation to `scripts/atcoder.py`
- [x] 2.4 Add single-problem ID parsing/URL derivation helpers and `--problem` operation to `scripts/luogu.py`
- [x] 2.5 Add focused Python tests for derivation helpers without live network calls

## 3. API miss fallback integration

- [x] 3.1 Add a bounded Rust subprocess helper that runs `uv run python3 <script> --problem <id> --db-path <configured-db>` for supported plans
- [x] 3.2 Update `GET /api/v1/problems/{source}/{id}` to read DB first, run direct fetch only on supported miss, re-read DB, and preserve 404 on failure
- [x] 3.3 Add focused Rust tests for database-hit no-fetch and supported-miss fallback behavior using a test seam or deterministic subprocess stub

## 4. Verification

- [x] 4.1 Run Python focused tests for single-problem derivation
- [x] 4.2 Run `cargo fmt`
- [x] 4.3 Run focused Rust tests for dynamic problem fetch behavior
- [x] 4.4 Run full `cargo test`
- [x] 4.5 Update Ralph `.omc/prd.json` story pass flags and `.omc/progress.txt` with verified evidence
