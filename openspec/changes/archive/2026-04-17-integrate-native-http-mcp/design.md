## Context

`oj-mcp-rs` 已經提供一套可用的 MCP tool surface，但它以 stdio transport 啟動，並透過 HTTP client 回呼 `oj-api-rs` 的 REST API。這代表 MCP 的 transport、授權、文件與部署生命週期都落在本專案之外，即使實際資料來源仍是 `oj-api-rs`。

本次 change 的目標不是重新設計 MCP 功能，而是把已在 `oj-mcp-rs` 驗證過的能力收斂回 `oj-api-rs` 內部，改以原生 HTTP MCP 對外提供。repo 內目前已經有未提交的 `src/mcp/`、`openspec/specs/http-mcp/`、README 與 config 修改，因此 design 需要把這些既有方向整理成清楚的技術決策，作為後續 specs 與驗證依據。

約束如下：
- `/mcp` 必須與既有 axum server 共用同一個 listener 與部署流程。
- `/mcp` 的授權行為必須和 `/api/v1/*`、`/status` 一致，不能再發明第二套 MCP-only auth。
- tool contract 要盡量對齊 `oj-mcp-rs`，降低既有 MCP client 的遷移成本。
- 實作應優先重用程序內既有 handler / model / 查詢流程，而不是 loopback 呼叫自己。

## Goals / Non-Goals

**Goals:**
- 在 `oj-api-rs` 內提供原生 `/mcp` Streamable HTTP endpoint。
- 對齊 `oj-mcp-rs` 的 5 個工具名稱、主要參數語意與輸出格式。
- 讓 `/mcp` 與現有 Bearer token toggle 使用完全一致的驗證邏輯。
- 讓 MCP tool 實作復用既有 API handler / data path，而不是額外建立 self-HTTP client。
- 補齊 config、README 與 OpenSpec，使 HTTP MCP 成為正式受規格保護的能力。

**Non-Goals:**
- 不在 `oj-api-rs` 內提供 stdio MCP transport，也不取代 `oj-mcp-rs` 的 CLI 使用情境。
- 不新增 crawler、admin 或其他尚未在 `oj-mcp-rs` 出現的 MCP tools。
- 不改動現有 REST endpoint 的 JSON contract。
- 不將 HTTP MCP 延伸成額外的獨立 port 或獨立服務。

## Decisions

### D1 — MCP transport 採用 Streamable HTTP，固定掛在 `/mcp`

**Decision**
- 使用 RMCP 的 Streamable HTTP server，直接掛在現有 axum router 的 `/mcp`。
- `/mcp` 與 `/health`、`/status`、`/api/v1/*` 共用同一個 server lifecycle。

**Rationale**
- 這樣可以讓 `oj-api-rs` 原生提供 HTTP MCP，而不需要維護額外 binary 或 sidecar。
- 共用 listener 與路由層可降低部署複雜度，並避免 API 與 MCP 分別配置不同網域、port 或 process。

**Alternatives considered**
- 保持外部 `oj-mcp-rs` stdio 包裝器不變：部署與文件仍然分裂，且 MCP 仍依賴 loopback HTTP。
- 在 `oj-api-rs` 內額外提供 stdio transport：會讓 web server binary 混入 CLI 啟動模式，超出本次需求。

### D2 — `/mcp` 沿用既有 Bearer token middleware

**Decision**
- `/mcp` 套用與 public API 相同的 `bearer_auth` middleware 與 token toggle。
- token auth 開啟時，MCP initialize / tool calls 都需要合法 Bearer token；token auth 關閉時，`/mcp` 與 public API 一樣免 token。

**Rationale**
- 這是使用者與部署者最可預期的模型：REST 和 MCP 都是同一份公開資料面，不應有不同的驗證語意。
- 共用現有驗證流程可以避免 MCP-only secret、第二份 token store 或例外行為。

**Alternatives considered**
- 為 `/mcp` 單獨新增 secret/header：增加管理負擔，也和現有 public API 權限模型不一致。
- 永遠讓 `/mcp` 無 auth：不符合目前 API 可透過 token gate 保護的部署方式。

### D3 — MCP tools 直接重用程序內 handler 與資料流程

