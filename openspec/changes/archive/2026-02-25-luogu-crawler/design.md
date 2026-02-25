## Context

現有系統已支援 LeetCode、AtCoder、Codeforces 三個 OJ 的爬蟲。每個爬蟲繼承 `BaseCrawler`，使用 `curl_cffi` 繞過 Cloudflare，透過 `ProblemsDatabaseManager` 寫入共用 `problems` 表（`PRIMARY KEY (source, id)`）。Rust 端透過 `CrawlerSource` 枚舉 + `ArgSpec` 白名單驗證爬蟲參數。

洛谷是中國最大的競程社群，約 15,400 題。其資料來源為 HTML 頁面中嵌入的 JSON（`lentille-context` script tag），而非 REST API。

## Goals / Non-Goals

**Goals:**
- 新增 `scripts/luogu.py` 爬蟲，抓取洛谷全部題目列表並寫入 DB
- Rust 端註冊 `Luogu` source，使 Admin API 可觸發爬蟲
- 支援斷點續傳、rate limiting、Cloudflare 偵測

**Non-Goals:**
- 不修改 DB schema
- 不重構現有爬蟲的共用邏輯（如抽出 mixin）

**Scope Addition (from spec-plan audit):**
- Admin UI 模板新增 Luogu source 按鈕（crawlers.html, problems.html, embeddings.html）
- i18n JSON 新增 Luogu 翻譯鍵
- Rust 端 crawler timeout 擴展為 per-source 機制

## Decisions

### D1: 類別結構 — 單一 `LuoguClient(BaseCrawler)`

沿用 `CodeforcesClient` 的風格，單一類別包含所有邏輯。

- **選擇理由**：Luogu 需求單純（列表頁 + tags API + 分頁），不需要拆分子類別
- **替代方案**：抽出 `CurlBackoffCrawlerMixin` 共用 throttle/retry — 但變更面過大，且目前只有 Luogu 一個新需求，屬過度設計
- **方法清單**：`__init__`, `_throttle`, `_fetch_text`, `_is_rate_limited`, `_extract_lentille_context`, `_fetch_tags_map`, `_map_difficulty`, `_map_problem`, `_serialize_tags`, `_compose_content_markdown`, `fetch_problem_content`, `sync`, `sync_content`, `show_status`, `show_missing_content_stats`, `get_progress`, `save_progress`

### D2: 資料擷取 — HTML lentille-context 解析

從 `https://www.luogu.com.cn/problem/list?page=N` 的 HTML 中，以 BeautifulSoup 選取 `script[type="application/json"]` 且含 `lentille-context` 屬性的 tag，解析其 JSON 內容。

- **選擇理由**：`_contentOnly=1` API 已失效（C2），HTML 嵌入 JSON 是目前唯一穩定的資料來源
- **JSON 路徑**：`data.problems.result[]`（題目陣列）、`data.problems.count`（總數）
- **分頁**：每頁 50 題，`total_pages = ceil(count / 50)`，每頁動態更新 `total_pages` 以應對 count 漂移

### D3: Tags 映射 — 快取至 progress 檔案

從 `/_lfe/tags` API 取得 `{"tags": [...], "types": [...], "version": ...}` 結構（C3），建立 `{tag_id: tag_name}` 映射。

- **快取策略**：tags_map 存入 `luogu_progress.json`，每次 `--sync` 重新抓取並覆蓋（overwrite with latest）
- **降級行為**：若 API 失敗，嘗試讀取快取；若快取也不存在，所有 tags 降級為原始 ID 字串（如 `"185"`）
- **未知 tag ID**：映射時找不到的 ID 直接轉為字串保留，不丟棄、不中斷

### D4: Difficulty 映射 — 有限映射 + None 降級

```python
DIFFICULTY_MAP = {
    0: "暂无评定", 1: "入门", 2: "普及−", 3: "普及/提高−",
    4: "普及+/提高", 5: "提高+/省选−", 6: "省选/NOI−", 7: "NOI/NOI+/CTSC",
}
```

- 0-7 以外的值（含 null、負數、>7）一律映射為 `None`，DB 存 NULL
- **理由**：避免猜測未來新等級的語義，NULL 明確表達「未知」

### D5: ac_rate 計算 — 安全除法

- `totalSubmit == 0` → `ac_rate = None`（表達「無提交」，區別於 0% 通過率）
- `totalSubmit > 0` → `ac_rate = totalAccepted / totalSubmit`（浮點數，範圍 [0.0, 1.0]）

### D6: Progress 追蹤 — completed_pages 集合

```json
{
  "completed_pages": ["1", "2", "3"],
  "last_completed_page": 3,
  "total_count_snapshot": 15400,
  "tags_map": {"185": "动态规划", "42": "模拟", ...},
  "last_updated": "2026-02-25T12:00:00+00:00"
}
```

- **寫入時機**：每頁 DB commit 成功後才 `save_progress(page)`
- **原子寫入**：tmp file → `fsync` → `rename`（沿用 codeforces 模式）
- **Resume**：`--sync` 時讀取 `completed_pages`，跳過已完成頁
- **單一寫入者**：Rust 端 `crawler_lock` 保證同時只有一個 crawler job

