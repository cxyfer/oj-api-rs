## Context

`0x3f` currently imports from a local CSV/TSV file, but the source is the fixed Tencent Docs sheet `DWGFoRGVZRmxNaXFz` / `BB08J2` (`🎈算法趣题`). Browser inspection showed public frontend endpoints can return sheet payloads, but cell data is encoded in Tencent Docs internal workbook formats. The Tencent Docs skill exposes MCP JSON-RPC tools that return sheet data directly as CSV through `sheet.get_cell_data`, which is simpler and more stable for the crawler.

The scripts environment already depends on `curl-cffi`, so no new Python HTTP dependency is needed. The Tencent Docs Authorization token may be configured directly in ignored local `config.toml`; the existing environment variable remains a deployment fallback.

## Goals / Non-Goals

**Goals:**
- Fetch `0x3f` daily rows from the fixed Tencent Docs sheet without requiring a local file.
- Keep Tencent Docs file ID and sheet ID fixed in code for this source.
- Prefer a direct Tencent Docs token in ignored local `config.toml`, then fall back to the configured environment variable; never store token values in tracked repository files or OpenSpec artifacts.
- Preserve `--daily-file` as an offline/debug fallback.
- Refresh additional daily sources at UTC+8 08:00, 10:00, and 12:00.
- Let API misses trigger configured additional daily source crawler jobs using the same no-duplicate and wait semantics as existing daily fallback.

**Non-Goals:**
- No browser automation in the production crawler.
- No reverse engineering of Tencent Docs frontend workbook compression in the first implementation.
- No configurable `0x3f` file ID or sheet ID in this change.
- No storage of Tencent Docs tokens in tracked examples, OpenSpec artifacts, or tests.

## Decisions

### Use Tencent Docs MCP JSON-RPC over frontend sheet endpoints

Use `POST https://docs.qq.com/openapi/mcp` with `Authorization` and JSON-RPC `tools/call` requests:

- `sheet.get_sheet_info` verifies sheet metadata and row count.
- `sheet.get_cell_data` fetches rows from `file_id = DWGFoRGVZRmxNaXFz`, `sheet_id = BB08J2`, with `return_csv = true`.

Rationale: the MCP tool returns normal CSV data that can reuse the dedicated tabular parser for mixed LeetCode, AtCoder, Codeforces/Gym, and Luogu problem URLs. The public frontend endpoints are unauthenticated but return internal encoded workbook payloads, which is brittle and harder to test.

Alternative considered: call `/dop-api/opendoc` and `/dop-api/get/sheet` directly. Rejected for first implementation because the returned cell data is not plain CSV/JSON cell values and would require format-specific decoding.

### Configure a direct local token with environment fallback

Add Python/Rust-readable config fields for the Tencent Docs token and its environment-variable fallback, for example:

```toml
[daily_sources.tencent_docs]
token = ""
token_env = "TENCENT_DOCS_TOKEN"
```

The crawler first uses a non-empty trimmed `token` value from the ignored local `config.toml`. If it is empty, it reads `os.environ[token_env]`. A direct token does not require a fallback name; when the direct token is empty, `token_env` must be non-empty. `config.toml.example` always leaves `token` empty, and no token values belong in repository files, OpenSpec artifacts, tests, or Debug output. If `--daily-source 0x3f` is used without `--daily-file` and neither source resolves to a non-empty token, the CLI exits with a clear configuration error.

Rationale: local development can use the project’s existing ignored config file while existing environment-based deployments remain compatible.

### Keep fixed 0x3f sheet identifiers in code

Define constants in `scripts/daily_source.py`:

- `TENCENT_DOCS_0X3F_FILE_ID = "DWGFoRGVZRmxNaXFz"`
- `TENCENT_DOCS_0X3F_SHEET_ID = "BB08J2"`

Rationale: the source is intentionally fixed, so config should not introduce unnecessary moving parts.

### Preserve local file fallback

`--daily-file` remains supported and takes precedence over online fetching for `--daily-source 0x3f`. This allows deterministic tests and manual recovery when Tencent Docs is unavailable.

### Share fallback spawning for API and scheduler

Extract daily-source crawler spawning so both API misses and scheduled refresh can launch `daily_source.py` with:

```text
daily_source.py --daily-source sheep --date <utc+8-date>
daily_source.py --daily-source 0x3f --date <utc+8-date>
```

Use the existing daily fallback key pattern by source/date to avoid duplicate jobs. Add a distinct crawler trigger variant if needed for scheduled jobs so admin status can distinguish request-triggered and schedule-triggered runs.

### Fixed UTC+8 scheduler window

Start a background task during server startup that computes the next UTC+8 run at 08:00, 10:00, or 12:00. Each run attempts configured additional sources:

- `sheep`: always eligible.
- `0x3f`: eligible only when the direct local Tencent Docs token or its configured environment-variable fallback resolves to a non-empty value.

Rationale: fixed times match the product requirement and avoid unnecessary config complexity.

## Risks / Trade-offs

- Tencent Docs MCP API may change → isolate MCP request construction and response parsing in small Python helpers with tests using fixture JSON.
- Token missing or expired → fail `0x3f` ingestion with a clear error; scheduler skips or records failed jobs without writing empty daily rows.
- Large sheet response → request only the fixed sheet and bounded columns; fetch enough rows based on sheet info or a conservative fixed range.
- Scheduled jobs overlap API fallback → reuse source/date runtime keys and running-job checks.
- External service latency → API fallback keeps existing wait limit and returns HTTP 202 when the crawler does not finish in time.
