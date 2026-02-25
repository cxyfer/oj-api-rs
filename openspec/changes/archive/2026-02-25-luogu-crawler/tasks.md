## 1. Rust 端註冊

- [x] 1.1 在 `src/models.rs` 新增 `LUOGU_ARGS` 靜態白名單（8 個 ArgSpec：`--sync`, `--sync-content`, `--fill-missing-content`, `--missing-content-stats`, `--status`, `--rate-limit`, `--data-dir`, `--db-path`）
- [x] 1.2 在 `CrawlerSource` 枚舉新增 `Luogu` variant，更新 `parse()`、`script_name()`、`arg_specs()` 三個 match arm
- [x] 1.3 執行 `cargo clippy` 和 `cargo build --release` 確認編譯通過無 warning

## 2. Rust 端 Timeout 擴展

- [x] 2.1 在 `src/config.rs` 的 `CrawlerConfig` 新增 `per_source_timeout: HashMap<String, u64>` 欄位（serde default 為空 map）
- [x] 2.2 在 `src/admin/handlers.rs` 取 timeout 時，優先查 `per_source_timeout.get(source)`，fallback 到全域 `timeout_secs`
- [x] 2.3 在 `src/api/daily.rs` 同步套用 per-source timeout 邏輯
- [x] 2.4 在 `config.toml.example` 新增範例：`[crawler.luogu] timeout_secs = 36000`

## 3. Admin UI 模板 + i18n

- [x] 3.1 `templates/admin/crawlers.html`：source-btn-group 新增 `<button class="btn source-btn" data-source="luogu">Luogu</button>`
- [x] 3.2 `templates/admin/problems.html`：source-btn-group 新增 Luogu tab button
- [x] 3.3 `templates/admin/embeddings.html`：source select 新增 `<option value="luogu">Luogu</option>`
- [x] 3.4 `static/i18n/en.json`：新增 `"problems.sources.luogu": "Luogu"` 鍵
- [x] 3.5 `static/i18n/zh-TW.json`：新增 `"problems.sources.luogu": "洛谷"` 鍵
- [x] 3.6 `static/i18n/zh-CN.json`：新增 `"problems.sources.luogu": "洛谷"` 鍵

## 4. Python 爬蟲骨架

- [x] 4.1 建立 `scripts/luogu.py`，定義 `LuoguClient(BaseCrawler)` 類別與 `__init__`（`super().__init__(crawler_name="luogu")`，data_dir, db_path, rate_limit, max_retries=3, backoff_base=2.0, max_backoff=60.0）；`self.rate_limit = max(rate_limit, 2.0)`
- [x] 4.2 定義常數 `CURL_IMPERSONATE = "chrome124"`，所有 `_create_curl_session` 呼叫使用此常數
- [x] 4.3 實作 `_throttle()` 方法（`time.monotonic()` + `asyncio.sleep`，強制 `max(rate_limit, 2.0)`）
- [x] 4.4 實作 `_fetch_text(session, url, referer)` 方法（retry max_retries=3 + backoff；HTTP 403/429/503 視為可重試；達上限回傳 None）
- [x] 4.5 實作 `_is_rate_limited(html)` 方法（複用 codeforces markers + lentille-context 缺失偵測）
- [x] 4.6 Logger 使用 `get_leetcode_logger()`，不自行 `basicConfig`

## 5. 資料擷取與映射

- [x] 5.1 實作 `_extract_lentille_context(html)` — BeautifulSoup 選取 `script[type="application/json"]` 含 `lentille-context` 屬性，解析 JSON；缺少必要路徑 `data.problems.result`/`count` 時 raise/return None + ERROR log
- [x] 5.2 實作 `_fetch_tags_map(session)` — 請求 `/_lfe/tags`，解析 `{"tags": [...]}` 結構，建立 `{tag_id: tag_name}` dict；失敗時 fallback 快取或全降級為 ID 字串
- [x] 5.3 實作 `DIFFICULTY_MAP` 常數與 `_map_difficulty(value)` — 0-7 映射中文文字，其餘返回 None
- [x] 5.4 實作 `_map_problem(raw, tag_map)` — 完整欄位映射：pid→id+slug, title→title+title_cn, ac_rate 安全除法（totalSubmit==0→None）, tags 轉換含 fallback, link 組合, difficulty 轉換, category="Algorithms", paid_only=0；缺少 pid 時 skip + WARNING
- [x] 5.5 實作 `_serialize_tags(tags)` — `json.dumps()` 序列化 tags 列表（必須在呼叫 `update_problems()` 前完成）

