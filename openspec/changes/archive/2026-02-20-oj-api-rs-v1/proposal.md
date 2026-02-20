# Proposal: OJ API RS v1

## Context

將現有 `leetcode-daily-discord-bot` 專案中的題目查詢邏輯抽離，以 Rust 後端提供 RESTful API，支援 LeetCode / AtCoder / Codeforces 題目查詢、LeetCode 每日一題、以及基於 sqlite-vec 的相似題目搜尋。

現有資料庫（173MB）已包含 24,704 道題目與 23,484 筆 embedding 資料，Python 爬蟲與 embedding pipeline 維持不變，Rust 端為純讀取層 + 管理功能。

## Design Decisions

| Item | Decision | Rationale |
|------|----------|-----------|
| Embedding | Python 生成，Rust 只讀 | 沿用現有 Gemini embedding pipeline，Rust 端僅載入 sqlite-vec 做 KNN 查詢 |
| Text Embedding (即時) | 委託 Python 子程序 | `/similar` 文字查詢模式需即時生成 embedding，呼叫現有 Python pipeline 而非在 Rust 重複實現 Gemini client |
| Source Detection | 完整移植 discord-bot 邏輯 | 包含 Luogu/UVA/SPOJ 等辨識，為未來擴充來源預留 |
| Resolve Endpoint | 獨立 `/api/v1/resolve/{query}` | 避免與現有 `/problems/{source}` 路由歧義，由前端決定是否跳轉 |
| ID Ambiguity | 純數字→LC、數字+大寫字母→CF、abc/arc/agc/ahc→AC | 沿用 discord-bot 預設規則，符合直覺 |
| Admin UI | Axum 內建簡易 HTML | Askama/Tera 模板引擎，Bearer token 驗證，避免 CSRF 風險 |
| Auth | 單一 admin secret + 多 API token，無 RPM 限制 | V1 最簡方案，後續可擴充 |
| DB Library | rusqlite + r2d2 | sqlite-vec 官方範例即為 rusqlite，整合最直接 |

## Existing Database Schema

直接複用現有 `data.db`，Rust 端以唯讀為主（admin CRUD 除外）。

### problems (PK: source, id)

| Column | Type | Notes |
|--------|------|-------|
| id | TEXT NOT NULL | 題目 ID（LC: 數字, CF: "1234A", AC: "abc123_a"）|
| source | TEXT NOT NULL | "leetcode" / "atcoder" / "codeforces" |
| slug | TEXT NOT NULL | URL slug |
| title | TEXT | 英文標題 |
| title_cn | TEXT | 中文標題（LeetCode 專用）|
| difficulty | TEXT | "Easy"/"Medium"/"Hard"（LC）或 null |
| ac_rate | REAL | 通過率 |
| rating | REAL | 難度分（CF rating / LC rating）|
| contest | TEXT | 所屬比賽 |
| problem_index | TEXT | 比賽中的題號 |
| tags | TEXT | JSON array of tag strings |
| link | TEXT | 題目 URL |
| category | TEXT | "Algorithms"/"Database"/"Shell"（LC）|
| paid_only | INTEGER | 0/1（LC 專用）|
| content | TEXT | 題目描述 HTML |
| content_cn | TEXT | 中文描述 HTML（LC 專用）|
| similar_questions | TEXT | JSON array |

### daily_challenge (PK: date, domain)

| Column | Type | Notes |
|--------|------|-------|
| date | TEXT NOT NULL | ISO 8601 "YYYY-MM-DD" |
| domain | TEXT NOT NULL | "com" / "cn" |
| id | TEXT NOT NULL | 題目 ID（與 problems.id 型別一致）|
| slug | TEXT NOT NULL | URL slug |
| (其餘欄位同 problems) | | |

### vec_embeddings (sqlite-vec virtual table)

| Column | Type | Notes |
|--------|------|-------|
| source | TEXT | 題目來源 |
| problem_id | TEXT | 題目 ID |
| embedding | float[768] | Gemini embedding-001 向量 |

### problem_embeddings (embedding metadata)

