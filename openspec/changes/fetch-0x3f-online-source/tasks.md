## 1. Configuration

- [x] 1.1 Add Rust config structs for `[daily_sources.tencent_docs]` with direct `token` support and `token_env` defaulting to `TENCENT_DOCS_TOKEN`
- [x] 1.2 Add Python `ConfigManager` accessors that resolve a direct Tencent Docs token before the configured environment fallback
- [x] 1.3 Update `config.toml.example` to document `[daily_sources.tencent_docs] token = ""`, `token_env = "TENCENT_DOCS_TOKEN"`, and the fixed UTC+8 refresh schedule
- [x] 1.4 Add config tests covering direct-token precedence, whitespace normalization, environment fallback, empty token env rejection, and no token value stored in examples

## 2. Tencent Docs 0x3f ingestion

- [x] 2.1 Keep `scripts/daily_source.py` as the dedicated additional daily-source crawler and add fixed `0x3f` Tencent Docs file/sheet constants there
- [x] 2.2 Implement a small Tencent Docs MCP JSON-RPC client using `curl-cffi` for `sheet.get_sheet_info` and `sheet.get_cell_data`
- [x] 2.3 Parse Tencent Docs `sheet.get_cell_data` CSV output through the existing 0x3f tabular parsing path
- [x] 2.4 Preserve `--daily-file` as a higher-priority offline fallback for `--daily-source 0x3f`
- [x] 2.5 Make missing or empty token env fail with a clear CLI error without writing an empty daily row

## 3. Daily fallback and scheduler

- [x] 3.1 Extract reusable daily-source crawler spawning from the existing LeetCode-only fallback path
- [x] 3.2 Change API misses for `source=sheep` to spawn `daily_source.py --daily-source sheep --date <date>` with existing wait/dedup behavior
- [x] 3.3 Change API misses for `source=0x3f` to spawn `daily_source.py --daily-source 0x3f --date <date>` only when the direct token or environment fallback resolves
- [x] 3.4 Keep `ingestion_required` for `source=0x3f` when both direct token and environment fallback are missing or empty
- [x] 3.5 Add a server startup background scheduler that runs additional daily source refreshes at UTC+8 08:00, 10:00, and 12:00
- [x] 3.6 Reuse source/date runtime keys so scheduled jobs and API fallback do not spawn duplicates

## 4. Tests and validation

- [x] 4.1 Add Python unit tests for Tencent Docs MCP response parsing and CSV extraction using fixture responses
- [x] 4.2 Add Python tests for online 0x3f ingestion, missing token env, and `--daily-file` fallback precedence
- [x] 4.3 Add Rust API tests for Sheep fallback spawn, 0x3f fallback spawn with token env, 0x3f ingestion-required without token env, and duplicate-job reuse
- [x] 4.4 Add Rust scheduler tests for UTC+8 08:00/10:00/12:00 next-run calculation and source eligibility
- [x] 4.5 Run `uv ruff format`, `uv ruff check`, `cargo fmt`, and relevant Rust/Python tests
