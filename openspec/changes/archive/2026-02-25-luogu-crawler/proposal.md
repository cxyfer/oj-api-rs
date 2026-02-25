# Change: luogu-crawler

## Context

新增 Luogu（洛谷）爬蟲腳本，使現有 OJ API 能查詢洛谷的競程題目。洛谷是中國最大的競程社群之一，擁有約 15,400 道題目。

## Requirements

### R1: Python 爬蟲腳本 `scripts/luogu.py`

- 繼承 `BaseCrawler`，使用 `curl_cffi` 的 `AsyncSession(impersonate="chrome124")`
- 遵循 `codeforces.py` 的程式碼結構與風格（argparse CLI、logger、rate limiting、retry with backoff）

### R2: 資料來源 — HTML 嵌入 JSON

- 從 `https://www.luogu.com.cn/problem/list?page=N` 抓取 HTML
- 從 `<script>` tag（`type="application/json"`，含 `lentille-context` 屬性）提取嵌入的 JSON
- JSON 路徑：解析後取得 `data.problems.result[]` 陣列和 `data.problems.count` 總數
- 每頁 50 題，自動遍歷所有分頁直到抓完

### R3: Tags 映射

- 從 `https://www.luogu.com.cn/_lfe/tags` 取得完整 tag 列表（496 個）
- 回傳結構：`{"tags": [...], "types": [...], "version": ...}`
- 每個 tag：`{"id": int, "name": str, "type": int, "parent": int|null}`
- 保留所有 type 的 tags（type 1-6），將題目的 `tags` 數字陣列轉換為對應的 `name` 文字陣列
- Tags 映射在爬蟲啟動時抓取一次，快取於記憶體中

### R4: Difficulty 轉換

- 將數字 0-7 轉換為中文文字後存入 DB 的 `difficulty` TEXT 欄位：
  - 0 → `暂无评定`, 1 → `入门`, 2 → `普及−`, 3 → `普及/提高−`
  - 4 → `普及+/提高`, 5 → `提高+/省选−`, 6 → `省选/NOI−`, 7 → `NOI/NOI+/CTSC`

### R5: DB 欄位映射

寫入 `problems` 表，`source = "luogu"`，欄位映射：

| Luogu 欄位 | DB 欄位 | 說明 |
|---|---|---|
| `pid` | `id`, `slug` | 如 "P1000" |
| `title` | `title` | 題目名稱 |
| — | `title_cn` | 同 `title`（洛谷本身為簡中） |
| `difficulty` (轉換後) | `difficulty` | 中文文字 |
| `totalAccepted / totalSubmit` | `ac_rate` | 計算通過率 |
| — | `rating` | NULL（洛谷無 rating） |
| — | `contest` | NULL |
| — | `problem_index` | NULL |
| `tags` (轉換後) | `tags` | JSON array of strings |
| 組合 URL | `link` | `https://www.luogu.com.cn/problem/{pid}` |
| — | `category` | "Algorithms" |
| — | `paid_only` | 0 |
| `content` 物件組合 | `content` | Markdown（由 `--sync-content` 填入，初始為 NULL） |

### R6: CLI 介面

- `--sync`：同步所有題目列表（遍歷全部分頁）
- `--sync-content`：逐題抓取題目詳細內容，填入 DB 的 `content` 欄位（僅處理 content 為 NULL 的題目）
- `--fill-missing-content`：`--sync-content` 的別名，與 AtCoder/Codeforces 命名一致
- `--missing-content-stats`：顯示缺少 content 的題目數量統計
- `--status`：顯示進度（已抓取頁數、DB 中 luogu 題目數）
- `--rate-limit <float>`：請求間隔秒數（預設 2.0）
- `--data-dir <str>`：資料目錄
- `--db-path <str>`：資料庫路徑

### R7: Rust 端註冊

在 `src/models.rs` 中：
- `CrawlerSource` 枚舉新增 `Luogu` variant
- 新增 `LUOGU_ARGS` 白名單（`--sync`, `--sync-content`, `--fill-missing-content`, `--missing-content-stats`, `--status`, `--rate-limit`, `--data-dir`, `--db-path`）
- `parse()` 新增 `"luogu"` 映射
- `script_name()` 回傳 `"luogu.py"`
- `arg_specs()` 回傳 `LUOGU_ARGS`

### R8: 進度追蹤

- 使用 `data/luogu_progress.json` 記錄已抓取的最大頁碼和時間戳
- `--sync` 支援斷點續傳（從上次頁碼繼續）
- `--sync-content` 使用 `content_completed_pids` 集合追蹤已抓取 content 的題目，支援斷點續傳

### R9: 題目詳細內容爬取

- 從 `https://www.luogu.com.cn/problem/{pid}` 抓取 HTML
- 從 `lentille-context` script tag 提取 `data.problem.content` 物件和 `data.problem.samples` 陣列
- 將各區塊組合為單一 Markdown 文件，格式：
  - `## 题目背景`（background，若非空）
  - `## 题目描述`（description）
  - `## 输入格式`（formatI）
  - `## 输出格式`（formatO）
  - `## 样例`（samples，格式化為輸入/輸出程式碼區塊）
  - `## 说明/提示`（hint，若非空）
- 各區塊的原始內容已是 Markdown，無需 HTML 轉換
- 組合後的 Markdown 寫入 DB 的 `content` 欄位

## Constraints

### Hard Constraints

- C1: 必須使用 `curl_cffi` + `impersonate="chrome124"` — Luogu 有 Cloudflare 防護
- C2: 題目資料從 HTML 中的 `lentille-context` script tag 提取 — `_contentOnly=1` API 已失效
- C3: Tags API (`/_lfe/tags`) 回傳 dict 結構 `{"tags": [...]}` 而非純 array
- C4: DB schema 不可變更 — 使用現有 `problems` 表的 `PRIMARY KEY (source, id)`
- C5: `ProblemsDatabaseManager` 的 `update_problems()` 使用 `INSERT OR IGNORE`，tags 欄位需預先 `json.dumps()`
- C6: Rust 端 `validate_args()` 會驗證所有參數，新增的 `LUOGU_ARGS` 必須與 Python CLI 的 argparse 定義完全對應
- C7: 請求間隔最低 2.0 秒 — Luogu 有嚴格的 rate limiting

### Soft Constraints

- S1: 遵循 `codeforces.py` 的程式碼風格（class 結構、logger 使用、error handling 模式）
- S2: 使用 `get_leetcode_logger()` 作為 logger（與其他爬蟲一致）
- S3: 進度檔案使用 atomic write（tmp + rename）模式，與 codeforces 一致

## Success Criteria

- SC1: `python3 luogu.py --sync` 能成功抓取所有 ~15,400 題並寫入 DB
- SC2: DB 中 `source='luogu'` 的題目 difficulty 為中文文字、tags 為中文文字 JSON array
- SC3: `cargo build --release` 編譯通過，Rust 端能識別 `"luogu"` source 並驗證參數
- SC4: `cargo clippy` 無 warning
