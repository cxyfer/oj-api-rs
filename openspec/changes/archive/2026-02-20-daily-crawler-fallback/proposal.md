# Change: daily-crawler-fallback

## Context

Rust 後端的 `/daily` API 端點（`src/api/daily.rs`）僅查詢 DB，找不到資料時直接回傳 404。缺少自動觸發 Python 爬蟲抓取的 fallback 機制。

Admin crawler trigger（`src/admin/handlers.rs:244-246`）呼叫 `python3 leetcode.py` 時不帶任何 CLI 參數，導致腳本直接結束不做任何操作。

Python 爬蟲目前放在 `references/leetcode-daily-discord-bot/`（外部 git clone），應搬移至 `scripts/` 納入專案自身版控管理。腳本具備完整 argparse 介面（`--daily`、`--date`、`--init`、`--monthly` 等），直接寫入共用 `data/data.db`。

Admin 後台 GUI 目前僅有 Dashboard、Problems、Tokens 三個頁面，缺少 Crawlers 管理介面。API 端點已就緒但無對應前端。

## Requirements

### R1: /daily API 異步爬蟲 fallback

`GET /api/v1/daily` 查詢 DB 無資料時，SHALL 自動觸發 Python 爬蟲作為背景任務，並回傳 HTTP 202 (Accepted) 通知客戶端稍後重試。

#### Scenario: 今日無資料 → 異步觸發
- **WHEN** client 查詢 `/api/v1/daily?domain=com`（today）且 DB 無資料
- **THEN** 系統觸發背景 `python3 leetcode.py --daily` 任務
- **AND** 回傳 HTTP 202 `{"status": "fetching", "retry_after": 30}`

#### Scenario: 歷史日期無資料 → 異步觸發
- **WHEN** client 查詢 `/api/v1/daily?domain=com&date=2024-06-15` 且 DB 無資料
- **THEN** 系統觸發背景 `python3 leetcode.py --date 2024-06-15` 任務
- **AND** 回傳 HTTP 202 `{"status": "fetching", "retry_after": 30}`

#### Scenario: 已有爬蟲在執行中 → 不重複觸發
- **WHEN** DB 無資料但已有爬蟲任務在執行中
- **THEN** 系統不重複觸發爬蟲
- **AND** 回傳 HTTP 202 `{"status": "fetching", "retry_after": 30}`

#### Scenario: 爬蟲完成後重試成功
- **WHEN** client 在爬蟲完成後重新查詢相同日期
- **THEN** DB 已有資料，回傳 HTTP 200 + DailyChallenge JSON

### R2: Admin Crawler Trigger 支援 CLI 參數

`POST /admin/api/crawlers/trigger` SHALL 接受 `args` 欄位，作為傳遞給 Python 腳本的 CLI 參數。

#### Scenario: 帶 --daily 參數
- **WHEN** client 發送 `{"source": "leetcode", "args": ["--daily"]}`
- **THEN** 系統執行 `python3 leetcode.py --daily`

#### Scenario: 帶 --date 參數
- **WHEN** client 發送 `{"source": "leetcode", "args": ["--date", "2024-06-15"]}`
- **THEN** 系統執行 `python3 leetcode.py --date 2024-06-15`

#### Scenario: 帶 --init 參數
- **WHEN** client 發送 `{"source": "leetcode", "args": ["--init"]}`
- **THEN** 系統執行 `python3 leetcode.py --init`

#### Scenario: 不帶 args 欄位（向下相容）
- **WHEN** client 發送 `{"source": "leetcode"}` 不含 `args`
- **THEN** 系統執行 `python3 leetcode.py`（與現有行為相同）

#### Scenario: 參數白名單驗證
- **WHEN** client 傳入不允許的參數（如 `--malicious`）
- **THEN** 系統回傳 HTTP 400 拒絕

### R3: Python 腳本搬移至 scripts/

爬蟲腳本 SHALL 從 `references/leetcode-daily-discord-bot/` 搬移至 `scripts/`，僅包含 Rust 後端實際呼叫的腳本及其依賴模組。

#### Scenario: 搬移檔案清單
- **WHEN** 搬移完成
- **THEN** `scripts/` 包含以下結構：
  ```
  scripts/
  ├── leetcode.py
  ├── atcoder.py
  ├── codeforces.py
  ├── embedding_cli.py
  ├── requirements.txt（僅含爬蟲 + embedding 所需依賴）
  ├── utils/
  │   ├── __init__.py
  │   ├── config.py
  │   ├── database.py
  │   ├── html_converter.py
  │   └── logger.py
  └── embeddings/
      ├── __init__.py
      ├── generator.py
      ├── rewriter.py
      ├── searcher.py
      └── storage.py
  ```
- **AND** `references/` 目錄可安全移除

#### Scenario: Rust 後端路徑更新
- **WHEN** 腳本搬移完成
- **THEN** `src/admin/handlers.rs:246` 的 `cmd.current_dir()` 從 `references/leetcode-daily-discord-bot/` 改為 `scripts/`
- **AND** `src/api/similar.rs:143` 的 `cmd.current_dir()` 同步更新為 `scripts/`
- **AND** 所有爬蟲呼叫路徑一致指向 `scripts/`

