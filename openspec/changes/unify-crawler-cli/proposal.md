## Why

目前各平台爬蟲使用不同 CLI 動詞表達相同工作，例如 LeetCode 使用 `--init`、Luogu 使用 `--sync`、AtCoder/Codeforces 使用 `--fetch-all --resume`，導致後台選項、Rust 參數白名單與腳本文件需要分散維護。統一爬蟲 CLI 可以讓「同步題庫 metadata」、「從比賽抓題目內容」、「補齊缺失內容」成為跨平台穩定介面，同時降低後續新增來源或調整後台操作的成本。

## What Changes

- Introduce `--sync-problemset` as the canonical operation for fetching initial problem metadata while skipping existing problems.
- Introduce `--fetch-contest` as the canonical operation for fetching contest/archive problems and their content for AtCoder and Codeforces.
- Make contest fetching resume by default for AtCoder and Codeforces, using existing progress JSON to skip already-fetched contests.
- Add `--no-resume` for AtCoder and Codeforces contest fetching when the operator wants to ignore saved progress and rescan contests.
- Keep `--fill-missing-content` as the canonical operation for filling content on existing metadata-only problems across supported sources.
- Preserve legacy flags as compatibility aliases where practical, including `--init`, `--sync`, `--sync-spoj`, `--sync-kenkoooo`, `--sync-history`, `--fetch-all`, and `--resume`.
- Update admin crawler options and Rust argument validation to expose and accept the unified operation flags.
- Keep auxiliary flags for non-unified workflows, including daily challenge fetching, single-contest fetch, Luogu training lists, diagnostics, rate limits, batch size, overwrite behavior, and debug/status operations.

## Capabilities

### New Capabilities

- `crawler-cli`: Defines the canonical cross-platform crawler CLI operations, supported sources, resume semantics, legacy aliases, and auxiliary flags that remain maintained.

### Modified Capabilities

- `admin-management`: Admin crawler trigger validation and UI options SHALL expose the unified crawler operations while preserving supported auxiliary and compatibility flags.

## Impact

- Python crawler entry points:
  - `scripts/leetcode.py`
  - `scripts/atcoder.py`
  - `scripts/codeforces.py`
  - `scripts/luogu.py`
- Admin crawler argument validation and source-to-script routing in `src/models.rs`.
- Admin crawler UI flag configuration in `static/admin.js`.
- Crawler flag translations in `static/i18n/en.json`, `static/i18n/zh-TW.json`, and `static/i18n/zh-CN.json`.
- README crawler script documentation and any existing CLI examples that mention legacy operation flags.
- No new runtime dependency is expected.
