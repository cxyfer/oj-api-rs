# Tasks

## T1: Rust — LUOGU_ARGS 擴展 ✅

**Files:** `src/models.rs`

1. 在 `LUOGU_ARGS` 陣列末尾（`--db-path` 之前）新增：
   ```rust
   ArgSpec { flag: "--training-list", arity: 1, value_type: ValueType::Str, ui_exposed: true },
   ArgSpec { flag: "--source", arity: 1, value_type: ValueType::Str, ui_exposed: true },
   ```

**Acceptance:** `validate_args(&CrawlerSource::Luogu, &["--training-list", "378042"])` returns Ok; `validate_args(&CrawlerSource::Luogu, &["--source", "spoj"])` returns Ok.

---

## T2: Rust — CrawlerSource::Spoj + SPOJ_ARGS ✅

**Files:** `src/models.rs`

1. 新增 `SPOJ_ARGS` 靜態陣列：
   ```rust
   pub static SPOJ_ARGS: &[ArgSpec] = &[
       ArgSpec { flag: "--sync-spoj", arity: 0, value_type: ValueType::None, ui_exposed: true },
       ArgSpec { flag: "--rate-limit", arity: 1, value_type: ValueType::Float, ui_exposed: true },
       ArgSpec { flag: "--batch-size", arity: 1, value_type: ValueType::Int, ui_exposed: true },
       ArgSpec { flag: "--data-dir", arity: 1, value_type: ValueType::Str, ui_exposed: false },
       ArgSpec { flag: "--db-path", arity: 1, value_type: ValueType::Str, ui_exposed: false },
   ];
   ```
2. `CrawlerSource` 枚舉新增 `Spoj` 變體
3. `parse("spoj")` → `Ok(Self::Spoj)`
4. `script_name()` → `"luogu.py"`
5. `arg_specs()` → `SPOJ_ARGS`

**Acceptance:** `CrawlerSource::parse("spoj")` returns Ok; `script_name()` == `"luogu.py"`; `validate_args(&CrawlerSource::Spoj, &["--sync-spoj"])` returns Ok; `validate_args(&CrawlerSource::Spoj, &["--training-list", "x"])` returns Err.

---

## T3: Rust — detect.rs SP\d+ 路由 ✅

**Files:** `src/detect.rs`

1. 從 `LUOGU_ID_RE` 移除 `SP\d+`：
   - 新 regex: `^([PBTU]\d+|CF\d+[A-Z]|AT_(?:abc|arc|agc|ahc)\d+_[a-z]\d*|UVA\d+)$`
2. 新增 `SP_ID_RE`: `^SP\d+$`（`(?i)` 大小寫無關）
3. 在 `detect_source()` 中，Luogu ID regex 匹配前新增：
   ```rust
   if SP_ID_RE.is_match(pid) {
       return ("spoj", pid.to_uppercase());
   }
   ```
4. 在 Luogu URL handler 中，偵測到 SP 開頭 pid 時改為回傳 `("spoj", luogu_pid)`：
   ```rust
   if luogu_pid.starts_with("SP") {
       return ("spoj", luogu_pid);
   }
   ```
   （此檢查須在 CF/AT 檢查之後、最終 return `("luogu", ...)` 之前）

**Acceptance:** `detect_source("SP1")` == `("spoj", "SP1")`; `detect_source("https://www.luogu.com.cn/problem/SP1")` == `("spoj", "SP1")`; `detect_source("P1000")` == `("luogu", "P1000")`（不受影響）.

---

## T4: Rust — API VALID_SOURCES ✅

**Files:** `src/api/problems.rs`

1. 在 `VALID_SOURCES` 常數中新增 `"spoj"`

**Acceptance:** API endpoint `/api/v1/problems?source=spoj` 不再回傳 400.

---

## T5: Python — sync_training_list() ✅

**Files:** `scripts/luogu.py`

1. 新增 `sync_training_list(self, training_list_value: str, overwrite: bool = False)` 方法
2. 解析輸入值：
   - 若包含 `luogu.com.cn/training/`，提取 training ID（URL path 最後一段數字）
   - 否則視為純數字 ID
3. Fetch `https://www.luogu.com.cn/training/{id}` HTML
4. 提取 lentille-context，取 `data.training.problems[]`
5. 對每個 item，取 `item["problem"]` 子物件
6. 過濾：skip `pid.startswith("AT")` or `pid.startswith("CF")`，log skipped
7. 呼叫 `_map_problem()` 映射後 `update_problems()` 寫入 DB
8. argparse 新增 `--training-list` 參數（type=str）
9. main() 中 `--training-list` 觸發 `client.sync_training_list(args.training_list, overwrite=args.overwrite)`

**Acceptance:** `luogu.py --training-list 378042` 成功爬取並寫入 DB；AT/CF pid 被 skip 並 log.

---

## T6: Python — sync_spoj() ✅

**Files:** `scripts/luogu.py`

