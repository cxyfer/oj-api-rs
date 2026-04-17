# Tasks: native-http-mcp-support

## 1. Runtime / config foundation
- [x] 1.1 Add MCP runtime dependencies (`rmcp`, `htmd`, `ammonia`) to `Cargo.toml` and lockfile.
- [x] 1.2 Add `[mcp]` config surface in `src/config.rs` with `allowed_hosts: Vec<String>`.
- [x] 1.3 Wire the MCP router into `src/main.rs` so `/mcp` is served by the existing axum server.

## 2. Native HTTP MCP server
- [x] 2.1 Create `src/mcp/mod.rs` with an RMCP `StreamableHttpService` mounted at `/mcp`.
- [x] 2.2 Apply existing `bearer_auth` middleware to `/mcp` so token toggle behavior matches `/api/v1/*`.
- [x] 2.3 Expose 5 tools matching `oj-mcp-rs`: `resolve_problem`, `get_problem`, `get_daily_challenge`, `find_similar_problems`, `get_platform_status`.

## 3. Shared response / formatting plumbing
- [x] 3.1 Promote public API response structs needed by MCP (`problem`, `resolve`, `daily`, `similar`, `status`) so they can be deserialized inside the MCP module.
- [x] 3.2 Add Markdown formatting helpers in the MCP module, including HTML-to-Markdown conversion and bounded truncation.
- [x] 3.3 Keep MCP implementation in-process by reusing existing handlers instead of loopback HTTP requests.

## 4. Verification
- [x] 4.1 Add backend tests covering `/mcp` auth behavior when token auth is on/off.
- [x] 4.2 Add backend tests covering initialize, `tools/list`, and at least one successful tool call.
- [x] 4.3 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.

## 5. Documentation / spec sync
- [x] 5.1 Update `README.md` with `/mcp` usage and auth notes.
- [x] 5.2 Update `config.toml.example` with `[mcp] allowed_hosts`.
- [x] 5.3 Add / update OpenSpec capability docs for `http-mcp`, `authentication`, and `config-example`.