| Column | Type | Notes |
|--------|------|-------|
| source | TEXT NOT NULL | PK part 1 |
| problem_id | TEXT NOT NULL | PK part 2 |
| rewritten_content | TEXT | LLM 改寫後的題目摘要 |
| model | TEXT NOT NULL | 使用的 embedding model 名稱 |
| dim | INTEGER NOT NULL | 向量維度（768）|
| updated_at | TEXT NOT NULL | ISO 8601 timestamp |

### api_tokens (新增)

| Column | Type | Notes |
|--------|------|-------|
| token | TEXT PRIMARY KEY | Bearer token 值 |
| label | TEXT | 用途標記 |
| created_at | INTEGER NOT NULL | Unix timestamp |
| last_used_at | INTEGER | 最後使用時間 |
| is_active | INTEGER NOT NULL DEFAULT 1 | 啟用/停用 |

### Non-managed Tables (Rust 端不管理，保留不動)

以下表由 Python Discord bot 使用，Rust 端不做 CRUD 操作，亦不得刪除或修改 schema：

| Table | Purpose |
|-------|---------|
| server_settings | Discord 伺服器設定（server_id, channel_id, role_id, post_time, timezone）|
| llm_translate_results | LLM 翻譯回應快取（problem_id, domain）|
| llm_inspire_results | LLM 靈感回應快取（problem_id, domain）|

## Data Statistics

| Source | Problems | Embeddings |
|--------|----------|------------|
| LeetCode | 3,715 | 2,995 |
| AtCoder | 8,183 | 7,881 |
| Codeforces | 12,806 | 12,608 |
| Daily Challenges | 917 | - |

## API Endpoints

### Public API (require Bearer token)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | 健康檢查（回傳 DB 連線狀態、sqlite-vec 載入狀態、版本資訊；無需 token）|
| GET | `/api/v1/problems/{source}/{id}` | 依 source + id 查詢題目完整內容 |
| GET | `/api/v1/problems/{source}` | 列出/篩選題目（支援 tags, difficulty, page, per_page；per_page 預設 20，最大 100）|
| GET | `/api/v1/resolve/{query}` | 自動辨識題目來源：接受任意格式（純 ID、URL、source:id 前綴），回傳 `{ source, id, problem }` |
| GET | `/api/v1/daily?domain={com\|cn}&date={YYYY-MM-DD}` | LeetCode 每日一題（date 預設今日 UTC，最早 2020-04-01）|
| GET | `/api/v1/similar/{source}/{id}?limit={n}&threshold={f}` | 以題找題：取得該題 embedding → KNN 查詢（threshold 為 similarity 0-1，越大越相似）|
| GET | `/api/v1/similar?query={text}&limit={n}&threshold={f}&source={filter}` | 以文字找題：委託 Python 生成 embedding → KNN 查詢（需先實作 `--embed-text` CLI）|

### Admin API (require X-Admin-Secret header)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/admin/` | 管理後台首頁（HTML）|
| GET | `/admin/problems` | 題目管理頁面（HTML）|
| POST | `/admin/api/problems` | 新增題目 |
| PUT | `/admin/api/problems/{source}/{id}` | 更新題目 |
| DELETE | `/admin/api/problems/{source}/{id}` | 刪除題目 + 對應 embedding |
| POST | `/admin/api/crawlers/trigger` | 觸發爬蟲（subprocess，單實例鎖：已在執行時回傳 409 Conflict）|
| GET | `/admin/api/tokens` | 列出 API tokens |
| POST | `/admin/api/tokens` | 建立新 token |
| DELETE | `/admin/api/tokens/{token}` | 撤銷 token |

## Tech Stack

| Component | Choice |
|-----------|--------|
| Language | Rust (stable, latest edition) |
| HTTP Framework | axum |
| Template Engine | askama (compile-time, type-safe) |
| Database | rusqlite + r2d2 connection pool |
| Vector Search | sqlite-vec crate (static link) |
| Serialization | serde + serde_json |
| Regex | regex (source detection patterns) |
| Vector I/O | zerocopy (zero-copy f32 slice passing)；注意 0.8+ 版本 `AsBytes` 已更名為 `IntoBytes`，實際版本需配合 sqlite-vec crate 依賴 |
| Auth | Bearer token middleware (API) + X-Admin-Secret (admin) |
| CORS | tower-http CorsLayer（V1 預設允許所有 origin）|
| Logging | tracing + tracing-subscriber（structured logging）|
| Async Runtime | tokio |
| Process Mgmt | tokio::process::Command (crawler trigger) |

