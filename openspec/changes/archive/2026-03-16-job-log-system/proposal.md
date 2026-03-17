## Why

目前 job 產生的 `stdout`、`stderr`、`progress.json` 與 Python 每日日誌共用 `scripts/logs/` 根目錄，讓 runtime artifacts 難以辨識，也讓後續維運與除錯變得混亂。Rust 後台目前只能在 job 結束後查看有限的 `stdout` / `stderr`，Python logger 輸出不是缺席就是混在 `stderr` 中，且還夾帶 ANSI 色碼，導致管理介面無法可靠呈現執行中的真實日誌。

## What Changes

- 建立統一的 job-scoped log artifact 規約，將任務輸出改為寫入 `scripts/logs/{job_type}/{job_id}/`，並保留 `scripts/logs/` 根目錄只放 Python 日期型日誌。
- 將 Rust 啟動的 Python subprocess 輸出與 Python logger 輸出都提升為一級 job artifacts，讓 crawler、embedding 與納入範圍的背景任務都能以一致方式檢索。
- 擴充 admin crawler / embedding 的 API 與頁面，使後台可以查看執行中的即時日誌，而不再只在完成後顯示 `stdout` / `stderr`。
- 正規化 Python 日誌的後台展示內容，移除或消化 ANSI escape codes，確保 UI 顯示為穩定、可讀的純文字日誌。
- **BREAKING**：捨棄目前扁平的 `scripts/logs/{job_id}.stdout.log`、`{job_id}.stderr.log`、`{job_id}.progress.json` 佈局，不保留舊檔案相容邏輯。

## Capabilities

### New Capabilities
- `job-log-management`: 定義統一的 per-job log artifacts、即時輸出捕捉、Python job log 展示，以及 crawler / embedding / 背景任務的日誌歸檔規約。

### Modified Capabilities
- `admin-management`: 調整 crawler 管理需求，讓 admin 後台能以新的 job artifact 規約讀取與展示 crawler 執行中的日誌，而不再依賴扁平 log 檔與僅完成後可見的 `stdout` / `stderr`。
- `admin-embedding-page`: 調整 embedding 頁面的進度與輸出需求，讓 progress、Python logger、stdout、stderr 都遵循統一 job log system，並可在 admin UI 中即時查看。

## Impact

- Rust backend：`src/admin/handlers.rs`、`src/api/daily.rs`、`src/models.rs`、`src/main.rs`
- Python scripts：`scripts/utils/logger.py`、`scripts/embedding_cli.py`，以及所有由 Rust 啟動的 crawler / embedding scripts
- Admin frontend：`templates/admin/crawlers.html`、`templates/admin/embeddings.html`、`static/admin.js`
- Runtime artifact layout：`scripts/logs/` 目錄結構與 job output API 契約
