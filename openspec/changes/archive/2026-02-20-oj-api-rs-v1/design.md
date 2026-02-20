## Context

現有 `leetcode-daily-discord-bot` 專案中的題目查詢邏輯需抽離為獨立的 Rust RESTful API 後端。現有 SQLite 資料庫（173MB）包含 24,704 道題目與 23,484 筆 embedding，Python 爬蟲與 embedding pipeline 維持不變，Rust 端為純讀取層 + 管理功能。

Stakeholders：Discord bot（現有消費者）、Admin 管理員、未來前端/第三方 API 消費者。

## Goals / Non-Goals

**Goals:**
- 提供穩定的 RESTful API 支援題目查詢、每日一題、相似題搜尋、來源辨識
- Admin 後台支援題目 CRUD、API token 管理、爬蟲觸發
- 與現有 Python 爬蟲/embedding pipeline 共存（共享 SQLite）
- Fail-fast 啟動自檢、graceful shutdown

**Non-Goals:**
- 水平擴展（V1 為單實例部署）
- RPM 限制 / Rate limiting
- API token TTL / 過期機制
- 替換 Python embedding pipeline（保留 subprocess 委託）
- 前端 SPA（Admin 為 server-side rendered HTML）

## Decisions

### D1: HTTP Framework — axum

**選擇**: axum
**理由**: Tokio 生態原生、tower middleware 相容、社群活躍。
**排除**: actix-web（不同 runtime 生態）、warp（filter 組合語法較晦澀）。

### D2: Database — rusqlite + r2d2 (bundled)

**選擇**: `rusqlite` 啟用 `bundled` feature + `r2d2` 同步連線池
**理由**: sqlite-vec 官方 Rust 範例基於 rusqlite；`bundled` 確保 SQLite 版本與 sqlite-vec 相容；r2d2 為 rusqlite 生態主流 pool。
**排除**:
- `sqlx`：async 但 sqlite-vec 靜態連結需 `sqlite3_auto_extension`，與 sqlx 的連線初始化流程整合較複雜。
- `deadpool-sqlite`：更 async-friendly，但 V1 流量預期低，r2d2 + `spawn_blocking` 足夠。

**讀寫分離**: 建立兩個 pool —— read-only pool（`PRAGMA query_only=ON`）供 API 使用，read-write pool 供 Admin 使用。

### D3: sqlite-vec 整合

**選擇**: `sqlite-vec = "0.1.6"` 靜態連結，啟動時透過 `sqlite3_auto_extension` 註冊。
**約束**:
- 註冊 MUST 在任何 r2d2 pool 建立前完成。
- 每個連線初始化時驗證 `SELECT vec_version()` 成功。
- 啟動自檢驗證 `vec_length()` == 768。

### D4: zerocopy 版本

**選擇**: 鎖定 `zerocopy = "0.7"`，使用 `AsBytes` trait。
**理由**: sqlite-vec crate 的依賴基於 0.7 API；升級到 0.8 需全面改用 `IntoBytes`，V1 無此必要。

### D5: 模板引擎 — Askama

**選擇**: `askama`（編譯時型別安全模板）。
**約束**: 預設 HTML auto-escape 防 XSS；題目 content 以 `<iframe sandbox>` 預覽。

### D6: Python subprocess 整合

**選擇**: `tokio::process::Command` 執行 `embedding_cli.py --embed-text`。
**約束**:
- timeout = 30s，使用 `tokio::time::timeout` + `kill_on_drop(true)`。
- 並發限制：`tokio::sync::Semaphore`（permits = 4）防止大量並發開程序。
- stderr 記錄為 `warn` level，不暴露給 API 消費者。
- exit code != 0 或 JSON 解析失敗 → HTTP 502。
- timeout → HTTP 504。
- Rust 端顯式傳遞 `GEMINI_API_KEY` 環境變數，工作目錄設為 `references/leetcode-daily-discord-bot/`。

### D7: 爬蟲觸發 — 非同步 + 單實例鎖

**選擇**: `POST /admin/api/crawlers/trigger` 為非同步，立即回傳 `job_id`（HTTP 202）。
**實作**: `Arc<Mutex<Option<CrawlerJob>>>` 於 AppState，記錄 job_id / source / started_at / status。
**約束**:
- 同一時間僅允許一個爬蟲執行，重複觸發回傳 409。
- timeout = 300s。
- `GET /admin/api/crawlers/status` 查詢當前狀態。

### D8: 認證架構

**選擇**: 雙層認證 —— Bearer token（public API）+ X-Admin-Secret header（admin）。
**約束**:
- `ADMIN_SECRET` 缺失則拒絕啟動（fail-fast）。
- Token 為 64-char hex（32 random bytes），無 TTL。
- 停用/無效/缺失 token → 401。
- Admin 路由不接受 Bearer token 替代。
- `last_used_at` 於每次成功認證時更新。