## Architecture

```
Python Crawlers (existing, unchanged)
  leetcode.py / atcoder.py / codeforces.py
  embedding_cli.py --build --source all
       │
       │ SQLite WAL mode (write)
       ▼
  ┌─────────────┐
  │   data.db    │  (shared SQLite file)
  │  + sqlite-vec│
  └──────┬──────┘
         │ SQLite WAL mode (read)
         ▼
  ┌──────────────────────────────┐
  │     Rust Backend (axum)      │
  │                              │
  │  ┌─────────┐  ┌───────────┐ │
  │  │ API     │  │ Admin     │ │
  │  │ Routes  │  │ Routes +  │ │
  │  │ (JSON)  │  │ HTML UI   │ │
  │  └────┬────┘  └─────┬─────┘ │
  │       │             │       │
  │  ┌────▼─────────────▼────┐  │
  │  │  rusqlite + r2d2 pool │  │
  │  │  + sqlite-vec loaded  │  │
  │  └──────────────────────┘  │
  └──────────────────────────────┘
```

## Technical Constraints

### SQLite Concurrency

- 必須啟用 WAL mode：每個 r2d2 連線初始化時執行 `PRAGMA journal_mode=WAL`
- API 連線設為 `PRAGMA query_only=ON`（唯讀），Admin 連線不設此 pragma
- Python 爬蟲寫入時 Rust 端讀取不受影響（WAL 特性）

### sqlite-vec Integration

```rust
use sqlite_vec::sqlite3_vec_init;
use rusqlite::ffi::sqlite3_auto_extension;

unsafe {
    sqlite3_auto_extension(Some(std::mem::transmute(
        sqlite3_vec_init as *const (),
    )));
}
```

- 向量以 `Vec<f32>` + `zerocopy::AsBytes`（或 0.8+ 的 `IntoBytes`）傳遞，避免 JSON 序列化
- 從 DB 讀取向量時需支援雙格式：先嘗試 binary float32 LE 解析，失敗則嘗試 JSON 字串解析（舊版相容）
- KNN 查詢：`SELECT source, problem_id, distance FROM vec_embeddings WHERE embedding MATCH ? AND k = ?`
- `k` 值使用 over-fetch 策略：`k = limit * over_fetch_factor`（factor 可配置，預設 4），取回後在記憶體中做 source 過濾及 threshold 過濾，最後截斷至 `limit`
- 距離轉換：`similarity = 1.0 - distance`，API 回傳 similarity score（0-1），`threshold` 參數語意為最小 similarity
- 啟動時驗證 `vec_length()` 一致性（預期 768）

### Source Detection Module (detect.rs)

移植自 `references/leetcode-daily-discord-bot/utils/source_detector.py`。

**辨識優先順序**：

1. **URL 辨識**：解析 atcoder.jp / leetcode.com / leetcode.cn / codeforces.com / luogu.com.cn 的 URL
2. **前綴格式**：`source:id`（如 `atcoder:abc321_a`、`codeforces:2179A`）
3. **ID 模式辨識**：
   - 純數字 → LeetCode（如 `2000`）
   - `CF\d+[A-Z]\d*` 或 `\d+[A-Z]\d*` → Codeforces（如 `CF2000A`、`2000A`、`1999B1`）
   - `(abc|arc|agc|ahc)\d+_[a-z]\d*` → AtCoder（如 `abc321_a`）
   - Luogu 模式（`P/B/T/U` + 數字、`AT_` 前綴、`UVA`/`SP` 前綴）→ 預留，目前 DB 無資料
4. **預設**：無法辨識時視為 LeetCode slug

**正則表達式清單**（Rust `regex` crate）：

