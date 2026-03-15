## Context

目前 Rust 後端對 crawler、embedding 與 daily fallback 的 subprocess 仍採用 `wait_with_output()`，等子行程結束後才把 `stdout` / `stderr` 寫成扁平檔 `scripts/logs/{job_id}.stdout.log`、`{job_id}.stderr.log`，對應邏輯分散在 `src/admin/handlers.rs` 與 `src/api/daily.rs`。這種模式有三個直接問題：

1. admin 介面無法可靠顯示執行中的 live log，只能在完成後查看有限輸出。
2. Python logger 目前只寫 root date log，且 console/file formatter 行為與 per-job output 分離，導致單次任務的 logger output 無法成為一級 artifact。
3. `scripts/logs/` 混放 job artifacts 與日期型 Python 日誌，runtime artifacts 難以辨識，也讓後續清理與查詢規則不一致。

這次變更需要同時覆蓋 Rust subprocess capture、Python logger、embedding progress、crawler admin UI、daily fallback history 與 runtime log retention，因此屬於跨模組的設計型變更。

## Goals / Non-Goals

**Goals:**
- 將所有 Rust 啟動的 job artifacts 統一到 `scripts/logs/{job_type}/{job_id}/`。
- 為每個 job 固定產出 `stdout.log`、`stderr.log`、`python.log`、`progress.json`。
- 保留既有 admin API split：crawler 走 `/admin/api/crawlers/*`，embedding 走 `/admin/api/embeddings/*`。
- 讓 crawler、embedding 與 daily fallback 都能在 admin UI 中查看執行中的 live log。
- 保留 `scripts/logs/YYYY-MM-DD.log` 這類 root date log 的 ANSI 顏色輸出。
- 讓 per-job `python.log` 與 admin 顯示內容穩定為純文字，不含 ANSI escape codes。
- 讓 daily fallback 採用相同 job artifact 佈局，並出現在 crawler history 同一張表中。
- 定義 7 天 retention，自動清理過舊的 per-job artifact 目錄。

**Non-Goals:**
- 不新增統一的 `/admin/api/jobs/*` 介面。
- 不導入 SSE、WebSocket 或資料庫型 job index。
- 不保留舊扁平 log 檔讀取相容性。
- 不在本次實作所有 crawler 的細粒度數值 progress；crawler v1 僅要求 phase-level。
- 不在本次清理舊扁平檔或 root date log。

## Decisions

### 1. 採用固定的 per-job artifact directory layout

**Decision**
- 所有 Rust 啟動的 job 一律寫入 `scripts/logs/{job_type}/{job_id}/`。
- `job_type` 採封閉集合：`crawler`、`embedding`。
- daily fallback 在 artifact taxonomy 中歸類為 `crawler`，但在 history / status 以 `trigger=daily_fallback` 區分。

**Rationale**
- 固定路徑比沿用 `logging.directory` 更容易被 Rust admin API、前端 polling 與 retention cleanup 共用。
- 將 daily fallback 併入 `crawler` job_type，可避免第三套 artifact namespace，並直接共用 crawler admin surface。

**Alternatives considered**
- **沿用 `logging.directory`**：保留配置彈性，但 Rust 端必須理解 Python logger 配置，增加 lookup 與 cleanup 複雜度。
- **為 daily fallback 建立獨立 job_type**：語義更細，但 admin route split 與 history 呈現會變得更複雜，收益不足。

### 2. 保留 root date log，另外新增 per-job plain-text logger artifact

**Decision**
- `scripts/logs/YYYY-MM-DD.log` 仍維持現有 root logger date file，保留 ANSI 色碼。
- Python logger 若偵測到 job env（例如 `OJ_JOB_DIR`、`OJ_PYTHON_LOG_PATH`），額外掛載 per-job file handler，輸出到 `python.log`。
- `python.log` 必須是 ANSI-clean 純文字，且不得與 `stderr.log` 合併。

**Rationale**
- root date log 是跨任務的營運日誌，保留彩色輸出方便終端與既有維運習慣。
- per-job `python.log` 是 admin UI 與 job artifact 的主要 logger channel，需要穩定可讀、可輪詢、可比較，因此必須純文字。
- 將 logger output 與 raw stderr 分離，可避免 traceback / subprocess stderr 與應用層 logger 混在一起。

