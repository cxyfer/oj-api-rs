# Tasks: crawler-output-and-args-rework

## T1: 重構 CrawlerSource + ArgSpec + validate_args (src/models.rs)

### T1.1: 新增 ArgSpec / ValueType / CrawlerSource
- [x] 在 `src/models.rs` 定義 `ArgSpec`, `ValueType`, `CrawlerSource` enum
- [x] `CrawlerSource` impl: `parse()`, `script_name()`, `arg_specs()`
- [x] 定義三組常數表：`LEETCODE_ARGS`, `ATCODER_ARGS`, `CODEFORCES_ARGS`

### T1.2: 實作 validate_args
- [x] `pub fn validate_args(source: &CrawlerSource, raw_args: &[String]) -> Result<Vec<String>, String>`
- [x] 逐 token 走訪：查白名單 → 消耗 arity 個 value → 驗證 value_type → 重複檢查
- [x] 孤立 value / 未知 flag / 值缺失 / 型別錯誤 → Err 含明確訊息
- [x] `--data-dir` / `--db-path`：額外驗證相對路徑、不含 `..`

### T1.3: 移除 CrawlerAction enum
- [x] 刪除 `CrawlerAction` enum 及 `parse()`, `to_args()` impl
- [x] 所有引用點改用 `CrawlerSource::parse()` + `validate_args()`

### T1.4: CrawlerJob 擴充
- [x] 新增 `stdout: Option<String>` + `stderr: Option<String>`
- [x] `#[serde(skip_serializing_if = "Option::is_none")]` 避免 status API 傳輸
- [x] 新增 helper: `fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>)` — 截斷至 64KB + lossy decode

**驗收**：`cargo check` 通過，所有引用 CrawlerAction 的編譯錯誤已修復

---

## T2: Admin handler 保存輸出 + 新增 output API (src/admin/handlers.rs)

### T2.1: trigger_crawler 保存輸出
- [x] `wait_with_output()` 完成後：
  1. `tokio::fs::create_dir_all("scripts/logs")` 確保目錄存在
  2. 非空 stdout → `tokio::fs::write(format!("scripts/logs/{}.stdout.log", job_id), &output.stdout)`
  3. 非空 stderr → 同上 `.stderr.log`
  4. 寫入失敗 `tracing::warn!`，不影響 job 狀態
  5. `job.set_output(output.stdout, output.stderr)` 截斷存入記憶體

### T2.2: trigger_crawler 改用 CrawlerSource
- [x] 將 `valid_sources` 字串白名單改為 `CrawlerSource::parse(&body.source)`
- [x] 將 `CrawlerAction::parse(&body.args)` 改為 `validate_args(&source, &body.args)`
- [x] `script` 改為 `source.script_name()`

### T2.3: status API 排除輸出
- [x] `crawler_status` 回傳 history 時 map 每筆 job，將 stdout/stderr 設為 None

### T2.4: 新增 output API
- [x] `GET /admin/api/crawlers/{job_id}/output`
- [x] Handler: `crawler_output(Path(job_id), State(state))`
- [x] UUID 格式驗證防止 path traversal
- [x] 查詢 in-memory history → 找到即回傳（含空輸出）
- [x] 未找到 → 嘗試讀取 `scripts/logs/{job_id}.stdout.log` + `.stderr.log`
- [x] 都沒有 → 404
- [x] 路由註冊在 admin router

**驗收**：`cargo check` 通過，`/admin/api/crawlers/trigger` 執行後 `scripts/logs/` 有檔案產生

---

## T3: Daily fallback 保存輸出 (src/api/daily.rs)

### T3.1: 產生 job_id
- [x] 在 spawn 前 `let job_id = uuid::Uuid::new_v4().to_string()`

### T3.2: 保存輸出至檔案
- [x] `wait_with_output()` 完成後：
  1. 建立 `scripts/logs/` 目錄
  2. 非空 stdout/stderr → 寫入 `scripts/logs/{job_id}.*.log`
  3. 失敗 `tracing::warn!`

### T3.3: tracing 摘要
- [x] 使用 `.chars().take(500)` 安全截取避免 UTF-8 char boundary panic

**驗收**：觸發 daily fallback 後，`scripts/logs/` 有對應檔案，Rust log 含摘要

---

## T4: Admin UI 動態參數 + Modal (templates/ + static/)

### T4.1: CRAWLER_CONFIG 物件
- [x] 在 `admin.js` 定義 `CRAWLER_CONFIG` 物件
- [x] 各 source 的 flags 陣列（排除 `--data-dir`, `--db-path`）
- [x] 每項：`{ flag, label, type, placeholder?, step? }`

### T4.2: 動態渲染邏輯
- [x] 新增 `renderArgs(source)` 函式，清空 `#crawler-args-options` 並根據 config 渲染
- [x] Checkbox + 對應 input（disabled 直到勾選）
- [x] `--monthly` 特殊處理：year + month 分開

### T4.3: getArgs() 重寫
- [x] 取代原 `getSelectedAction()`
- [x] 遍歷已勾選 checkbox，收集 flag + value
- [x] 帶值 flag 的 value 為空 → toast 提示 + return null

### T4.4: crawlers.html 模板修改
- [x] 移除固定的 radio buttons 區塊
- [x] 改為 `<div id="crawler-args-options"></div>` 動態容器
- [x] 新增 Modal HTML 結構（hidden by default）

### T4.5: Output Modal
- [x] History 表格新增 "Logs" 欄位
- [x] Completed/Failed/TimedOut 行顯示 "View" 按鈕（data-job-id 屬性）
- [x] 點擊 → fetch `/admin/api/crawlers/{job_id}/output` → 填入 Modal `<pre>`
- [x] stdout tab + stderr tab
- [x] stderr 紅色字體（`.log-stderr { color: #ff6b6b; }`）
- [x] 關閉按鈕 / ESC 關閉

### T4.6: CSS 更新
- [x] checkbox grid 樣式（args-grid）
- [x] flag-item 佈局（checkbox + label + input inline）
- [x] Modal 樣式（backdrop + centered content + tabs）

### T4.7: XSS 防護（Review 修復）
- [x] 新增 `esc()` HTML escape 函式
- [x] `updateHistoryTable` 所有動態內容使用 `esc()` 過濾

**驗收**：切換 source 顯示對應 flags，勾選組合觸發成功，Modal 顯示輸出

---

## T5: 收尾

### T5.1: .gitignore
- [x] 新增 `scripts/logs/` 至 `.gitignore`

### T5.2: 編譯驗證
- [x] `cargo build` 通過

### T5.3: 手動驗證
- [ ] LeetCode `--daily` 從 admin 觸發 → 完成 → View 按鈕 → Modal 顯示輸出
- [ ] AtCoder `--fetch-all --resume` 組合觸發 → 通過驗證
- [ ] Codeforces `--contest 1234` 觸發 → 通過驗證
- [ ] Daily fallback 自動觸發 → `scripts/logs/` 有檔案
- [ ] 不合法參數 → 400 錯誤訊息

## Dependency Graph

```
T1 → T2 → T5
T1 → T3 → T5
T1 → T4 → T5
```

T1 先行（模型層），T2/T3/T4 可並行，T5 最後收尾。
