# Proposal: native-http-mcp-support

## Why

目前 `oj-api-rs` 的 MCP 能力只存在於外部包裝器 `oj-mcp-rs`：MCP client 先連到獨立的 stdio server，再由該 server 反向呼叫 `oj-api-rs` 的 REST API。這造成三個問題：

1. 部署面要同時維護 API server 與另一個 MCP binary / package。
2. MCP auth 與 API auth 雖然語意相近，實際上仍要靠外部包裝器重做一層。
3. 相同能力（resolve / get_problem / daily / similar / status）被拆成兩套 surface，文件與維護容易漂移。

本次變更要讓 `oj-api-rs` **原生提供 HTTP MCP**，把 `oj-mcp-rs` 已驗證的工具能力直接收斂到本專案內。

## What Changes

- 在現有 axum server 內新增原生 **Streamable HTTP MCP** 端點 `/mcp`。
- 將 `oj-mcp-rs` 既有 5 個工具能力移植到 `oj-api-rs` 內部：
  - `resolve_problem`
  - `get_problem`
  - `get_daily_challenge`
  - `find_similar_problems`
  - `get_platform_status`
- `/mcp` 授權策略與現有 public API 一致：**沿用 Bearer token 開關**。
- 新增 `[mcp] allowed_hosts` 設定，用於控制 HTTP MCP 的 Host allow-list。
- 新增 OpenSpec capability：`http-mcp`。

## Capabilities

### New Capabilities
- `http-mcp`: 定義 `/mcp` 的 Streamable HTTP transport、tool surface、授權與輸出契約。

### Modified Capabilities
- `authentication`: 補充 Bearer token gate 對 `/mcp` 的適用規則，以及 token auth toggle 關閉時的繞過行為。
- `config-example`: 補充 `[mcp] allowed_hosts` 的設定文件契約。

## Impact

- Rust runtime: `src/main.rs`, `src/config.rs`, `src/mcp/mod.rs`
- Shared API response models: `src/api/*`, `src/models.rs`, `src/db/problems.rs`
- Public docs/config: `README.md`, `config.toml.example`
- Dependencies: `rmcp`, `htmd`, `ammonia`