### D9: 錯誤格式 — RFC 7807

**選擇**: 統一使用 RFC 7807 Problem Details。
**欄位**: `type`（URI）、`title`、`status`（HTTP code）、`detail`。
**驗證錯誤**: 額外含 `errors: [{field, message}]`。
**安全**: `detail` 不暴露內部錯誤（如 Python stderr）。

### D10: API 列表回應格式

**選擇**: `{"data": [...], "meta": {"total", "page", "per_page", "total_pages"}}`。
**列表欄位**: `source`, `id`, `slug`, `title`, `title_cn`, `difficulty`, `rating`, `ac_rate`, `tags`, `contest`, `problem_index`, `link`。
**省略**: `content`, `content_cn`, `similar_questions`, `category`, `paid_only`。

### D11: Similar search 設計

**參數預設/上限**:
| Param | Default | Max |
|-------|---------|-----|
| limit | 10 | 50 |
| threshold | 0.0 | 1.0 |
| over_fetch_factor | 4 | (k cap = 200) |

**約束**:
- `source` 支援多值（逗號分隔）。
- 結果排除 seed problem。
- 結果按 similarity 降序排列。

### D12: CORS 策略

**選擇**: Public API（`/api/v1/*`、`/health`）允許所有 origin；Admin 路由（`/admin/*`）不設 CORS（僅同源）。
**實作**: `tower-http::CorsLayer` 僅套用於 public router。

### D13: 配置管理

所有配置項均可透過環境變數覆寫：

| Env Var | Default | Description |
|---------|---------|-------------|
| `LISTEN_ADDR` | `0.0.0.0:3000` | 監聽地址 |
| `DATABASE_PATH` | `data/data.db` | SQLite 路徑 |
| `ADMIN_SECRET` | (required) | Admin 密鑰 |
| `GEMINI_API_KEY` | (optional) | Python embedding 用 |
| `DB_POOL_MAX_SIZE` | `8` | r2d2 最大連線數 |
| `BUSY_TIMEOUT_MS` | `5000` | SQLite busy timeout |
| `EMBED_TIMEOUT_SECS` | `30` | Python subprocess timeout |
| `CRAWLER_TIMEOUT_SECS` | `300` | 爬蟲 timeout |
| `OVER_FETCH_FACTOR` | `4` | KNN over-fetch 倍數 |
| `GRACEFUL_SHUTDOWN_SECS` | `10` | 關閉等待時間 |
| `RUST_LOG` | `info` | tracing level |

### D14: 初始化順序

1. 載入 config + 初始化 `tracing`
2. 註冊 `sqlite3_auto_extension`（sqlite-vec）
3. 建立 r2d2 read-only pool + read-write pool（WAL mode、busy_timeout）
4. 啟動自檢：DB 連線、`vec_version()`、`vec_length()` == 768
5. 組裝 `AppState`（pools、config、crawler lock、embed semaphore）
6. 組裝 axum routers + middleware layers
7. 啟動 HTTP server + graceful shutdown handler

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| sqlite-vec + rusqlite 版本不相容 | 編譯失敗或 runtime 異常 | 固定 `sqlite-vec 0.1.6` + `rusqlite 0.31 bundled`；啟動自檢驗證 |
| Python subprocess 延遲/失敗 | 文字相似搜尋不可用 | 30s timeout + kill_on_drop + semaphore(4) 限流；502/504 錯誤回應 |
| SQLite WAL 併發 busy | API 請求偶發延遲 | `busy_timeout=5000ms`；讀寫分離 pool；短交易 |
| KNN over-fetch 記憶體開銷 | limit 大時記憶體突增 | `k` 上限 200；`limit` 上限 50 |
| Dockerfile 包含 Rust + Python | Image 體積大 | Multi-stage build 最小化；Python 僅含必要依賴 |
| Admin 題目刪除與 similar 查詢併發 | KNN 結果 JOIN 失敗 | LEFT JOIN + null 過濾；刪除包在交易內 |

## Migration Plan

1. **Pre-implementation**: Python 端新增 `embedding_cli.py --embed-text` CLI
2. **Build**: `cargo build --release` 產出靜態二進位
3. **Deploy**: Docker image 含 Rust binary + Python runtime + templates + static
4. **Data**: `data/data.db` 以 volume mount 掛載，不打包進 image
5. **Rollback**: 停止 Rust 容器即可，Python 爬蟲獨立運作不受影響

## Open Questions

（無 — 所有歧義已在規劃階段解決）
