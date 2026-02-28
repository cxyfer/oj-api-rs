## Context

現有系統已支援 LeetCode、AtCoder、Codeforces、Luogu 四個 OJ 的爬蟲。Luogu 爬蟲（`scripts/luogu.py`）支援全量同步和內容補全。使用者需要：
1. 從 Luogu 題單 (training list) 爬取特定題目集合
2. 從 Luogu 的 SP 子題庫爬取 SPOJ 題目，以 `source='spoj'` 存入 DB

## Goals / Non-Goals

**Goals:**
- luogu.py 新增 `--training-list <url_or_id>` 爬取指定題單
- luogu.py 新增 `--sync-spoj` 爬取 Luogu SP 子題庫，存為 `source='spoj'`
- luogu.py 新增 `--source` 參數支援 `--fill-missing-content` 指定目標 source
- Rust 端新增 `CrawlerSource::Spoj` + `SPOJ_ARGS` 白名單
- detect.rs 將 `SP\d+` 從 luogu 重新路由到 spoj
- Admin UI 新增 spoj tab、luogu tab 新增 training-list/source 欄位
- i18n 新增對應翻譯鍵

**Non-Goals:**
- 不修改 DB schema
- 不重構 LuoguClient 類別結構
- 不新增獨立的 spoj.py 腳本

## Decisions

### D1: Architecture — CrawlerSource::Spoj 獨立枚舉 + 共用 luogu.py

新增 `CrawlerSource::Spoj` 變體，`script_name()` 回傳 `"luogu.py"`。Spoj 擁有獨立的 `SPOJ_ARGS` 白名單。

- **選擇理由**：清楚的 source 語義（DB PK、API 篩選、UI tab 對應），同時避免腳本重複
- **替代方案**：僅在 Luogu 下新增 `--sync-spoj` subcommand — 但會導致 LUOGU_ARGS 膨脹且 API 篩選語義模糊
- **pattern 可擴展性**：未來 UVA 等透過 Luogu 閘道的 OJ 可依循相同模式

### D2: detect.rs SP\d+ 路由 — 直接改為 spoj

將 `SP\d+` 從 `LUOGU_ID_RE` 中抽出，在 Luogu ID regex 匹配前先檢查 SP 前綴。

- **LUOGU_ID_RE** 移除 `SP\d+`：改為 `^([PBTU]\d+|CF\d+[A-Z]|AT_(?:abc|arc|agc|ahc)\d+_[a-z]\d*|UVA\d+)$`
- **新增 SP 專用邏輯**：在 `detect_source()` 中 Luogu ID 匹配前加入 `if pid.starts_with("SP") && SP_RE.is_match(pid)` → return `("spoj", pid)`
- **Luogu URL 分支**：現有 URL handler 在偵測到 `SP` 開頭 pid 時，改為回傳 `("spoj", pid)`
- **選擇理由**：DB 目前無 SP 資料，無遷移負擔；早期確立正確路由避免日後資料不一致

### D3: Slug 推導策略

SPOJ 題目的 title 格式為 `"CODE - Name"`。

- `slug = title.split(" - ", 1)[0].strip()`；若無 `" - "` 分隔符，`slug = id`
- `title = title.split(" - ", 1)[1].strip()` 若有分隔符；否則保留原始 title
- `link = https://www.spoj.com/problems/{slug}/`
- **edge cases**：title 以 `" - "` 開頭 → slug 為空 → fallback 到 id

### D4: Progress 隔離

- SPOJ 使用 `spoj_progress.json`，獨立於 `luogu_progress.json`
- 結構與 `luogu_progress.json` 完全相同（completed_pages, last_completed_page, total_count_snapshot, last_updated）
- LuoguClient 的 `progress_file` 在 `__init__` 中根據操作模式設定

### D5: --source 參數設計

luogu.py 新增 `--source` CLI 參數：
- type=str, default=None
- 影響 `--fill-missing-content` 和 `--missing-content-stats` 的行為
- `--source spoj` → 查詢/更新 `source='spoj'` 的行
- `--source luogu` 或未指定 → 查詢/更新 `source='luogu'`（backward compatible）
- Rust 端 `LUOGU_ARGS` 新增 `--source { arity: 1, value_type: Str, ui_exposed: true }`

### D6: SPOJ_ARGS 白名單

```rust
pub static SPOJ_ARGS: &[ArgSpec] = &[
    ArgSpec { flag: "--sync-spoj", arity: 0, value_type: ValueType::None, ui_exposed: true },
    ArgSpec { flag: "--rate-limit", arity: 1, value_type: ValueType::Float, ui_exposed: true },
    ArgSpec { flag: "--batch-size", arity: 1, value_type: ValueType::Int, ui_exposed: true },
    ArgSpec { flag: "--data-dir", arity: 1, value_type: ValueType::Str, ui_exposed: false },
    ArgSpec { flag: "--db-path", arity: 1, value_type: ValueType::Str, ui_exposed: false },
];
```

### D7: LUOGU_ARGS 擴展

新增兩個 ArgSpec：
```rust
ArgSpec { flag: "--training-list", arity: 1, value_type: ValueType::Str, ui_exposed: true },
ArgSpec { flag: "--source", arity: 1, value_type: ValueType::Str, ui_exposed: true },
```

### D8: Admin UI 變更

**admin.js CRAWLER_CONFIG:**
- luogu 新增：`{ flag: '--training-list', i18nKey: 'training_list', type: 'text', placeholder: 'URL or ID' }`
- luogu 新增：`{ flag: '--source', i18nKey: 'source', type: 'select', options: ['luogu', 'spoj'] }`
- 新增 spoj key：`[{ flag: '--sync-spoj', i18nKey: 'sync_spoj', type: 'checkbox' }, { flag: '--rate-limit', ... }, { flag: '--batch-size', ... }]`

**Templates:**
- crawlers.html: source-btn-group 新增 spoj button
- problems.html: source-btn-group 新增 spoj tab
- embeddings.html: source select 新增 spoj option

**i18n (en.json, zh-TW.json, zh-CN.json):**
- `sources.spoj`: "SPOJ" / "SPOJ" / "SPOJ"
- `crawlers.flags.training_list`: "Training List" / "題單" / "题单"
- `crawlers.flags.sync_spoj`: "Sync SPOJ" / "同步 SPOJ" / "同步 SPOJ"
- `crawlers.flags.source`: "Source" / "資料來源" / "数据来源"

### D9: SPOJ 難度對照

沿用 Luogu 的 DIFFICULTY_MAP (0-7)。i18n 的 difficulty labels 可直接複用 `luogu_0` ~ `luogu_7` 鍵值。

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|---|---|---|
| Luogu 反爬影響大量 SP 題目爬取（3674 題 × 內容補全） | 長時間爬取中斷 | spoj_progress.json 支援斷點續傳 + rate-limit |
| 題單 lentille-context 結構可能與題目列表頁不同 | 解析失敗 | C3 約束已記錄結構差異；fail fast + ERROR log |
| SPOJ title 格式不一致（無 " - " 分隔符） | slug 推導失敗 | D3 定義 fallback 到 id |
| 5 處同步遺漏 | 功能不完整 | PBT P7/P8 覆蓋驗證 |
| --source 參數擴展 luogu.py 複雜度 | 維護成本增加 | 僅影響 fill-missing-content/stats 兩個 code path |
