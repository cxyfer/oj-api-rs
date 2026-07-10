## Why

The current `0x3f` daily source path assumes a manually downloaded CSV/TSV file, but the source is a fixed Tencent Docs online sheet that can be read through Tencent Docs APIs. Supporting the online sheet directly enables scheduled refreshes and request-triggered fallback without relying on manual exports.

## What Changes

- Read the `0x3f` source from the fixed Tencent Docs sheet `DWGFoRGVZRmxNaXFz`, tab `BB08J2` (`🎈算法趣题`) through the dedicated daily-source crawler.
- Keep the fixed Tencent Docs file ID and sheet ID in crawler code for this source; resolve the token from ignored local config first, then the configured environment-variable fallback.
- Use Python `curl-cffi` to call Tencent Docs MCP JSON-RPC `sheet.get_sheet_info` and `sheet.get_cell_data` with an Authorization token resolved from local config or the fallback environment variable, then parse mixed LeetCode, AtCoder, Codeforces/Gym, and Luogu URLs.
- Keep `--daily-file` as an optional offline/debug fallback, but no longer require it for `--daily-source 0x3f` when Tencent Docs config is available.
- Add scheduled refresh for configured additional daily sources at UTC+8 08:00, 10:00, and 12:00.
- Change missing additional daily source API behavior so configured sources can trigger the dedicated daily-source fallback crawler instead of only returning `ingestion_required`.

## Capabilities

### New Capabilities

### Modified Capabilities
- `daily-challenge-sources`: Replace the local-only `0x3f` ingestion requirement with fixed Tencent Docs online sheet ingestion and scheduled source refresh semantics.
- `crawler-cli`: Allow `daily_source.py --daily-source 0x3f --date <YYYY-MM-DD>` to read configured Tencent Docs data without `--daily-file`, while preserving local file fallback.
- `daily-challenge`: Trigger configured additional daily source fallback jobs when API reads miss usable stored data.
- `crawler-config`: Add daily-source configuration for a direct local Tencent Docs token and its environment-variable fallback.
- `config-example`: Document the empty Tencent Docs token placeholder, environment fallback, and fixed scheduler behavior without storing secrets.

## Impact

- Affects the dedicated Python daily-source crawler in `scripts/daily_source.py` and shared config reading in `scripts/utils/config.py`.
- Affects Rust daily challenge fallback orchestration in `src/api/daily.rs` and configuration structs/defaults.
- Affects crawler argument validation in `src/models.rs` if new config-driven flags are exposed through admin workflows.
- Adds tests for Tencent Docs MCP response parsing, configured `0x3f` online ingestion, scheduler timing, and API fallback behavior.
- Requires a Tencent Docs Authorization token from ignored local config or its environment fallback; no token value is stored in tracked repository artifacts.