```rust
// URL patterns
ATCODER_URL: r"atcoder\.jp/contests/([^/]+)/tasks/([^/?#]+)"
LEETCODE_URL: r"leetcode\.(?:com|cn)/(?:contest/[^/]+/)?problems/([^/?#]+)"
CODEFORCES_URL: r"\b(?:https?://)?(?:www\.)?codeforces\.com/(?:contest/(\d+)/problem/([A-Z0-9]+)|problemset/problem/(\d+)/([A-Z0-9]+))"
LUOGU_URL: r"luogu\.com\.cn/problem/([A-Z0-9_]+)"

// ID patterns
CF_ID: r"^(?i:CF)?\d+[A-Z]\d*$"
ATCODER_ID: r"^(?i:abc|arc|agc|ahc)\d+_[a-z]\d*$"
LUOGU_ID: r"^(?i:[PBTU]\d+|CF\d+[A-Z]|AT_(?:abc|arc|agc|ahc)\d+_[a-z]\d*|UVA\d+|SP\d+)$"
```

**有效來源常數**：`["atcoder", "leetcode", "codeforces", "luogu", "uva", "spoj"]`

**回傳型別**：`(source: &str, id: String)`

**Resolve Endpoint 回應格式**：
```json
{
  "source": "codeforces",
  "id": "2000A",
  "problem": { /* 完整題目物件，若存在 */ }
}
```
若辨識成功但題目不存在於 DB，`problem` 為 `null`，HTTP 200。

### Similar Problem Search Flow

**模式 1：以題找題**
```
GET /api/v1/similar/{source}/{id}?limit=10&threshold=0.7
  → 從 vec_embeddings 取得該題的 embedding vector
  → 若不存在則回傳 404
  → embedding MATCH ? AND k = ? 做 KNN（k = limit * over_fetch_factor）
  → 計算 similarity = 1.0 - distance
  → 過濾 similarity < threshold 的結果
  → 若有 source filter 則過濾 source
  → 截斷至 limit 筆
  → JOIN problems 表取得 title, difficulty, link 等資訊
  → 回傳 JSON array（含 similarity score）
```

**模式 2：以文字找題**
```
GET /api/v1/similar?query=binary+search+on+sorted+array&limit=5&threshold=0.5&source=leetcode
  → 驗證 query 非空（最少 3 字元）
  → 呼叫 Python 子程序生成 embedding：
    tokio::process::Command::new("python")
      .args(["embedding_cli.py", "--embed-text", "<query>"])
      .output()
  → Python 端流程：EmbeddingRewriter 改寫 → EmbeddingGenerator 生成向量
  → Python stdout 輸出 JSON: [f32; 768]
  → Rust 解析向量 → KNN 查詢 → 回傳結果
  → timeout: 30s（Gemini API 延遲）
```

**Python 子程序介面**（需在 `embedding_cli.py` 新增）：
```bash
python embedding_cli.py --embed-text "find two numbers that sum to target"
# stdout: {"embedding": [0.123, -0.456, ...], "rewritten": "..."}
```

### Daily Challenge Date Validation

- `date` query parameter 格式：`YYYY-MM-DD`（regex `^\d{4}-\d{2}-\d{2}$`）
- 範圍限制：`2020-04-01` ≤ date ≤ today（UTC）
- 省略 `date` 時預設為今日 UTC
- 無效格式、超出範圍均回傳 400

### Text-based Similar Search (Python Subprocess)

**前置需求**：需在 Python 端 `embedding_cli.py` 新增 `--embed-text` CLI 旗標（目前尚未實作，為阻塞性前置任務）。

**新增 CLI 介面**（`embedding_cli.py --embed-text`）：

```bash
# input
python embedding_cli.py --embed-text "find two numbers that sum to target"

# stdout (JSON, single line)
{"embedding": [0.123, -0.456, ...], "rewritten": "Given array, find two indices..."}
```

**Rust 端整合**：
- `tokio::process::Command` 執行，capture stdout
- timeout: 30 秒（Gemini API 可能延遲）
- stderr 記錄為 warning log
- exit code != 0 或 JSON 解析失敗 → 502 Bad Gateway
- Rust config 統一管理所有環境變數，啟動子程序時明確傳遞：
  - `GEMINI_API_KEY`（或 `GOOGLE_API_KEY` / `GOOGLE_GEMINI_API_KEY`）
  - 工作目錄設為 `references/leetcode-daily-discord-bot/`（Python 端讀取自身 `config.toml`）

### Admin HTML Security

