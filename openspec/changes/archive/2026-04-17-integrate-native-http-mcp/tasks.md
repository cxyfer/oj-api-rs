## 1. Runtime and configuration foundation

- [x] 1.1 Add or confirm MCP runtime dependencies needed for native HTTP MCP output and transport.
- [x] 1.2 Add `[mcp]` configuration support in `src/config.rs`, including `allowed_hosts` parsing.
- [x] 1.3 Wire the `/mcp` router into `src/main.rs` so it is served by the existing axum listener.

## 2. Native HTTP MCP server and tool surface

- [x] 2.1 Implement the RMCP Streamable HTTP service in `src/mcp/` and mount it at `/mcp`.
- [x] 2.2 Apply existing Bearer-token middleware to `/mcp` so auth toggle behavior matches `/api/v1/*` and `/status`.
- [x] 2.3 Expose the 5 MCP tools matching `oj-mcp-rs`: `resolve_problem`, `get_problem`, `get_daily_challenge`, `find_similar_problems`, and `get_platform_status`.

## 3. Shared data-path and formatting integration

- [x] 3.1 Reuse in-process handlers and data paths for resolve, problem lookup, daily, similar search, and status instead of loopback HTTP calls.
- [x] 3.2 Promote or adjust shared response structs so the MCP module can consume existing API results without duplicating contracts.
- [x] 3.3 Add Markdown formatting, HTML-to-text/Markdown conversion, and bounded truncation for MCP tool outputs.

## 4. Verification and test coverage

- [x] 4.1 Add tests covering `/mcp` authentication behavior with token auth enabled and disabled.
- [x] 4.2 Add tests covering MCP initialize, `tools/list`, and at least one successful tool invocation.
- [x] 4.3 Run project verification for this change, including formatting, linting/static checks, and tests.

## 5. Documentation and spec alignment

- [x] 5.1 Update `README.md` with native HTTP MCP usage, authentication notes, and deployment expectations.
- [x] 5.2 Update `config.toml.example` with `[mcp].allowed_hosts` examples and comments.
- [x] 5.3 Reconcile implementation with the `http-mcp`, `authentication`, and `config-example` specs before finalizing the change.