### D7: Cloudflare 偵測 — 複用 codeforces markers

`_is_rate_limited(html)` 檢測規則：
1. Cloudflare challenge title：`"just a moment..."`, `"attention required"`
2. 通用 rate limit markers：`"too many requests"`, `"captcha"`, `"cloudflare"`
3. **額外信號**：若 HTTP 200 但 `lentille-context` script tag 不存在，視為疑似被攔截，觸發 retry

偵測到時觸發指數退避（backoff_base=2.0, max_backoff=60.0），不寫 progress。

### D8: Rate Limiting — 最低 2.0 秒

- Python 端 `__init__` 強制 `self.rate_limit = max(rate_limit, 2.0)`（C7）
- Rust 端 `LUOGU_ARGS` 的 `--rate-limit` 為 `Float` 類型，驗證 >0
- 雙重保護：Rust 驗證格式，Python 強制下限

### D9: Rust 端註冊 — 最小變更

在 `src/models.rs` 中：
- `CrawlerSource` 枚舉新增 `Luogu`
- `parse("luogu")` → `Ok(Self::Luogu)`
- `script_name()` → `"luogu.py"`
- `arg_specs()` → `LUOGU_ARGS`

`LUOGU_ARGS` 白名單：

| Flag | Arity | ValueType | ui_exposed |
|---|---|---|---|
| `--sync` | 0 | None | true |
| `--sync-content` | 0 | None | true |
| `--fill-missing-content` | 0 | None | true |
| `--missing-content-stats` | 0 | None | true |
| `--status` | 0 | None | true |
| `--rate-limit` | 1 | Float | true |
| `--data-dir` | 1 | Str | false |
| `--db-path` | 1 | Str | false |

### D10: DB 寫入策略 — batch INSERT OR IGNORE

- 每頁 50 題，呼叫 `ProblemsDatabaseManager.update_problems()` 批次寫入
- `tags` 欄位需預先 `json.dumps()`（C5）
- `INSERT OR IGNORE` 保證冪等性，重跑不會覆蓋已存在的資料

### D11: Content 組合 — 結構化 Markdown

從 `data.problem.content` 物件和 `data.problem.samples` 陣列組合為單一 Markdown 字串：

```python
def _compose_content_markdown(self, content: dict, samples: list) -> str:
    sections = []
    if content.get("background"):
        sections.append(f"## 题目背景\n\n{content['background']}")
    if content.get("description"):
        sections.append(f"## 题目描述\n\n{content['description']}")
    if content.get("formatI"):
        sections.append(f"## 输入格式\n\n{content['formatI']}")
    if content.get("formatO"):
        sections.append(f"## 输出格式\n\n{content['formatO']}")
    if samples:
        parts = []
        for i, (inp, out) in enumerate(samples, 1):
            parts.append(f"### 样例输入 #{i}\n\n```\n{inp}\n```")
            parts.append(f"### 样例输出 #{i}\n\n```\n{out}\n```")
        sections.append(f"## 样例\n\n" + "\n\n".join(parts))
    if content.get("hint"):
        sections.append(f"## 说明/提示\n\n{content['hint']}")
    return "\n\n".join(sections)
```

- 各區塊原始內容已是 Markdown（含 LaTeX `$...$` 數學公式），無需 HTML 轉換
- 區塊標題使用洛谷官方的簡中標題（`题目背景`、`题目描述` 等）
- 空區塊（如無 background）直接跳過
- samples 格式化為帶編號的輸入/輸出程式碼區塊

### D12: Content 爬取策略 — 獨立 `--sync-content` 指令（DB-driven resume）

- `--sync-content` 查詢 DB 中 `source='luogu' AND (content IS NULL OR content = '') AND category='Algorithms' AND paid_only=0` 的所有 pid（透過 `get_problem_ids_missing_content(source="luogu")`）
- 逐題抓取 `https://www.luogu.com.cn/problem/{pid}` 的 HTML
- 從 `lentille-context` 提取 `data.problem.content` 和 `data.problem.samples`
- 呼叫 `_compose_content_markdown()` 組合後，透過 `ProblemsDatabaseManager.batch_update_content()` 寫入 DB（`batch_size=10`）
- **Resume 策略**：純 DB-driven，不在 progress JSON 中追蹤 `content_completed_pids`。每次執行重新查詢 DB 中 `content IS NULL` 的 pid 列表，已完成的自然不會出現
- `--fill-missing-content` 為 `--sync-content` 的別名
- `--missing-content-stats` 透過 `count_missing_content(source="luogu")` 顯示數量
- **全空 content 處理**：若 `_compose_content_markdown` 所有區塊皆空（回傳 `""`），caller 不更新 DB（保持 NULL），下次重試

### D13: Progress 檔案結構（僅 --sync 使用）

`luogu_progress.json` 不包含 content 追蹤欄位：