- Askama 模板預設 HTML escape（防 XSS）
- 題目 content（原始 HTML）以純文字或 iframe sandbox 預覽
- 使用 X-Admin-Secret header 驗證（非 cookie，免 CSRF）
- `tower-http::ServeDir` 提供靜態資源（已有 path traversal 防護）

## Edge Cases

1. **AtCoder 題目語言**：部分題目只有日文。API 回傳時 `content` 可能為空或為日文，不做強制翻譯。
2. **Codeforces ID 格式**：`1234A` 非純數字，路由需接受 `{id}` 為任意字串。
3. **LeetCode paid_only**：`paid_only=1` 的題目可能缺少 `content`，API 回應中包含此欄位供前端判斷。
4. **Embedding 缺失**：`/similar/{source}/{id}` 查詢時若該題無 embedding，回傳 404 並附錯誤訊息。
5. **Daily challenge 僅限 LeetCode**：AtCoder / Codeforces 無官方每日一題。`/daily` 端點的 `domain` 只接受 `com` / `cn`。
6. **tags / similar_questions JSON 解析**：欄位可能為 null、空字串或非法 JSON。Rust 端需以 `Option<Vec<String>>` 處理，解析失敗時降級為空陣列。
7. **Embedding 維度變更**：若日後更換 model，需 Python 端 `--rebuild` 重建。Rust 端啟動時檢查 `vec_length()` 是否為 768，不一致則 log warning。
8. **Resolve 歧義**：`/resolve/1234A` 辨識為 Codeforces，但 DB 中可能不存在。回傳 200 + `problem: null`，由前端呈現「題目不存在」。
9. **Daily 日期範圍**：`date` 參數必須為 `YYYY-MM-DD` 格式，且 >= `2020-04-01`（LeetCode 每日一題起始日）。超出範圍回傳 400。未來日期亦回傳 400。
10. **Daily 無資料**：若 DB 中無指定日期的 daily challenge（例如歷史資料未爬取），回傳 404。
11. **Similar 文字查詢 Python 失敗**：Python 子程序可能因 Gemini API key 無效、API 限流、或 timeout 失敗。回傳 502 + 錯誤訊息。
12. **Similar 文字查詢過短**：`query` 參數少於 3 字元時回傳 400。
13. **Resolve URL 輸入**：query 可能是完整 URL，需 URL decode 後再辨識。路由中的 `{query}` 需支援 path encoding。
14. **Luogu/UVA/SPOJ 辨識結果**：辨識為非三大 OJ 的來源時，`problem` 固定為 `null`（DB 無資料），`source` 仍正常回傳。

## Crawler Entry Points

| Source | Script | Class | CLI Usage |
|--------|--------|-------|-----------|
| LeetCode | `leetcode.py` | `LeetCodeClient` | `python leetcode.py` |
| AtCoder | `atcoder.py` | `AtCoderClient` | `python atcoder.py` |
| Codeforces | `codeforces.py` | `CodeforcesClient` | `python codeforces.py` |
| Embeddings | `embedding_cli.py` | - | `python embedding_cli.py --build --source all` |

Admin 觸發爬蟲時，透過 `tokio::process::Command` 執行上述腳本，需設定 timeout（建議 300s）。執行結果以 exit code + stdout/stderr 返回。使用 `AtomicBool` 或 `Mutex` 於 AppState 中實作單實例鎖，同一時間僅允許一個爬蟲執行，重複觸發回傳 409 Conflict。

## Project Structure (Proposed)

