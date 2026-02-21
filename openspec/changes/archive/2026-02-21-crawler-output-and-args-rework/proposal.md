# Change: crawler-output-and-args-rework

## Context

先前 `2026-02-20-daily-crawler-fallback` change 已將爬蟲管理系統上線，但存在三個問題：

1. **爬蟲輸出不可見**：`child.wait_with_output()` 捕獲了 stdout/stderr，但結果被丟棄。Admin 頁面僅能看到狀態（Completed/Failed），無法知道爬蟲實際做了什麼。
2. **動作白名單僅適用 LeetCode**：`CrawlerAction` enum 只支援 `--daily`、`--date`、`--init`、`--monthly`，而 AtCoder/Codeforces 各有不同的 CLI 參數集。目前從 admin 對 AtCoder/Codeforces 觸發時只能用 LeetCode 的四個動作。
3. **每次只能傳一組參數**：`CrawlerAction::parse` 僅解析第一個 flag，無法支援如 `--fetch-all --resume` 的組合。

## Requirements

### R1: 保存爬蟲輸出（stdout + stderr）

`wait_with_output()` 取得的 stdout/stderr SHALL 被保存，且可透過 admin API 查閱。

#### Scenario: 輸出保存至 CrawlerJob
- **WHEN** 爬蟲執行完畢（無論成功或失敗）
- **THEN** `CrawlerJob` 包含 `stdout` 和 `stderr` 欄位（`String`，UTF-8 lossy decode）
- **AND** 在 in-memory history 中可透過 status API 取得

#### Scenario: 輸出截斷
- **WHEN** stdout 或 stderr 超過 64KB
- **THEN** 保留最後 64KB（截斷前段），避免記憶體膨脹

#### Scenario: 輸出寫入檔案
- **WHEN** 爬蟲執行完畢
- **THEN** 完整 stdout 寫入 `scripts/logs/{job_id}.stdout.log`
- **AND** 完整 stderr 寫入 `scripts/logs/{job_id}.stderr.log`
- **AND** 若輸出為空，不建立檔案

#### Scenario: Admin API 回傳輸出
- **WHEN** 查詢 `GET /admin/api/crawlers/status`
- **THEN** history 中每個 job 包含 `stdout` 和 `stderr` 欄位
- **AND** `current_job`（Running 中）的 stdout/stderr 為 null

#### Scenario: Daily fallback 也保存輸出
- **WHEN** `/api/v1/daily` 觸發的 fallback 爬蟲完成
- **THEN** 輸出同樣寫入 `scripts/logs/` 目錄
- **AND** 透過 tracing 記錄輸出摘要（前 500 字元）

### R2: 按腳本差異化 CLI 參數白名單

不同 source 的爬蟲 SHALL 各有獨立的參數白名單，從各腳本實際支援的 argparse 參數推導。

#### Scenario: LeetCode 參數白名單
- **WHEN** `source = "leetcode"`
- **THEN** 支援的 flags：`--init`, `--full`, `--daily`, `--date <YYYY-MM-DD>`, `--monthly <YEAR> <MONTH>`, `--fill-missing-content`, `--fill-missing-content-workers <N>`, `--missing-content-stats`

#### Scenario: AtCoder 參數白名單
- **WHEN** `source = "atcoder"`
- **THEN** 支援的 flags：`--sync-kenkoooo`, `--sync-history`, `--fetch-all`, `--resume`, `--contest <ID>`, `--status`, `--fill-missing-content`, `--missing-content-stats`, `--reprocess-content`, `--rate-limit <FLOAT>`, `--data-dir <DIR>`, `--db-path <PATH>`

#### Scenario: Codeforces 參數白名單
- **WHEN** `source = "codeforces"`
- **THEN** 支援的 flags：`--sync-problemset`, `--fetch-all`, `--resume`, `--contest <INT>`, `--status`, `--fill-missing-content`, `--missing-content-stats`, `--missing-problems`, `--reprocess-content`, `--include-gym`, `--rate-limit <FLOAT>`, `--data-dir <DIR>`, `--db-path <PATH>`

