## Why

The daily challenge endpoint currently behaves as a LeetCode-only feature even though the compact `daily_challenge` table can already store arbitrary daily sources. Adding additional daily challenge sources lets the API expose external curated problem sets without changing the storage model.

## What Changes

- Accept additional daily challenge source names in `GET /api/v1/daily`, starting with `sheep` and `0x3f`.
- Keep LeetCode `domain` behavior and LeetCode fallback crawler behavior unchanged.
- Add crawler support for Sheep's GitHub Markdown feed.
- Add 0x3f support only for stable downloaded/exported local tabular input; do not scrape Tencent Docs anonymous UI or private APIs.
- Add parser and API tests for the new sources.

## Capabilities

### New Capabilities
- `daily-challenge-sources`: Additional daily challenge sources exposed through the existing daily challenge storage and API contract.

### Modified Capabilities
- `daily-challenge`: The existing daily endpoint accepts non-LeetCode daily sources while preserving LeetCode domain aliases and fallback behavior.
- `crawler-cli`: The crawler gains explicit daily-source CLI flags for external daily feeds.

## Impact

- Rust API source selection and fallback routing in `src/api/daily.rs`.
- Crawler argument validation in `src/models.rs`.
- Codeforces crawler CLI and parsing in `scripts/codeforces.py`.
- Python daily storage/parser tests and Rust API tests.
- No database migration is expected because `daily_challenge(date, source, problems)` is already source-agnostic.