```
oj-api-rs/
├── Cargo.toml
├── Dockerfile
├── .env.example              # 所有環境變數範例
├── src/
│   ├── main.rs              # Entry point, server setup
│   ├── config.rs             # Configuration (env / toml)
│   ├── db/
│   │   ├── mod.rs            # r2d2 pool init + sqlite-vec registration
│   │   ├── problems.rs       # Problem queries
│   │   ├── daily.rs          # Daily challenge queries
│   │   ├── embeddings.rs     # Vector search queries
│   │   └── tokens.rs         # API token CRUD
│   ├── api/
│   │   ├── mod.rs            # API router
│   │   ├── problems.rs       # GET /api/v1/problems/...
│   │   ├── resolve.rs        # GET /api/v1/resolve/{query} — source detection
│   │   ├── daily.rs          # GET /api/v1/daily
│   │   ├── similar.rs        # GET /api/v1/similar/... (both modes)
│   │   └── error.rs          # API error types (RFC 7807)
│   ├── admin/
│   │   ├── mod.rs            # Admin router
│   │   ├── handlers.rs       # Admin API handlers
│   │   └── pages.rs          # HTML page handlers
│   ├── auth/
│   │   └── mod.rs            # Bearer token + admin secret middleware
│   ├── detect.rs             # Source detection logic (ported from Python)
│   └── models.rs             # Shared types (Problem, DailyChallenge, etc.)
├── templates/                # Askama HTML templates
│   ├── base.html
│   ├── admin/
│   │   ├── index.html
│   │   ├── problems.html
│   │   └── tokens.html
├── static/                   # CSS / JS for admin UI
├── data/
│   └── data.db               # Shared SQLite (existing)
└── references/
    └── leetcode-daily-discord-bot/  # Python reference (existing)
```

## Success Criteria

1. `GET /api/v1/problems/leetcode/1` 回傳 Two Sum 完整資料（< 100ms）
2. `GET /api/v1/problems/codeforces/1234A` 回傳對應題目
3. `GET /api/v1/daily?domain=com` 回傳今日 LeetCode 每日一題
4. `GET /api/v1/daily?domain=com&date=2024-01-15` 回傳指定日期的每日一題
5. `GET /api/v1/daily?domain=com&date=2019-01-01` 回傳 400（早於 2020-04-01）
6. `GET /api/v1/similar/leetcode/1?limit=5` 回傳 5 道相似題目（含 similarity score，0-1 範圍）
7. `GET /api/v1/similar?query=binary+search&limit=5` 以文字查詢回傳相似題目（委託 Python embedding）
8. `GET /api/v1/resolve/2000` 回傳 `{ source: "leetcode", id: "2000", problem: {...} }`
9. `GET /api/v1/resolve/CF2000A` 回傳 `{ source: "codeforces", id: "2000A", problem: {...} }`
10. `GET /api/v1/resolve/abc321_a` 回傳 `{ source: "atcoder", id: "abc321_a", problem: {...} }`
11. `GET /api/v1/resolve/https://leetcode.com/problems/two-sum` 正確解析 URL 並回傳題目
12. Admin HTML 頁面可正常瀏覽題目清單、新增/編輯/刪除題目
13. Admin 可建立/撤銷 API token
14. Admin 可觸發爬蟲並查看執行結果；重複觸發回傳 409
15. 未帶 token 或 token 無效時回傳 401
16. Rust 端啟動時自動驗證 sqlite-vec 載入及向量維度一致性
17. `GET /health` 回傳 DB 連線狀態與 sqlite-vec 載入狀態
18. `GET /api/v1/problems/{source}?per_page=200` 回傳最多 100 筆（上限截斷）

## Deployment

### Docker

```dockerfile
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y python3 python3-pip && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/oj-api-rs /usr/local/bin/
COPY --from=builder /app/templates /app/templates
COPY --from=builder /app/static /app/static
COPY --from=builder /app/references /app/references
WORKDIR /app
EXPOSE 3000
CMD ["oj-api-rs"]
```

- 需包含 Python runtime（`/api/v1/similar?query=` 及爬蟲觸發需要）
- `data/data.db` 以 volume mount 掛載，不打包進 image
- 環境變數：`ADMIN_SECRET`、`GEMINI_API_KEY`、`DATABASE_PATH`、`LISTEN_ADDR`

### Logging

- 使用 `tracing` + `tracing-subscriber` 做 structured logging
- 預設 level: `info`，可透過 `RUST_LOG` 環境變數調整
- Python 子程序的 stderr 以 `warn` level 記錄

## Prerequisites (Pre-implementation)

在開始 Rust 實作前，需完成以下前置任務：

1. **Python 端新增 `--embed-text` CLI**：在 `embedding_cli.py` 新增旗標，封裝 `EmbeddingRewriter.rewrite()` + `EmbeddingGenerator.embed()`，stdout 輸出 `{"embedding": [...], "rewritten": "..."}`
2. **`daily_challenge.id` 型別統一**：將 Python 端寫入 `daily_challenge` 時的 `id` 欄位改為 TEXT，並提供 migration script
