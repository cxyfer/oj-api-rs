# Proposal: Luogu Training List & SPOJ Crawling

## Context

luogu.py 目前僅支援全量同步 (`--sync`) 和內容補全 (`--fill-missing-content`)。使用者需要：
1. 從 Luogu 題單 (training list) 爬取特定題目集合
2. 從 Luogu 的 SP 子題庫爬取 SPOJ 題目，以 `source='spoj'` 存入 DB

## FEAT 1: Luogu 題單爬取 (`--training-list`)

### 需求
- 新增 `--training-list <value>` 參數，接受：
  - 題單 URL: `https://www.luogu.com.cn/training/378042#problems` 或 `https://www.luogu.com.cn/training/378042`
  - 題單 ID: `378042`
- 從題單頁面的 lentille-context JSON 提取 `training.problems[]`
- 跳過 pid 以 `AT` 或 `CF` 開頭的題目（由其他 source 處理）
- 僅存 metadata（pid, title, difficulty, tags, ac_rate），不自動補全 content
- 所有題目以 `source='luogu'` 存入，複用現有 `_map_problem()` 邏輯

### 約束
- C1: 參數值型別為 `Str`，Python 端負責解析 URL/ID
- C2: 題單頁面無分頁（`perPage: null`），單次請求取得全部題目
- C3: 題單 lentille-context 結構為 `data.training.problems[]`，每個元素含 `problem` 子物件
- C4: 跳過規則：`pid.startswith("AT")` 或 `pid.startswith("CF")`
- C5: 需同步更新 3 處：`LUOGU_ARGS`（models.rs）、`CRAWLER_CONFIG`（admin.js）、i18n

### 驗收標準
- `luogu.py --training-list 378042` 成功爬取題單中的 Luogu 原生題目
- `luogu.py --training-list https://www.luogu.com.cn/training/378042` 同上
- AT/CF 開頭的題目被跳過並記錄 log
- DB 中新增的題目 source='luogu'，slug=pid，link 指向 Luogu

## FEAT 2: SPOJ 子題庫爬取 (`--sync-spoj`)

### 需求
- 新增 `--sync-spoj` 參數（無值，flag 型別）
- 爬取 `https://www.luogu.com.cn/problem/list?type=SP&page={n}`，共 3674 題，50 題/頁
- 存入 DB 時：`source='spoj'`，`id='SP{n}'`（保留 Luogu 編號），`slug` 從 title 解析 SPOJ code
- Title 格式 `"CODE - Name"` → slug=`CODE`，title=`Name`
- Link 指向 `https://www.spoj.com/problems/{CODE}/`
- Content 透過 Luogu 的 `/problem/SP{n}` 頁面取得（複用 `fetch_problem_content()`）
- 需在 Rust 端新增 `CrawlerSource::Spoj` 變體，但 `script_name()` 仍回傳 `"luogu.py"`（共用腳本）

### 約束
- C6: DB PK 為 `(source, id)`，SPOJ 題目用 `source='spoj'`, `id='SP1'` 等
- C7: slug 從 title 的 `" - "` 分隔符前半段取得；若無分隔符則 slug=id
- C8: 分頁結構與現有 `sync()` 相同：`data.problems.count`、`data.problems.result[]`、50/頁
- C9: 進度追蹤使用獨立檔案 `spoj_progress.json`，避免與 `luogu_progress.json` 衝突
- C10: `--fill-missing-content` 需支援 `source='spoj'`，透過 Luogu `/problem/{pid}` 取得
- C11: Rust 端需新增：`SPOJ_ARGS` 白名單、`CrawlerSource::Spoj`、`script_name()` → `"luogu.py"`
- C12: `detect.rs` 已支援 `SP\d+` 模式（歸類為 luogu），需考慮是否調整為歸類到 spoj

### 驗收標準
- `luogu.py --sync-spoj` 成功爬取所有 SP 題目並存入 DB（source='spoj'）
- DB 中 slug 為 SPOJ 原始 code（如 TEST），link 指向 spoj.com
- `luogu.py --fill-missing-content --source spoj` 可補全 SPOJ 題目內容
- Admin UI 可觸發 SPOJ 爬蟲（透過新的 CrawlerSource::Spoj）

## 影響範圍

### Python (scripts/)
- `luogu.py`: 新增 `sync_training_list()`, `sync_spoj()` 方法 + argparse 參數

### Rust (src/)
- `src/models.rs`: LUOGU_ARGS 新增 `--training-list`；新增 SPOJ_ARGS + CrawlerSource::Spoj
- `src/detect.rs`: 考慮 SP\d+ 偵測結果從 luogu → spoj 的調整

### Frontend (static/)
- `static/admin.js`: CRAWLER_CONFIG 新增 luogu 的 training-list 欄位；新增 spoj source tab
- `static/i18n.js`: 新增對應翻譯 key

## 風險
- R1: Luogu 反爬機制可能影響大量 SP 題目爬取（3674 題 × 內容補全）
- R2: `detect.rs` 中 SP\d+ 目前歸類為 luogu，改為 spoj 可能影響現有行為
- R3: 題單頁面的 lentille-context 結構可能與題目列表頁不同，需實際驗證