## 6. Progress 追蹤（僅 --sync）

- [x] 6.1 實作 `get_progress()` — 讀取 `luogu_progress.json`，JSON 損壞時返回安全預設值 + WARNING
- [x] 6.2 實作 `save_progress(page)` — 更新 `completed_pages`（字串 append-only set）+ `last_completed_page` + `last_updated`，原子寫入（tmp → fsync → rename）
- [x] 6.3 實作 tags_map 快取邏輯 — sync 時寫入 progress，下次 sync 先嘗試讀取快取作為 fallback

## 7. 核心 Sync 流程

- [x] 7.1 實作 `sync()` — 抓取 tags_map → 逐頁抓取 HTML → 解析 lentille-context → 映射題目 → batch `update_problems()` → save_progress
- [x] 7.2 實作隱式 resume 邏輯 — 讀取 `completed_pages`，跳過已完成頁（無顯式 `--resume` 旗標）
- [x] 7.3 實作分頁停止條件 — 動態更新 `total_pages`，空結果頁停止
- [x] 7.4 Extra fields in JSON SHALL be ignored；缺少非必要欄位以 WARNING + 降級處理

## 8. CLI 與 Status

- [x] 8.1 實作 `show_status()` — 顯示已完成頁數、最後完成頁、total_count_snapshot、last_updated、DB 中 luogu 題目數
- [x] 8.2 實作 `show_missing_content_stats()` — 透過 `count_missing_content(source="luogu")` 顯示數量
- [x] 8.3 實作 `main()` 函式 — argparse 定義 8 個旗標，`--sync-content` 和 `--fill-missing-content` 去重為同一操作，無 flag 時印出 help

## 9. 題目詳細內容爬取

- [x] 9.1 實作 `_compose_content_markdown(content, samples)` — 組合結構化 Markdown；所有區塊皆空時回傳 `""`
- [x] 9.2 實作 `fetch_problem_content(session, pid)` — 抓取 `/problem/{pid}` HTML → 解析 lentille-context → 提取 `data.problem.content` 和 `data.problem.samples` → 呼叫 `_compose_content_markdown()` 回傳；`data.problem.content` 為 null/missing 時 log WARNING 回傳 None
- [x] 9.3 實作 `sync_content()` — 透過 `get_problem_ids_missing_content(source="luogu")` 查詢 DB → 逐題 `fetch_problem_content()` → 結果為 `""` 時不更新 DB（保持 NULL）→ 結果非空時加入 batch → 每 10 題呼叫 `batch_update_content(..., batch_size=10)` 寫入 DB
- [x] 9.4 實作 `--sync-content` / `--fill-missing-content` CLI 入口 — DB 無 missing content 時 log "No problems with missing content" 並 exit

## 10. 驗證

- [x] 10.1 手動測試 `python3 luogu.py --sync --rate-limit 2.0` 抓取前幾頁確認資料正確寫入 DB
- [x] 10.2 驗證 DB 中 `source='luogu'` 的題目：slug 非空、difficulty 為中文文字、tags 為中文文字 JSON array、category='Algorithms'、paid_only=0
- [x] 10.3 驗證 `python3 luogu.py --status` 輸出正確
- [x] 10.4 驗證 Rust 端 `cargo build --release` + `cargo clippy` 通過
- [x] 10.5 手動測試 `python3 luogu.py --sync-content --rate-limit 2.0` 抓取數題確認 content 正確寫入 DB
- [x] 10.6 驗證 DB 中 content 為組合後的 Markdown 格式（含 ## 標題、程式碼區塊）
- [x] 10.7 驗證 `python3 luogu.py --missing-content-stats` 輸出正確
- [x] 10.8 驗證 `--sync-content` DB-driven resume：中斷後重跑，已有 content 的題目被跳過
- [x] 10.9 驗證 Admin UI crawlers 頁面顯示 Luogu 按鈕且可觸發
- [x] 10.10 驗證 Admin UI problems 頁面顯示 Luogu tab
- [ ] 10.11 驗證 per-source timeout 生效（Luogu 使用擴展 timeout）
