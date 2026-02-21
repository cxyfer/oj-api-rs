# Design: crawler-output-and-args-rework

## Architecture Decisions

### AD1: 爬蟲輸出持久化策略
- **方案**：維持 `wait_with_output()`，完成後同時寫入檔案和記憶體
- **檔案**：`scripts/logs/{job_id}.stdout.log` + `scripts/logs/{job_id}.stderr.log`（完整輸出，空則不建立）
- **記憶體**：`CrawlerJob` 新增 `stdout: Option<String>` + `stderr: Option<String>`，截斷至最後 64KB
- **UTF-8**：`String::from_utf8_lossy()` 處理非法 bytes

### AD2: 輸出 API 分離
- `/admin/api/crawlers/status` 回傳 history 時**不含** stdout/stderr（避免輪詢 payload 膨脹）
- 新增 `GET /admin/api/crawlers/{job_id}/output` 按需取得單筆 job 的完整 stdout/stderr
- 查詢邏輯：先查 in-memory history，若無則嘗試讀取 `scripts/logs/{job_id}.*.log` 檔案

### AD3: Per-source 參數白名單（Data-driven ArgSpec）

移除 `CrawlerAction` enum，改為 data-driven 常數表。

```rust
struct ArgSpec {
    flag: &'static str,
    arity: u8,          // 0=boolean, 1=takes-one-value, 2=takes-two-values
    value_type: ValueType,
    ui_exposed: bool,
}

enum ValueType {
    None,
    Date,       // YYYY-MM-DD, validated via chrono
    Int,        // positive integer
    Float,      // positive f64
    String,     // non-empty string
    YearMonth,  // arity=2: year(2000-2100) + month(1-12)
}
```

每個 source 有對應的 `&[ArgSpec]` 常數表：

**LeetCode** (ui_exposed=true 除非特別標註)：
| flag | arity | value_type |
|------|-------|------------|
| `--init` | 0 | None |
| `--full` | 0 | None |
| `--daily` | 0 | None |
| `--date` | 1 | Date |
| `--monthly` | 2 | YearMonth |
| `--fill-missing-content` | 0 | None |
| `--fill-missing-content-workers` | 1 | Int |
| `--missing-content-stats` | 0 | None |

**AtCoder**：
| flag | arity | value_type | ui_exposed |
|------|-------|------------|------------|
| `--sync-kenkoooo` | 0 | None | true |
| `--sync-history` | 0 | None | true |
| `--fetch-all` | 0 | None | true |
| `--resume` | 0 | None | true |
| `--contest` | 1 | String | true |
| `--status` | 0 | None | true |
| `--fill-missing-content` | 0 | None | true |
| `--missing-content-stats` | 0 | None | true |
| `--reprocess-content` | 0 | None | true |
| `--rate-limit` | 1 | Float | true |
| `--data-dir` | 1 | String | false |
| `--db-path` | 1 | String | false |

**Codeforces**：
| flag | arity | value_type | ui_exposed |
|------|-------|------------|------------|
| `--sync-problemset` | 0 | None | true |
| `--fetch-all` | 0 | None | true |
| `--resume` | 0 | None | true |
| `--contest` | 1 | Int | true |
| `--status` | 0 | None | true |
| `--fill-missing-content` | 0 | None | true |
| `--missing-content-stats` | 0 | None | true |
| `--missing-problems` | 0 | None | true |
| `--reprocess-content` | 0 | None | true |
| `--include-gym` | 0 | None | true |
| `--rate-limit` | 1 | Float | true |
| `--data-dir` | 1 | String | false |
| `--db-path` | 1 | String | false |

### AD4: 通用參數驗證器

```
validate_args(source: &str, raw_args: &[String]) -> Result<Vec<String>, String>
```

邏輯：
1. 取得 source 對應的 `&[ArgSpec]` 表
2. 逐 token 走訪 `raw_args`：
   - 遇到 `--flag`：在白名單中查找，若不存在回 Err
   - 根據 `arity` 消耗後續 N 個 token 作為 value
   - 根據 `value_type` 驗證每個 value
   - 重複 flag 拒絕（回 Err）
3. 若 token 不以 `--` 開頭且不是前一 flag 的 value → Err（孤立值）
4. 回傳驗證通過的 args（原樣透傳）

### AD5: 路徑安全

`--data-dir` 和 `--db-path` 驗證：
- 必須為相對路徑（不以 `/` 開頭）
- 不可含 `..`
- `ui_exposed=false`，前端不顯示

### AD6: Admin UI 動態參數

- 前端維護 `CRAWLER_CONFIG` JS 物件，定義各 source 的可用 flags
- 切換 source 時動態渲染 checkbox grid
- 帶值的 flag：checkbox 旁顯示 input（checkbox 勾選才啟用）
- `--monthly` 特殊處理：顯示 year + month 兩個 input

### AD7: 輸出顯示（Modal）

- History 表格新增 "Logs" 欄位，放置 "View" 按鈕
- 點擊後 fetch `/admin/api/crawlers/{job_id}/output`
- Modal 內 `<pre>` 顯示，stdout 白色 + stderr 紅色
- Loading 狀態：按鈕顯示 spinner

### AD8: Daily Fallback 輸出記錄

- 為 daily fallback 也產生 UUID job_id
- 輸出寫入 `scripts/logs/{job_id}.*.log`
- `tracing::info!` 記錄前 500 字元摘要 + job_id
- 不加入 admin crawler_history（維持原有分離設計）

### AD9: Log 檔案管理

- `.gitignore` 新增 `scripts/logs/`
- 不做自動清理（留給使用者/cron 處理）

## Sequence: Admin Trigger

```
Admin UI → POST /admin/api/crawlers/trigger {source, args}
  → validate source (whitelist)
  → validate_args(source, args)
  → check crawler_lock (mutex)
  → create CrawlerJob (stdout=None, stderr=None)
  → tokio::spawn:
    → Command::new("uv").args(["run","python3",script]).args(&args)
    → child.wait_with_output() with timeout
    → capture output.stdout / output.stderr
    → write to scripts/logs/{job_id}.{stdout,stderr}.log
    → truncate to 64KB → store in job.stdout/stderr
    → update job status + push to history
  ← 202 {job_id}

Admin UI → GET /admin/api/crawlers/status (polling)
  ← {running, current_job (no output), history (no output)}

Admin UI → GET /admin/api/crawlers/{job_id}/output (on-demand)
  ← {stdout, stderr}
```

## Sequence: Daily Fallback

```
Client → GET /api/v1/daily?domain=com&date=...
  → DB miss → check daily_fallback map
  → generate job_id (UUID)
  → spawn crawler
  → wait_with_output() with timeout
  → write to scripts/logs/{job_id}.{stdout,stderr}.log
  → tracing::info! summary (500 chars)
  → update fallback entry status
  ← 202 {status: "fetching", retry_after: 30}
```
