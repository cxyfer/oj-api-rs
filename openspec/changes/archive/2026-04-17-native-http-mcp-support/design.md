# Design: native-http-mcp-support

## Context

`oj-mcp-rs` 已有一套成熟的 MCP tool surface，但它是以 stdio transport 啟動，並透過 HTTP client 呼叫 `oj-api-rs` 的 `/api/v1/*` 與 `/status`。本專案現在要把這套能力直接搬進 axum server 內，改成原生 `/mcp` HTTP transport。

已實作版本的關鍵選擇：
- transport 採 **RMCP Streamable HTTP server**，掛在既有 server 上，不開新 port。
- 授權採 **現有 bearer_auth middleware**，所以 `/mcp` 和 `/api/v1/*` 共用 token toggle 行為。
- MCP tool 儘量重用既有 handler / response model，而不是再做一層 loopback HTTP。

影響範圍：`src/main.rs`、`src/config.rs`、`src/mcp/mod.rs`、多個 `src/api/*` response type 的可見性與 serde 能力。

## Goals / Non-Goals

**Goals:**
- 提供原生 `/mcp` Streamable HTTP endpoint。
- 對齊 `oj-mcp-rs` 的 5 個 tool 名稱、參數與輸出格式。
- 與現有 token auth toggle 完全一致。
- 不依賴 self-HTTP / loopback 呼叫。
- 補齊測試與 OpenSpec 契約。

**Non-Goals:**
- 不提供 stdio MCP transport（`oj-api-rs` 內部不取代 `oj-mcp-rs` 的 CLI 角色）。
- 不新增 legacy SSE-only 相容 surface。
- 不改動現有 REST endpoint 的 JSON contract。
- 不把 crawler/admin 類能力暴露成 MCP tools。

## Decisions

### D1 — Transport 採 Streamable HTTP，端點固定 `/mcp`

**Decision**
- 使用 `rmcp::transport::streamable_http_server::StreamableHttpService`。
- 將 service 掛到 `Router::nest_service("/mcp", ...)`。
- 與現有 axum listener 共用同一個 server lifecycle。

**Rationale**
- 這是目前 MCP HTTP 的主流 transport，能直接對接 HTTP MCP client。
- 不需要多進程或多 binary 協調。

**Alternative rejected**
- 內建 stdio transport：會把 web server 與 CLI server 混在同一 binary 的啟動模式，超出本次需要。

### D2 — `/mcp` 沿用 bearer_auth middleware

**Decision**
- `/mcp` 套用與 `/api/v1/*` 相同的 `crate::auth::bearer_auth` middleware。
- 當 token auth 開啟時，需要 `Authorization: Bearer <token>`。
- 當 token auth 關閉時，`/mcp` 與 public API 一樣不驗 token。

**Rationale**
- 這是最一致、最少驚訝的授權模型。
- 不需再引入第二套 MCP-only secret/token 管理。

### D3 — Tool 實作直接復用程序內 handler 與模型

**Decision**
- MCP tool 直接呼叫既有 handler，例如：
  - `problems::get_problem`
  - `resolve::resolve`
  - `daily::get_daily`
  - `similar::similar_by_problem` / `similar::similar_by_text`
  - `status::get_status`
- 不再建立內部 `reqwest` client 去打自己的 REST API。

**Rationale**
- 減少一層序列化 / 網路 / auth / 路由重複。
- 讓 MCP 與 REST 共用同一份資料讀取與錯誤邏輯。

**Trade-off**
- 為了共用 JSON 反序列化，需要把部分 API response type 提升為可跨模組使用，並補上 `Deserialize`。

### D4 — MCP output 使用 Markdown surface，保留 oj-mcp-rs 慣例

**Decision**
- `get_problem` / `resolve_problem` / `get_daily_challenge` 回傳 Markdown 問題敘述。
- `find_similar_problems` 回傳 Markdown table。
- `get_platform_status` 回傳 Markdown table。
- 長輸出沿用截斷策略，避免單次 tool response 過大。
- HTML statement 透過 `htmd`，失敗時退回 `ammonia::clean_text`。

**Rationale**
- 與既有 `oj-mcp-rs` client 體驗一致，遷移成本最低。

### D5 — 新增 `[mcp] allowed_hosts` 設定

**Decision**
- 在 `Config` 中新增 `McpConfig { allowed_hosts: Vec<String> }`。
- 若設定為空，使用 `disable_allowed_hosts()`，允許所有 Host。
- 若有值，則傳給 RMCP 的 `with_allowed_hosts(...)`。

**Rationale**
- RMCP 預設偏向 loopback host allow-list，不適合直接公開部署。
- 本專案既有部署形態常在反向代理後面，因此需要顯式設定入口。

**Trade-off**
- 空陣列代表較寬鬆；需在 spec / README 明確標註部署風險。

## Risks / Trade-offs

- **[R1] MCP 與 REST 共用 handler，若 future handler shape 大改，MCP 也會跟著受影響** → Mitigation: 用 OpenSpec 固定 `/mcp` tool contract，後續若要改 handler shape，需同步檢視 MCP spec。
- **[R2] 空的 `allowed_hosts` 代表允許任意 Host，公開部署若未經反向代理保護會較寬鬆** → Mitigation: 在 config example 與 README 註明建議在公開部署時明確設定 host allow-list。
- **[R3] `daily` / `similar` 等 tool 仍依賴底層 embedding / crawler 行為，MCP 只是包裝而非新邏輯** → Mitigation: 沿用既有 handler 與測試，避免再做平行實作。

## Migration Plan

1. 新增 `src/mcp/mod.rs` 與 `rmcp` 依賴。
2. 補齊 MCP 所需 config 與 route wiring。
3. 提升 REST response type 的共用性（`Deserialize` / `pub(crate)`）。
4. 新增 `/mcp` auth / initialize / tools/list / tool call 測試。
5. 更新 `README.md`、`config.toml.example` 與 OpenSpec。

## Open Questions

無。此變更的 transport、auth 與 tool scope 已固定。
