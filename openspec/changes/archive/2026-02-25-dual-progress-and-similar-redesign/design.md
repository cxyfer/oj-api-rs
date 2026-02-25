# Design: dual-progress-and-similar-redesign

## R1: Dual Progress Bar

### Decision
前端 `updateEmbedProgressBar()` 改為同時渲染兩條進度條,不再根據 `phase` 做互斥切換.

### Constraints
- Python/Rust 後端零改動 — 資料已齊備
- `rewrite_progress` 和 `embed_progress` 各自獨立渲染,只要 `total > 0` 就顯示
- Rewriting label 格式: `Rewriting: {done}/{total} (Skipped: {skipped})`
- Embedding label 格式: `Embedding: {done}/{total}`
- `total == 0` 時不渲染該條進度條
- `phase` 為 `completed`/`failed` 時僅顯示狀態標籤（維持現行為）
- 百分比計算: `total <= 0 ? 0 : clamp(done/total * 100, 0, 100)`
- i18n: 新增 `embeddings.progress.skipped` key（三語言）

### Edge Cases
- `embed_progress.total` 可能隨 `rewrite_skipped` 增加而縮小 — 信任後端數值,不在 JS 重算
- Pipeline 失敗時 progress JSON 可能只有 `{phase:"failed"}` — 兩條進度條都不渲染

## R2: Similar API Response Wrapper

### Decision
兩個端點統一回傳 `SimilarResponse { rewritten_query: Option<String>, results: Vec<SimilarResult> }`.

### New Structs (similar.rs)

```rust
#[derive(Serialize)]
struct SimilarResponse {
    rewritten_query: Option<String>,
    results: Vec<SimilarResult>,
}

#[derive(Deserialize)]
struct EmbedTextOutput {
    embedding: Vec<f32>,
    rewritten: Option<String>,
}
```

### New DB Function (db/embeddings.rs)

```rust
pub fn get_rewritten_content(pool: &DbPool, source: &str, id: &str) -> Option<String>
// SQL: SELECT rewritten_content FROM problem_embeddings WHERE source = ?1 AND problem_id = ?2
// Normalize: NULL / blank after trim → None
```

### Data Flow Changes

**similar_by_text**:
1. 解析 Python stdout 為 `EmbedTextOutput`（取代現行 `serde_json::Value` 手動取值）
2. 擷取 `rewritten` 欄位,trim 後空字串轉 `None`
3. 用 `embedding` 做 KNN 搜尋
4. 回傳 `SimilarResponse { rewritten_query, results }`

**similar_by_problem**:
1. 取得 embedding 做 KNN 搜尋（不變）
2. 呼叫 `get_rewritten_content(pool, source, id)` 取得改寫文字
3. 回傳 `SimilarResponse { rewritten_query, results }`

### Edge Cases
- Python 回傳 `rewritten` 為 null/空 → `rewritten_query: null`
- DB 無 `problem_embeddings` 記錄 → `rewritten_query: null`
- Python subprocess 失敗 → 維持現有 502/504 錯誤處理
- 無搜尋結果 → `{ rewritten_query: "...", results: [] }`
