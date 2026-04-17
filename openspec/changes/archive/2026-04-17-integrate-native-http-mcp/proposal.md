## Why

目前 `oj-api-rs` 的 MCP 能力主要存在於外部包裝器 `oj-mcp-rs`：MCP client 先連到獨立的 stdio server，再由該 server 反向呼叫 `oj-api-rs` 的 REST API。這會造成部署與維護分裂，並讓 MCP 的授權、工具定義與文件容易和本專案漂移。現在專案內已經有未提交的原生 HTTP MCP 實作與規格草稿，因此需要把這些能力正式整理成 OpenSpec 變更，讓後續整合、驗證與文件同步都有明確依據。

## What Changes

- 在 `oj-api-rs` 既有 axum server 內原生提供 HTTP MCP `/mcp` 端點，而不是依賴外部 stdio 包裝器。
- 將 `oj-mcp-rs` 已實作的 5 個 MCP tools 整合到本專案：
  - `resolve_problem`
  - `get_problem`
  - `get_daily_challenge`
  - `find_similar_problems`
  - `get_platform_status`
- 讓 `/mcp` 的授權行為與現有 public API Bearer token toggle 保持一致。
- 補齊 MCP 的 Host allow-list 設定與對應文件，讓公開部署時能控制 `/mcp` 的 Host 驗證。
- 將目前 repo 中尚未提交的 MCP 相關實作、規格與文件修改納入同一個 change，整理為可驗證的 OpenSpec 契約。

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `http-mcp`: 定義原生 `/mcp` Streamable HTTP transport、工具清單、輸出格式、大小限制與 Host allow-list 契約。
- `authentication`: 擴充 Bearer token gate 的適用範圍，使 `/mcp` 與 `/api/v1/*`、`/status` 使用一致的驗證邏輯與 token toggle 行為。
- `config-example`: 擴充 `config.toml.example`，加入 `[mcp] allowed_hosts` 的設定範例與說明。

## Impact

- Runtime / routing: `src/main.rs`, `src/config.rs`, `src/mcp/`
- Shared API / model surface: `src/api/*`, `src/models.rs`, `src/db/problems.rs`
- Documentation / config: `README.md`, `config.toml.example`
- OpenSpec artifacts: `openspec/specs/http-mcp/`, `openspec/specs/authentication/spec.md`, `openspec/specs/config-example/spec.md`
- Dependencies and formatting pipeline related to native MCP output, including `rmcp`, `htmd`, and `ammonia`