```json
{
  "completed_pages": ["1", "2", "3"],
  "last_completed_page": 3,
  "total_count_snapshot": 15400,
  "tags_map": {"185": "动态规划", "42": "模拟"},
  "last_updated": "2026-02-25T12:00:00+00:00"
}
```

- `completed_pages` 為字串陣列（與 Codeforces `fetched_contests` 格式一致）
- `completed_pages` 為 append-only monotonic set，`--sync` 永不自動清空
- 顯示時使用 numeric sort key 避免字典序陷阱

### D14: HTTP 請求參數（spec-plan 審計新增）

- `_fetch_text` 的 `max_retries=3`（內部參數，不暴露為 CLI）
- `impersonate="chrome124"`（以常數 `CURL_IMPERSONATE` 集中定義）
- HTTP `403`/`429`/`503` 視為可重試狀態，觸發指數退避並消耗 retry 次數
- 達 retry 上限後回傳 `None`，該頁/題不寫 progress
- 無整體任務層級 retry loop，失敗頁/題留待下次重跑

### D15: Logging 規範（spec-plan 審計新增）

- 使用 `get_leetcode_logger()` + `scripts/utils/logger.py` 全域 formatter
- 不自行 `basicConfig`
- INFO：一般進度（頁碼、題數）；WARNING：重試、跳過、降級；ERROR：解析失敗、DB 失敗

### D16: CLI 語義（spec-plan 審計新增）

- 不提供顯式 `--resume` 旗標；`--sync` 一律隱式 resume
- `--sync-content` 和 `--fill-missing-content` 同時傳入時視為同一操作（去重）
- 無 flag 時印出 help 並 exit
- `--sync-content` 在 DB 無 missing content 時 log "No problems with missing content" 並正常 exit

### D17: Rust 端 Timeout 擴展（spec-plan 審計新增）

- `CrawlerConfig` 新增 `per_source_timeout` 欄位（`HashMap<String, u64>`），允許 per-source 覆蓋全域 `timeout_secs`
- `config.toml` 範例：`[crawler] timeout_secs = 300` + `[crawler.luogu] timeout_secs = 36000`
- Admin handler 取 timeout 時優先查 per-source，fallback 到全域
- 此機制對所有 source 通用，不僅限 Luogu

### D18: Admin UI 模板更新（spec-plan 審計新增）

需修改的檔案：
- `templates/admin/crawlers.html`：source-btn-group 新增 `<button data-source="luogu">Luogu</button>`
- `templates/admin/problems.html`：source-btn-group 新增 Luogu tab
- `templates/admin/embeddings.html`：source select 新增 `<option value="luogu">Luogu</option>`
- `static/i18n/en.json`、`zh-TW.json`、`zh-CN.json`：新增 `problems.sources.luogu` 鍵

### D19: DB 欄位必填約束（spec-plan 審計新增）

- `slug` 為 `NOT NULL`：Luogu 使用 `pid`（如 `"P1000"`）作為 `slug`
- `category` 必須為 `"Algorithms"`：`get_problem_ids_missing_content` 過濾條件依賴此值
- `paid_only` 必須為 `0`：同上過濾條件
- DB 模組路徑為 `scripts/utils/database.py`（非 `db.py`）
- `tags` 必須預先 `json.dumps()` 序列化（`update_problems()` 不自動序列化）

### D20: 增量更新語義（spec-plan 審計新增）

- `--sync` 為增量更新（append-only），不處理 upstream 刪除
- 與 Codeforces/AtCoder 行為一致
- `INSERT OR IGNORE` 保證冪等性

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|---|---|---|
| Cloudflare 升級封鎖 `curl_cffi` | 整體 sync 失敗 | `impersonate="chrome124"` 常數集中管理 + retry/backoff；未來僅改一處 |
| `lentille-context` 結構變更 | JSON 解析失敗 | 必要路徑缺失 → fail fast + ERROR log；非必要欄位缺失 → WARNING + 降級；不寫 progress |
| 分頁 count 漂移 | 漏抓或多抓 | 每頁動態更新 total_pages；`INSERT OR IGNORE` 容忍重複 |
| Tags API 格式變更 | tag 映射失敗 | 降級為原始 ID 字串，不中斷 sync |
| 15000+ 題全量抓取耗時長 | 中途中斷 | completed_pages 集合支援斷點續傳 |
| 15000+ 題逐題抓取 content 耗時極長（~8.5hr） | Rust timeout kill | per-source timeout 擴展（D17）；DB-driven resume 無需額外 progress 追蹤 |
| 題目詳情頁 content 結構變更 | content 解析失敗 | 檢查 content 物件是否存在，缺失時 skip + WARNING；不影響已完成的題目 |
| 部分題目無 content（如特殊題型） | content 為空 | `_compose_content_markdown` 回傳 `""`，caller 不更新 DB（保持 NULL），下次重試 |
| Admin UI 硬編碼 source 列表 | Luogu 不可見 | 同步更新 3 個模板 + 3 個 i18n JSON（D18） |
| HTTP 403/429 非 challenge 頁面 | 無限重試 | 視為可重試狀態，消耗 max_retries=3 後放棄（D14） |
