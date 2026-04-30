## 1. Python Crawler CLI

- [x] 1.1 Add `--sync-problemset` to `scripts/leetcode.py` and map it to the existing metadata initialization behavior while keeping `--init`.
- [x] 1.2 Add `--sync-problemset`, `--fetch-contest`, and `--no-resume` to `scripts/atcoder.py`, mapping legacy `--sync-kenkoooo`, `--sync-history`, `--fetch-all`, and `--resume` to the canonical behavior.
- [x] 1.3 Update `scripts/atcoder.py` so contest fetching resumes by default and `--no-resume` disables progress-based skipping.
- [x] 1.4 Add `--fetch-contest` and `--no-resume` to `scripts/codeforces.py`, keeping `--sync-problemset`, `--fetch-all`, and `--resume` compatible.
- [x] 1.5 Update `scripts/codeforces.py` so contest fetching resumes by default and `--no-resume` disables progress-based skipping.
- [x] 1.6 Add `--sync-problemset` to `scripts/luogu.py` for Luogu metadata sync while keeping `--sync`.
- [x] 1.7 Add SPOJ handling for `--sync-problemset` in `scripts/luogu.py` while keeping `--sync-spoj` and existing `--source spoj` content backfill behavior.
- [x] 1.8 Ensure unsupported canonical operations, especially `--fetch-contest` on LeetCode, Luogu, and SPOJ, are rejected by argparse rather than silently ignored.

## 2. Admin Trigger Validation

- [x] 2.1 Update `src/models.rs` LeetCode argument specs to accept `--sync-problemset` and keep existing daily/content auxiliary flags.
- [x] 2.2 Update `src/models.rs` AtCoder argument specs to accept `--sync-problemset`, `--fetch-contest`, and `--no-resume` while preserving legacy aliases and auxiliary flags.
- [x] 2.3 Update `src/models.rs` Codeforces argument specs to accept `--fetch-contest` and `--no-resume` while preserving `--sync-problemset`, legacy aliases, and auxiliary flags.
- [x] 2.4 Update `src/models.rs` Luogu and SPOJ argument specs to accept `--sync-problemset` while preserving source-specific auxiliary and hidden routing flags.
- [x] 2.5 Add or update Rust tests for `validate_args` covering accepted canonical flags, rejected unsupported flags, `--no-resume`, and legacy aliases.

## 3. Admin UI And I18n

- [x] 3.1 Update `static/admin.js` crawler flag configuration to show canonical operation flags for LeetCode, AtCoder, Codeforces, Luogu, and SPOJ.
- [x] 3.2 Keep maintained auxiliary controls visible in `static/admin.js`, including LeetCode daily flags, single-contest fetch, rate limit, batch size, overwrite, training list, status/debug, and diagnostics.
- [x] 3.3 Hide or de-emphasize legacy operation flags in the admin UI while keeping them accepted by backend validation.
- [x] 3.4 Add i18n labels for `--sync-problemset`, `--fetch-contest`, and `--no-resume` in `static/i18n/en.json`, `static/i18n/zh-TW.json`, and `static/i18n/zh-CN.json`.
- [x] 3.5 Verify the crawler page still renders controls for all supported sources after the flag configuration change.

## 4. Documentation

- [x] 4.1 Update README crawler script documentation to present canonical operations first.
- [x] 4.2 Document legacy operation flags as compatibility aliases rather than the preferred interface.
- [x] 4.3 Document AtCoder and Codeforces resume-by-default behavior and the `--no-resume` override.
- [x] 4.4 Update any crawler command examples that still teach legacy operation names as primary commands.

## 5. Verification

- [x] 5.1 Run Python CLI help checks for `leetcode.py`, `atcoder.py`, `codeforces.py`, and `luogu.py` to confirm canonical flags are exposed on supported scripts.
- [x] 5.2 Run focused Python parse/behavior checks for AtCoder and Codeforces to verify `--fetch-contest` defaults to resume and `--no-resume` disables it without network access.
- [x] 5.3 Run Rust tests covering admin crawler argument validation.
- [x] 5.4 Run frontend or static checks available in the project to catch JavaScript/i18n syntax issues.
- [x] 5.5 Run `openspec validate unify-crawler-cli --strict` and confirm the change remains valid after implementation.
