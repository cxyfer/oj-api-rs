# Tasks: dual-progress-and-similar-redesign

## T1: Add `get_rewritten_content()` to `src/db/embeddings.rs`

```rust
pub fn get_rewritten_content(pool: &DbPool, source: &str, id: &str) -> Option<String>
```
- SQL: `SELECT rewritten_content FROM problem_embeddings WHERE source = ?1 AND problem_id = ?2`
- `NULL` / row not found → `None`
- Trim result; empty after trim → `None`
- Pattern: follow `get_embedding()` style (same file line 58)

## T2: Refactor `src/api/similar.rs` response format

### T2.1: Add structs
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

### T2.2: Update `similar_by_text` handler
- Replace `serde_json::Value` parsing (line ~195-209) with `serde_json::from_str::<EmbedTextOutput>(&stdout)`
- Extract `rewritten`: trim, empty → `None`
- Return `Json(SimilarResponse { rewritten_query, results })` instead of `Json(results)`

### T2.3: Update `similar_by_problem` handler
- After getting embedding, call `get_rewritten_content(&pool, &source, &id)` inside `spawn_blocking`
- Return `Json(SimilarResponse { rewritten_query, results })` instead of `Json(results)`

## T3: Update `static/admin.js` `updateEmbedProgressBar()`

Replace current if-else logic (line ~650-670) with:

```
function updateEmbedProgressBar(prog) {
    var bar = getElementById('embedding-progress-bar');
    if (!bar) return;

    if (prog.phase === 'completed') {
        bar.innerHTML = '<div class="progress-label">' + i18n.t('embeddings.progress.completed') + '</div>';
        return;
    }
    if (prog.phase === 'failed') {
        bar.innerHTML = '<div class="progress-label" style="color:var(--color-danger)">' + i18n.t('embeddings.progress.failed') + '</div>';
        return;
    }

    var html = '';

    // Rewriting bar
    if (prog.rewrite_progress && prog.rewrite_progress.total > 0) {
        var rp = prog.rewrite_progress;
        var pct = Math.min(Math.round((rp.done / rp.total) * 100), 100);
        var label = i18n.t('embeddings.progress.rewriting') + ': ' + rp.done + '/' + rp.total;
        if (rp.skipped > 0) {
            label += ' (' + i18n.t('embeddings.progress.skipped') + ': ' + rp.skipped + ')';
        }
        html += '<div class="progress-label">' + label + '</div>';
        html += '<div class="progress-bar"><div class="progress-fill" style="width:' + pct + '%"></div></div>';
    }

    // Embedding bar
    if (prog.embed_progress && prog.embed_progress.total > 0) {
        var ep = prog.embed_progress;
        var pct = Math.min(Math.round((ep.done / ep.total) * 100), 100);
        html += '<div class="progress-label">' + i18n.t('embeddings.progress.embedding') + ': ' + ep.done + '/' + ep.total + '</div>';
        html += '<div class="progress-bar"><div class="progress-fill" style="width:' + pct + '%"></div></div>';
    }

    bar.innerHTML = html;
}
```

## T4: Add i18n key `embeddings.progress.skipped`

| File | Key | Value |
|------|-----|-------|
| `static/i18n/en.json` | `embeddings.progress.skipped` | `"Skipped"` |
| `static/i18n/zh-TW.json` | `embeddings.progress.skipped` | `"已跳過"` |
| `static/i18n/zh-CN.json` | `embeddings.progress.skipped` | `"已跳过"` |

## Execution Order

```
T1 (DB function) → T2 (API refactor, depends on T1)
T3 (frontend) — independent, can parallel with T1+T2
T4 (i18n) — independent, can parallel with T1+T2
```