### R4: Admin Crawlers GUI 頁面

Admin 後台 SHALL 新增 Crawlers 管理頁面（`/admin/crawlers`），提供爬蟲觸發、參數選擇及歷史任務記錄。

#### Scenario: 導覽列顯示 Crawlers 入口
- **WHEN** 使用者登入 admin 後台
- **THEN** 導覽列顯示 Crawlers 連結（位於 Problems 和 Tokens 之間）

#### Scenario: 觸發爬蟲帶 CLI 參數
- **WHEN** 使用者在 Crawlers 頁面選擇 source（leetcode/atcoder/codeforces）並選擇操作（--daily/--date/--init/--monthly）
- **THEN** 前端呼叫 `POST /admin/api/crawlers/trigger` 帶 `args` 欄位
- **AND** 頁面顯示觸發成功提示

#### Scenario: 即時顯示爬蟲狀態
- **WHEN** 有爬蟲任務執行中
- **THEN** 頁面透過輪詢 `GET /admin/api/crawlers/status` 即時顯示狀態（Running/Completed/Failed/TimedOut）
- **AND** 顯示 job_id、source、started_at 等資訊

#### Scenario: 爬蟲歷史記錄
- **WHEN** 使用者瀏覽 Crawlers 頁面
- **THEN** 顯示最近一次爬蟲任務的結果（status、source、started_at）

### R5: /daily fallback 不復用 crawler_lock

`/daily` 的自動 fallback 爬蟲 SHALL 使用獨立的鎖機制（與 admin crawler_lock 分離），避免 admin 手動觸發的長時間爬蟲影響 `/daily` 的即時抓取需求。

#### Scenario: admin 爬蟲執行中不阻擋 /daily fallback
- **WHEN** admin 正在執行 `--init` 爬蟲（長時間）
- **AND** client 查詢 `/daily` 觸發 fallback
- **THEN** /daily fallback 仍可正常觸發自己的 `--daily` 爬蟲任務

## Success Criteria

1. `GET /api/v1/daily` 查詢 DB 無資料時回傳 HTTP 202 並自動觸發爬蟲
2. 爬蟲完成後，相同查詢回傳 HTTP 200 + 完整 DailyChallenge JSON
3. `POST /admin/api/crawlers/trigger` 支援 `args` 欄位傳遞 CLI 參數
4. Python 爬蟲透過 argparse 正確接收並執行對應操作
5. Python 和 Rust 共用 `data/data.db`，爬蟲寫入的資料可被 Rust 即時讀取
6. Python 腳本位於 `scripts/`，`references/` 不再被後端引用
7. Admin GUI 可從 Crawlers 頁面觸發爬蟲（含 CLI 參數選擇）並查看狀態和歷史

## Constraints

- Python 爬蟲腳本搬移至 `scripts/` 後不做功能修改，僅作路徑遷移
- 共用 SQLite DB，Rust 透過 r2d2 connection pool 讀取
- fallback 爬蟲超時設定復用 `CRAWLER_TIMEOUT_SECS` 環境變數
- admin crawler trigger 的 `args` 使用語法白名單驗證（解析為 enum action 再轉 argv）
- `/daily` fallback 使用獨立鎖機制（`HashMap<DailyKey, RunningJob>`），不與 admin crawler_lock 衝突
- R1 fallback 僅支援 `domain=com`，`domain=cn` 無資料時直接回 404
- Crawlers GUI 頁面沿用現有 admin 深色主題風格（Askama template + 原生 JS）
- 爬蟲歷史記錄使用 in-memory VecDeque（上限 50 筆），不新增 DB 表
- R1 fallback spawn 失敗時回 HTTP 500，不回 202；已失敗的 key 設 30s cooldown 防止風暴

## Affected Modules

- `scripts/` — 新目錄，從 references 搬入爬蟲腳本、embedding 和依賴
- `src/api/daily.rs` — 新增 fallback 邏輯（僅 domain=com）
- `src/api/similar.rs` — 更新 current_dir 路徑
- `src/admin/handlers.rs` — TriggerCrawlerRequest 新增 args 欄位，更新 current_dir 路徑，status API 回傳 history
- `src/admin/pages.rs` — 新增 crawlers_page handler
- `src/admin/mod.rs` — 新增 `/admin/crawlers` 路由
- `src/main.rs` — AppState 新增 daily fallback 鎖 + crawler history ring buffer
- `src/models.rs` — CrawlerJob 擴充 args/trigger/finished_at 欄位
- `templates/admin/crawlers.html` — 新增 Crawlers 頁面模板
- `templates/base.html` — 導覽列新增 Crawlers 連結
- `static/admin.js` — 新增 Crawlers 頁面前端邏輯（輪詢、觸發、歷史顯示）
- `static/admin.css` — 新增 Crawlers 頁面樣式
