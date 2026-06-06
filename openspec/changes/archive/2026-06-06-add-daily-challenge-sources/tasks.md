## 1. API source selection

- [x] 1.1 Generalize `src/api/daily.rs` source selection to accept `leetcode.com`, `leetcode.cn`, `sheep`, and `0x3f`.
- [x] 1.2 Keep `domain` LeetCode-only and reject conflicts between `domain` and non-matching `source` values.
- [x] 1.3 Preserve LeetCode crawler fallback behavior and return HTTP 202 without spawning a crawler for missing additional daily source rows.
- [x] 1.4 Add Rust API tests for additional daily source response projection and validation conflicts.

## 2. Codeforces crawler daily source support

- [x] 2.1 Add Codeforces CLI flags `--daily-source`, `--date`, and `--daily-file` to `scripts/codeforces.py`.
- [x] 2.2 Implement Sheep raw Markdown URL construction, fetch, table parsing, Codeforces URL extraction, and missing-file handling.
- [x] 2.3 Implement 0x3f local CSV/TSV-style file parsing with header aliases, date filtering, Codeforces URL extraction, and clear failure on missing `--daily-file`.
- [x] 2.4 Upsert minimal `source=codeforces` problem snapshots before writing compact daily refs.
- [x] 2.5 Store daily rows through `DailyChallengeDatabaseManager.update_daily` using `sheep` and `0x3f`.

## 3. Argument whitelist and tests

- [x] 3.1 Extend `src/models.rs` Codeforces argument whitelist for the new daily flags and validate daily-source values.
- [x] 3.2 Add Python parser/storage tests for Sheep regular and Gym links.
- [x] 3.3 Add Python parser/storage tests for 0x3f local export parsing and invalid input.

## 4. Verification

- [x] 4.1 Run `uv ruff format` and `uv ruff check` for changed Python files.
- [x] 4.2 Run targeted Python tests for daily storage/parser behavior.
- [x] 4.3 Run `cargo fmt` and `cargo test`.
- [x] 4.4 Verify all `.omc/prd.json` acceptance criteria and request reviewer verification.