**Alternatives considered**
- **把 logger output 併入 `stderr.log`**：少一個檔案，但會混淆 stderr 與 application log，降低 debug 可讀性。
- **在讀取時才做 ANSI stripping**：可保留原始 artifact，但 Rust/前端每次輪詢都需重複清理與判斷，且 per-job artifact 本身不再是穩定輸出。

### 3. Rust 端改為 live tee capture，而非 wait-then-write

**Decision**
- `trigger_crawler`、`trigger_embedding`、daily fallback 都在啟動子行程前先建立 job dir。
- Rust 注入標準 env：`OJ_JOB_ID`、`OJ_JOB_TYPE`、`OJ_JOB_DIR`、`OJ_PROGRESS_PATH`、`OJ_PYTHON_LOG_PATH`。
- 子行程的 `stdout` / `stderr` 以並行 reader 即時追加到 `stdout.log` / `stderr.log`，同時維持 bounded in-memory tail 供 status/output endpoint 使用。
- job 結束時只做 terminal status 與 final metadata 更新，不再以 `wait_with_output()` 作為主要輸出來源。

**Rationale**
- live polling 的前提是 artifact 在執行期間就持續可讀。
- 並行 tee capture 能避免某一側 pipe 堵塞，且可保持檔案內容與 API payload 單調成長。

**Alternatives considered**
- **維持 `wait_with_output()` 再落檔**：實作最簡單，但完全無法達成 live log 需求。
- **改用 SSE/串流 API**：體驗更即時，但前後端改動遠高於需求所需，且現有 admin 已有 polling 基礎。

### 4. 保留既有 admin API split，只擴充 payload 與少量端點

**Decision**
- 繼續使用既有路由：
  - crawler：`/admin/api/crawlers/status`、`/admin/api/crawlers/{job_id}/output`、新增 `/admin/api/crawlers/{job_id}/progress`
  - embedding：保留 `/admin/api/embeddings/status`、`/admin/api/embeddings/{job_id}/output`、`/admin/api/embeddings/{job_id}/progress`
- output payload 至少回傳 `stdout`、`stderr`、`python_log`；running job 與 completed job 都可讀。
- crawler output endpoint 可讀 admin crawler job 與 daily fallback job；embedding endpoint 僅讀 embedding job。

**Rationale**
- 既有頁面與 JS 已按 crawler / embedding 拆分，保留 split 能把變更集中在 payload 擴充，不必重寫整個 admin model。
- 讓 running job 也能讀 output，可直接重用現有 modal + polling 模式。

**Alternatives considered**
- **新建 `/admin/api/jobs/*`**：長期看較統一，但本次屬 breaking runtime layout 變更，再疊加 API reshape 風險過高。

### 5. progress schema 採分流策略

**Decision**
- crawler `progress.json` v1 僅保證 phase-level schema，使用封閉 phase enum：`queued`、`running`、`completed`、`failed`、`cancelled`、`timed_out`。
- crawler progress 可附帶 `message`、`updated_at`，但不得引入 embedding 專用的 `rewrite_progress` / `embed_progress` 作為必填欄位。
- embedding 維持既有詳細 schema：`phase`、`rewrite_progress`、`embed_progress`，並在 terminal state 更新最終 phase。

**Rationale**
- embedding 已有成熟的詳細進度模型；crawler 各腳本現況差異大，先統一 phase 能最小成本收斂契約。
- 封閉 phase enum 可避免後端與前端對 terminal state 解讀不一致。

**Alternatives considered**
- **所有 crawler 一次補齊 done/total**：資訊更完整，但需要同時修改多支 crawler script，超出本次必要範圍。
- **完全不做 crawler progress**：無法支撐 admin 對 running job 的一致顯示與後續擴充。

### 6. Admin UI 延續 polling，擴充 running log 與 `python.log` tab

**Decision**
- 保留現有 3 秒 polling 架構。
- crawler 與 embedding status card 繼續走 `/status`；log modal 在開啟後對 `/output` 做定時抓取，顯示 live `stdout` / `stderr` / `python.log`。
- crawler history table 與 daily fallback 共用同一張表，透過 `trigger` 區分來源。
- running job 也允許開啟 log modal，不再限制只有 completed 才能 View。