**Decision**
- `resolve_problem`、`get_problem`、`get_daily_challenge`、`find_similar_problems`、`get_platform_status` 直接重用 `oj-api-rs` 既有 handler / query path / model，而不是用 `reqwest` 回打本服務。
- 必要時提升部分 response struct 的可見性與 serde 能力，讓 MCP 模組可直接消費現有回傳資料。

**Rationale**
- 減少一層網路、序列化、授權與錯誤轉換邏輯，降低實作漂移風險。
- 可以保證 MCP 和 REST 對相同 problem data、resolve 規則、similar search 與 status 統計使用同一份核心邏輯。

**Alternatives considered**
- 在 MCP 層建立內部 HTTP client 呼叫 `/api/v1/*`：會複製 transport 與 auth 成本，也讓錯誤處理更脆弱。
- 重寫一套與 REST 平行的 MCP-only data path：維護成本高，容易與現有 API 行為分岔。

### D4 — 對外輸出維持 `oj-mcp-rs` 風格的 Markdown surface

**Decision**
- `resolve_problem`、`get_problem`、`get_daily_challenge` 回傳 Markdown 問題內容。
- `find_similar_problems`、`get_platform_status` 回傳 Markdown table。
- HTML statement 轉換優先使用 `htmd`，失敗時退回文字清洗；並對輸出加上大小上限與截斷標記。

**Rationale**
- 與 `oj-mcp-rs` 對齊可降低 MCP client 遷移成本，也讓 LLM 端延續既有閱讀體驗。
- 對 problem statement 做大小限制能避免單次 tool response 過大，影響 client 穩定性。

**Alternatives considered**
- 直接回傳 REST JSON：可讀性較差，也和既有 MCP 客戶端預期不同。
- 不設大小限制：長題面或 HTML 內容可能產生過大 payload。

### D5 — Host allow-list 由 `[mcp].allowed_hosts` 顯式控制

**Decision**
- 在 config 增加 `mcp.allowed_hosts: Vec<String>`。
- 有值時，`/mcp` 僅接受 allow-list 中的 Host；空陣列則視為停用 Host allow-list。

**Rationale**
- MCP HTTP 常部署在反向代理或多 Host 環境下，應讓部署者能明確控制 Host 驗證行為。
- 透過 config example 與 README 一起文件化，可避免 RMCP 預設行為對公開部署造成意外限制。

**Alternatives considered**
- 完全沿用 library 預設 Host 驗證：對既有部署型態可能過於隱晦或不相容。
- 永遠允許所有 Host：雖然簡單，但少了公開部署時的最基本限制能力。

## Risks / Trade-offs

- **MCP 與 REST 共享同一批 handler / model，未來若 REST response shape 調整，MCP 也可能連帶受影響** → 以 OpenSpec 固定 tool contract，後續變更時需同時檢查 REST 與 MCP。
- **空的 `allowed_hosts` 代表停用 Host allow-list，若公開部署未經反向代理保護會較寬鬆** → 在 `config.toml.example` 與 README 清楚標示語意與建議設定。
- **MCP 只是把既有 daily / similar / resolve 能力換 transport 暴露，底層 crawler / embedding 風險仍然存在** → 明確標註這是 in-process reuse，而非新邏輯來源，並以既有 API 行為作為驗證基準。
- **將 response struct 提升為跨模組可用會稍微增加 API 內部耦合** → 只提升 MCP 實作必需的最小範圍，避免擴散成全面公開型別。

## Migration Plan

1. 確認 `src/mcp/`、router wiring、config 與文件修改都落在同一個 change 範圍。
2. 以 `http-mcp`、`authentication`、`config-example` 三個 capability 補齊 spec delta，固定 `/mcp` contract。
3. 對照 `oj-mcp-rs` 的 5 個 tools 與目前未提交實作，補足缺漏與驗證案例。
4. 執行 backend 測試，覆蓋 `/mcp` auth、initialize、tools/list 與至少一個成功 tool call。
5. 文件完成後再進入 tasks / implementation 驗證，避免 code 與 spec 再次漂移。

## Open Questions

目前沒有阻擋本 change 的開放問題。HTTP transport、auth 模型、tool 範圍與 config 方向都已由現有未提交實作與 archived change 收斂完成；後續重點是把 spec 與驗證補齊。