#### Scenario: 不在白名單中的參數
- **WHEN** 傳入任何 source 不支援的參數
- **THEN** 回傳 HTTP 400 明確指出哪個參數無效

### R3: 支援多參數組合

`CrawlerAction` SHALL 改為支援多個 flag 的同時傳入，前端 UI 可動態組合參數。

#### Scenario: 組合式參數
- **WHEN** admin 傳入 `{"source": "atcoder", "args": ["--fetch-all", "--resume"]}`
- **THEN** 系統將 `["--fetch-all", "--resume"]` 直接傳給腳本
- **AND** 參數白名單分別驗證每個 flag

#### Scenario: 含值參數的組合
- **WHEN** admin 傳入 `{"source": "codeforces", "args": ["--fetch-all", "--resume", "--rate-limit", "3.0"]}`
- **THEN** 系統正確解析 `--rate-limit` 需要一個值，且驗證 3.0 為合法 float
- **AND** 組合 args 為 `["--fetch-all", "--resume", "--rate-limit", "3.0"]`

#### Scenario: 衝突參數偵測（不實作，延後）
- 不做衝突偵測，直接將合法參數傳給腳本，腳本自行處理

### R4: Admin Crawlers UI 按 source 動態顯示參數

Admin Crawlers 頁面 SHALL 根據選擇的 source 動態顯示可用的 checkbox/input，取代現有的固定 radio button。

#### Scenario: 切換 source 後更新可用參數
- **WHEN** 使用者選擇 "LeetCode"
- **THEN** 顯示 LeetCode 支援的 flags（checkbox 形式）
- **AND** `--date` 顯示附帶日期輸入框，`--monthly` 附帶年月輸入框

#### Scenario: 切換到 AtCoder
- **WHEN** 使用者選擇 "AtCoder"
- **THEN** 顯示 AtCoder 的 flags（含 `--contest` ID 輸入框、`--rate-limit` 數值輸入框等）

#### Scenario: 組合勾選送出
- **WHEN** 使用者勾選多個 checkbox（如 `--fetch-all` + `--resume`）
- **THEN** 前端組合為 args 陣列送出

## Success Criteria

1. 爬蟲完成後，admin status API 的 history 每個 job 包含 `stdout` 和 `stderr`
2. `scripts/logs/` 目錄下有對應 job_id 的 log 檔案
3. 對 AtCoder 觸發 `--fetch-all --resume` 組合可正常通過白名單驗證並執行
4. 對 Codeforces 觸發 `--contest 1234` 可正常通過白名單驗證並執行
5. LeetCode 的 `--daily` 仍正常運作（向下相容）
6. Admin UI 根據 source 動態切換顯示可用參數，可勾選組合
7. Daily fallback 的輸出也被記錄到 `scripts/logs/`

## Constraints

- `CrawlerAction` enum 重構為 per-source 的參數驗證機制，取代現有的單一 enum
- in-memory 歷史仍限 50 筆，但每筆 stdout/stderr 截斷至 64KB
- 檔案 log 不截斷，寫入完整輸出
- `--data-dir` 和 `--db-path` 參數允許出現在白名單中但不開放前端 UI（僅 API 層面可用），避免安全風險
- 前端不做參數互斥/衝突偵測，交由腳本 argparse 處理
- Daily fallback 流程不變（仍只觸發 leetcode `--daily`/`--date`），本次僅加入輸出保存

## Affected Modules

- `src/models.rs` — `CrawlerJob` 新增 `stdout`/`stderr` 欄位；`CrawlerAction` 重構為 per-source 驗證
- `src/admin/handlers.rs` — `wait_with_output()` 後保存輸出至 job 和檔案；status API 回傳輸出
- `src/api/daily.rs` — fallback 完成後保存輸出至檔案，tracing 記錄摘要
- `templates/admin/crawlers.html` — 按 source 動態顯示參數 checkbox/input
- `static/admin.js` — 動態參數 UI 邏輯、組合 args 陣列送出
- `static/admin.css` — 新參數 UI 樣式（如有需要）