**Rationale**
- 現有 `static/admin.js` 已經具備 polling 與 progress bar 更新機制，延伸成本最低。
- 多 tab 明確分離 raw stdout、stderr 與 python logger，比單一輸出區塊更可讀。

**Alternatives considered**
- **只在 status card 顯示 live tail，不用 modal**：實作較省，但無法滿足完整檢視需求。
- **改用 SSE**：需要新增 server push、前端 reconnect 與 ordering 處理，對本次不划算。

### 7. retention 以「刪整個 job directory」為單位，保留 7 天

**Decision**
- cleanup target 僅限 `scripts/logs/{job_type}/{job_id}/` 這類 per-job 目錄。
- 刪除條件以 job directory 的最後更新時間為準；超過 7 天才可刪除。
- root date log `scripts/logs/YYYY-MM-DD.log`、舊扁平檔都不在本次 cleanup 範圍。
- 執行中 job 的 artifact directory 永不可被 cleanup，即使 mtime 異常也要跳過。
- cleanup 可在啟動時執行一次，並在新 job 啟動前再做一次輕量檢查。

**Rationale**
- 以目錄為單位最符合 artifact bundle 概念，也避免留下半套檔案。
- 7 天可兼顧除錯需求與磁碟控制。

**Alternatives considered**
- **用筆數上限清理**：實作需先列舉與排序每類 job，對 fallback/shared history 更麻煩。
- **順便清理舊扁平檔**：不屬於本次需求，且有破壞性風險。

## Risks / Trade-offs

- **[Live tee capture 引入更多 async I/O 複雜度] → Mitigation:** 將 job artifact path、tail cache、pipe reader 封裝到單一 helper，避免 crawler / embedding / fallback 各自實作。
- **[running job 與 cancel/timeout 競態導致 terminal phase 被覆寫] → Mitigation:** 在單一 finalize 路徑中做 idempotent terminal transition，terminal 後拒絕非 terminal 更新。
- **[per-job sanitization 與 root ANSI log 共存時容易混淆] → Mitigation:** 明確把 root date log 與 per-job handler 分開，per-job handler 一律使用 plain-text formatter。
- **[daily fallback 納入 crawler history 後，UI 排序與顯示資訊可能變複雜] → Mitigation:** 保持同表，但固定顯示 `trigger` 欄位，讓 admin 明確辨識 `admin` 與 `daily_fallback`。
- **[retention cleanup 誤刪執行中目錄] → Mitigation:** cleanup 前先查 active crawler / embedding job 與 in-memory running state，命中即跳過。
- **[輪詢 full file 造成大 log 傳輸浪費] → Mitigation:** output endpoint 預留 tail/offset 實作空間；首版先保證正確性，若實測需要再補 bounded reads。
- **[breaking layout 造成舊 log 在新版 UI 不可讀] → Mitigation:** 在 migration plan 與 release note 明確標示不相容；舊檔保留但不再由新 API 使用。

## Migration Plan

1. 先新增 Rust job artifact helper 與 Python per-job logger helper，不改動對外頁面行為。
2. 將 crawler、embedding、daily fallback 三條執行路徑切到新 artifact layout 與 live tee capture。
3. 更新 admin API，讓 output/progress 從新 job dir 讀取，並將 daily fallback 納入 crawler history/status。
4. 更新 admin UI，加入 running log modal、`python.log` tab 與 crawler progress 顯示。
5. 啟用 retention cleanup（7 天）於啟動時與新 job 啟動前執行。
6. 部署後的新 job 全部採新結構；舊扁平檔保留在磁碟上，但新版 API 不再讀取。

**Rollback strategy**
- 若需回滾，直接回退到舊版程式碼即可；舊版會重新產生扁平檔。
- 回滾後，新版產生的 per-job 目錄會留在 `scripts/logs/{job_type}/{job_id}/`，但舊版不會讀取它們；這些目錄可由維運後續手動清理。

## Open Questions

無。此 design 已將 implementation 所需決策固定化，後續應可直接機械式實作。