# Change: dual-progress-and-similar-redesign

## Context

Embedding pipeline 有兩個階段（rewriting → vectorization），但前端進度條根據 `phase` 只顯示其中一個。Python 端已回傳雙階段進度，前端未利用。

Similar API 回傳純陣列 `[SimilarResult]`，無法攜帶 rewritten query 等 metadata。使用者無法得知查詢被改寫成什麼，難以 debug 相關性問題。

## Requirements

### R1: 雙進度條同時顯示

**現狀**: `admin.js:updateEmbedProgressBar()` 根據 `prog.phase` 做 if-else，一次只渲染一條進度條。

**目標**: 同時渲染 rewriting 和 embedding 兩條進度條，反映 producer-consumer 的真實狀態。

**約束**:
- Python 端 (`embedding_cli.py:_update_progress`) 已同時回傳 `rewrite_progress` 和 `embed_progress`，無需改動
- Rust 端 (`admin/handlers.rs:read_progress_json`) 直接透傳 JSON，無需改動
- 僅需修改 `static/admin.js` 的 `updateEmbedProgressBar()` 函式
- 進度 JSON 結構: `{ phase, rewrite_progress: {done, total, skipped}, embed_progress: {done, total}, started_at }`
- completed/failed 狀態仍顯示單一狀態標籤

**驗收**:
- [ ] rewriting 進行中時，兩條進度條同時可見（embedding 可能為 0/N）
- [ ] embedding 追上 rewriting 時，兩條進度條數值各自正確
- [ ] completed/failed 狀態正常顯示

### R2: Similar API 回應格式改為物件包裝

**現狀**:
- `GET /api/v1/similar?q=...` → `[SimilarResult, ...]`
- `GET /api/v1/similar/{source}/{id}` → `[SimilarResult, ...]`

**目標**: 兩個端點統一改為：
```json
{
  "rewritten_query": "...",
  "results": [SimilarResult, ...]
}
```

**約束**:
- **Breaking change**（使用者已確認接受）
- `similar_by_text`: Python `--embed-text` 已回傳 `{"embedding": [...], "rewritten": "..."}`，Rust 目前丟棄 `rewritten` 欄位 → 改為擷取並回傳
- `similar_by_problem`: DB `problem_embeddings` 表有 `rewritten_content` 欄位 → 新增 DB 函式讀取，回傳該值
- `rewritten_query` 可為 `null`（例如 embedding 存在但無 rewritten_content 記錄）
- 回應結構體從 `Vec<SimilarResult>` 改為新的包裝結構體

**驗收**:
- [ ] `GET /api/v1/similar?q=...` 回傳 `{ rewritten_query: "...", results: [...] }`
- [ ] `GET /api/v1/similar/{source}/{id}` 回傳 `{ rewritten_query: "..." | null, results: [...] }`
- [ ] 無結果時回傳 `{ rewritten_query: "...", results: [] }`

## Affected Files

| File | Change |
|------|--------|
| `static/admin.js` | `updateEmbedProgressBar()` 改為同時渲染雙進度條 |
| `src/api/similar.rs` | 新增包裝結構體；兩個 handler 改為回傳包裝格式；擷取 rewritten 欄位 |
| `src/db/embeddings.rs` | 新增 `get_rewritten_content()` 函式 |

## Out of Scope

- Python 腳本不需改動
- Rust 進度端點不需改動
- i18n 翻譯檔（如需新增 key 則順帶處理）