1. 新增 `sync_spoj(self, overwrite: bool = False)` 方法
2. 設定 `self.progress_file = self.data_dir / "spoj_progress.json"` 在方法開頭
3. Fetch `https://www.luogu.com.cn/problem/list?type=SP&page={n}` 逐頁
4. 分頁結構同 `sync()`：`data.problems.count`、`data.problems.result[]`、50/頁
5. 每個 problem 的映射邏輯：
   - `source = "spoj"`
   - `id = pid`（如 `"SP1"`）
   - title 解析：`title.split(" - ", 1)` → `slug = parts[0].strip()`, `title = parts[1].strip()` if len > 1, else `slug = id`
   - `link = f"https://www.spoj.com/problems/{slug}/"`
   - difficulty: 沿用 `_map_difficulty()`
   - 其餘欄位同 `_map_problem()` 但 source/slug/link/title 覆寫
6. Progress tracking 使用 `spoj_progress.json`
7. argparse 新增 `--sync-spoj` 參數（action="store_true"）
8. main() 中 `--sync-spoj` 觸發 `client.sync_spoj(overwrite=args.overwrite)`

**Acceptance:** `luogu.py --sync-spoj` 爬取所有 SP 題目存入 DB（source='spoj'）；slug 為 SPOJ code；link 指向 spoj.com.

---

## T7: Python — --source 參數 ✅

**Files:** `scripts/luogu.py`

1. argparse 新增 `--source` 參數（type=str, default=None, choices=["luogu", "spoj"]）
2. `sync_content()` 修改：接受 `source` 參數，傳入 `get_problem_ids_missing_content(source=source)`
3. `show_missing_content_stats()` 修改：接受 `source` 參數，傳入 `count_missing_content(source=source)`
4. main() 中：
   - `source = args.source or "luogu"`
   - `sync_content()` 呼叫時傳入 `source`
   - `show_missing_content_stats()` 呼叫時傳入 `source`
5. `fetch_problem_content()` 已透過 pid 取 Luogu 頁面，SPOJ 的 pid 格式為 `SP{n}`，Luogu URL 格式 `luogu.com.cn/problem/SP{n}` 可正常運作，無需修改

**Acceptance:** `luogu.py --fill-missing-content --source spoj` 僅處理 source='spoj' 的行；不指定 --source 時行為不變.

---

## T8: Frontend — admin.js CRAWLER_CONFIG ✅

**Files:** `static/admin.js`

1. luogu 陣列新增：
   ```js
   { flag: '--training-list', i18nKey: 'training_list', type: 'text', placeholder: 'URL or ID' },
   { flag: '--source', i18nKey: 'source', type: 'select', options: ['luogu', 'spoj'] }
   ```
2. 新增 spoj key：
   ```js
   spoj: [
       { flag: '--sync-spoj', i18nKey: 'sync_spoj', type: 'checkbox' },
       { flag: '--rate-limit', i18nKey: 'rate_limit', type: 'number', placeholder: 'seconds', step: '0.1' },
       { flag: '--batch-size', i18nKey: 'batch_size', type: 'number', placeholder: '10', step: '1' }
   ]
   ```

**Acceptance:** Admin UI Luogu tab 顯示 training-list text 和 source select；SPOJ tab 顯示 sync-spoj checkbox.

---

## T9: Frontend — Templates ✅

**Files:** `templates/admin/crawlers.html`, `templates/admin/problems.html`, `templates/admin/embeddings.html`

1. crawlers.html: source-btn-group 新增 `<button type="button" class="source-btn" data-source="spoj">SPOJ</button>`
2. problems.html: source-btn-group 新增 SPOJ tab button
3. embeddings.html: source select 新增 `<option value="spoj">SPOJ</option>`

**Acceptance:** Admin 各頁面可見 SPOJ 選項.

---

## T10: Frontend — i18n ✅

**Files:** `static/i18n/en.json`, `static/i18n/zh-TW.json`, `static/i18n/zh-CN.json`

1. `sources` 新增 `"spoj": "SPOJ"`（三語相同）
2. `crawlers.flags` 新增：
   - `"training_list"`: "Training List" / "題單" / "题单"
   - `"sync_spoj"`: "Sync SPOJ" / "同步 SPOJ" / "同步 SPOJ"
   - `"source"`: "Source" / "資料來源" / "数据来源"

**Acceptance:** 切換三種語言時，新增的 flag labels 和 source name 正確顯示.

---

## Task Dependencies

```
T1 ─┐
T2 ─┤ (Rust arg plumbing, parallel)
T3 ─┤
T4 ─┘
     ↓
T5 ─┐
T6 ─┤ (Python implementation, parallel, depends on T1/T2)
T7 ─┘
     ↓
T8 ─┐
T9 ─┤ (Frontend, parallel, depends on T2)
T10─┘
```

T1-T4 可平行實作（Rust 端）。
T5-T7 可平行實作（Python 端），需 T1/T2 完成後才能透過 Admin 觸發測試。
T8-T10 可平行實作（Frontend），需 T2 完成後才能對應 source。
