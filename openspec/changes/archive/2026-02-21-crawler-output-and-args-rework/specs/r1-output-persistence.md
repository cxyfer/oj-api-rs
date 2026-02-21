# Spec: R1 — 爬蟲輸出持久化

## Requirements

### R1.1: CrawlerJob 新增輸出欄位
- `CrawlerJob` 新增 `stdout: Option<String>` + `stderr: Option<String>`
- Running 狀態時為 `None`
- 完成後填入截斷至 64KB（取尾部）的輸出

### R1.2: 輸出寫入檔案
- 路徑：`scripts/logs/{job_id}.stdout.log` + `scripts/logs/{job_id}.stderr.log`
- 空輸出不建立檔案
- 使用 `tokio::fs::write` 非同步寫入
- 寫入失敗僅 `tracing::warn!`，不影響 job 狀態

### R1.3: 輸出 API
- 新增 `GET /admin/api/crawlers/{job_id}/output`
- 回傳 `{"stdout": "...", "stderr": "..."}`
- 查詢順序：in-memory history → 檔案讀取
- 找不到回傳 404

### R1.4: Status API 不含輸出
- `GET /admin/api/crawlers/status` 的 history 項目不含 stdout/stderr
- `current_job` 亦不含

### R1.5: Daily fallback 輸出
- 產生 UUID job_id
- 輸出寫入 `scripts/logs/{job_id}.*.log`
- `tracing::info!` 記錄前 500 字元 + job_id

## PBT Properties

### P1.1: 截斷不變量
- **INVARIANT**: `job.stdout.map(|s| s.len()) <= Some(65536)` 恆成立
- **Falsification**: 產生 >64KB 的 stdout，驗證截斷後長度 ≤ 64KB 且為原始輸出尾部

### P1.2: 檔案完整性
- **INVARIANT**: 若 output.stdout 非空，`scripts/logs/{job_id}.stdout.log` 內容 == 完整 output.stdout
- **Falsification**: 比對 wait_with_output() 原始輸出與檔案內容

### P1.3: API 查詢一致性
- **INVARIANT**: `/crawlers/{job_id}/output` 回傳的 stdout 與 in-memory 或檔案一致
- **Falsification**: 觸發爬蟲 → 等待完成 → 比對 API 回傳與檔案內容

### P1.4: 空輸出不建檔
- **INVARIANT**: 若 stdout 為空，則 `{job_id}.stdout.log` 不存在
- **Falsification**: 觸發無輸出的命令 → 驗證檔案不存在
