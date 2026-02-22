# Tasks: Admin Problems Search & Filter

- [x] Task 1: DB Layer — `list_tags()` function
- [x] Task 2: DB Layer — Extend `ListParams` and `list_problems()`
- [x] Task 3: API Layer — Extend `ListQuery` and add validation
- [x] Task 4: Public API Handler — Integrate new params
- [x] Task 5: Admin Handler — Integrate new params + tags endpoint
- [x] Task 6: Frontend HTML — Filter bar UI
- [x] Task 7: Frontend JS — Search, filter, sort, URL sync logic
- [x] Task 8: Frontend CSS — Filter bar and sort indicator styles
- [x] Task 9: i18n — Add translation keys

## Task 1: DB Layer — `list_tags()` function

**File:** `src/db/problems.rs`

Add `list_tags(pool: &DbPool, source: &str) -> Option<Vec<String>>`:
- SQL: `SELECT DISTINCT LOWER(TRIM(je.value)) AS tag FROM problems p, json_each(CASE WHEN p.tags IS NOT NULL AND p.tags != '' AND json_valid(p.tags) THEN p.tags ELSE '[]' END) je WHERE p.source = ?1 AND TRIM(je.value) != '' ORDER BY tag ASC`
- Return `Option<Vec<String>>`

**Verification:** Unit test with known data returns expected deduplicated lowercase tags.

---

## Task 2: DB Layer — Extend `ListParams` and `list_problems()`

**File:** `src/db/problems.rs`

2a. Add fields to `ListParams`:
- `search: Option<&'a str>`
- `sort_by: Option<&'a str>` (pre-validated by handler)
- `sort_order: Option<&'a str>` (pre-validated by handler)
- `tag_mode: &'a str` (default `"any"`)

2b. In `list_problems()`:
- **Search clause:** When `search` is `Some` and non-empty, add `AND (id LIKE ?{i} ESCAPE '\' OR COALESCE(title,'') LIKE ?{i} ESCAPE '\' OR COALESCE(title_cn,'') LIKE ?{i} ESCAPE '\')`. Bind value: `%<escaped>%`. Escape function: `\` → `\\`, `%` → `\%`, `_` → `\_`. Use same bind index for all 3 (single `?{i}` reused — actually need to bind 3 times with same value since rusqlite positional params can't reuse).
- **Tag mode:** Change `tag_conditions.join(" OR ")` to use `params.tag_mode`: `"any"` → `" OR "`, `"all"` → `" AND "`.
- **Sort:** Replace hardcoded `ORDER BY id ASC` with dynamic:
  ```rust
  let order_col = match params.sort_by {
      Some("difficulty") => "CASE WHEN LOWER(difficulty)='easy' THEN 1 WHEN LOWER(difficulty)='medium' THEN 2 WHEN LOWER(difficulty)='hard' THEN 3 ELSE 4 END",
      Some("rating") => "rating",
      Some("ac_rate") => "ac_rate",
      _ => "id",
  };
  let order_dir = match params.sort_order {
      Some("desc") => "DESC",
      _ => "ASC",
  };
  // Use: ORDER BY {order_col} {order_dir}, id ASC
  // Secondary sort by id for stability
  ```

**Verification:** PBT P1, P2, P3, P4, P5, P7, P10.

---

## Task 3: API Layer — Extend `ListQuery` and add validation

**File:** `src/api/problems.rs`

3a. Add fields to `ListQuery`:
```rust
pub search: Option<String>,
pub sort_by: Option<String>,
pub sort_order: Option<String>,
pub tag_mode: Option<String>,
```

3b. Add validation constants:
```rust
pub(crate) const VALID_SORT_BY: &[&str] = &["id", "difficulty", "rating", "ac_rate"];
pub(crate) const VALID_SORT_ORDER: &[&str] = &["asc", "desc"];
pub(crate) const VALID_TAG_MODES: &[&str] = &["any", "all"];
```

3c. Add validation helper:
```rust
pub(crate) fn validate_list_query(query: &ListQuery) -> Result<(), String> {
    if let Some(ref s) = query.sort_by {
        if !VALID_SORT_BY.contains(&s.as_str()) {
            return Err(format!("invalid sort_by: {}", s));
        }
    }
    if let Some(ref s) = query.sort_order {
        if !VALID_SORT_ORDER.contains(&s.as_str()) {
            return Err(format!("invalid sort_order: {}", s));
        }
    }
    if let Some(ref s) = query.tag_mode {
        if !VALID_TAG_MODES.contains(&s.as_str()) {
            return Err(format!("invalid tag_mode: {}", s));
        }
    }
    Ok(())
}
```

**Verification:** PBT P8.

---

## Task 4: Public API Handler — Integrate new params

**File:** `src/api/problems.rs`

Update `list_problems()` handler:
- Call `validate_list_query(&query)` → return 400 on Err.
- Pass `search`, `sort_by`, `sort_order`, `tag_mode` to `ListParams`.

Add public tags endpoint:
```rust
pub async fn list_tags(...) -> impl IntoResponse { ... }
```

**File:** `src/api/mod.rs`
- Register `GET /api/v1/tags/:source` route.

**Verification:** PBT P8, P9.

---

## Task 5: Admin Handler — Integrate new params + tags endpoint

**File:** `src/admin/handlers.rs`

Update `get_problems_list()`:
- Call `validate_list_query(&query)` → return 400 on Err.
- Pass `search`, `sort_by`, `sort_order`, `tag_mode` to `ListParams`.

Add admin tags endpoint:
```rust
pub async fn get_tags_list(...) -> impl IntoResponse { ... }
```

**File:** `src/admin/mod.rs`
- Register `GET /admin/api/tags/:source` route.

**Verification:** Manual test via curl.

---

## Task 6: Frontend HTML — Filter bar UI

**File:** `templates/admin/problems.html`

Insert between source tabs card and stats line:

```html
<div class="card filter-bar" style="margin-bottom:1rem;padding:1rem 1.25rem">
    <div class="filter-row">
        <div class="form-group" style="flex:1;min-width:200px">
            <input type="text" id="problem-search" placeholder="..." data-i18n-placeholder="problems.search_placeholder">
        </div>
        <div class="form-group">
            <select id="problem-difficulty">
                <option value="" data-i18n="problems.difficulty_all">All</option>
                <option value="easy" data-i18n="problems.difficulty.easy">Easy</option>
                <option value="medium" data-i18n="problems.difficulty.medium">Medium</option>
                <option value="hard" data-i18n="problems.difficulty.hard">Hard</option>
            </select>
        </div>
        <div class="form-group">
            <select id="problem-per-page">
                <option value="20">20</option>
                <option value="50" selected>50</option>
                <option value="100">100</option>
            </select>
        </div>
    </div>
    <div class="filter-row" style="margin-top:0.5rem">
        <div class="form-group" style="flex:1;position:relative">
            <div class="multi-select" id="tags-select">
                <button class="multi-select-btn" type="button" id="tags-select-btn" data-i18n="problems.tags_placeholder">Filter by tags...</button>
                <div class="multi-select-panel" id="tags-panel"></div>
            </div>
        </div>
        <button class="btn btn-sm tag-mode-toggle" id="tag-mode-btn" type="button">OR</button>
    </div>
</div>
```

Update table headers to be sortable:
```html
<th data-sort="id">...</th>
<th>Title</th>  <!-- not sortable -->
<th data-sort="difficulty">...</th>
<th data-sort="rating">...</th>
<th data-sort="ac_rate">...</th>
```

**Verification:** Visual inspection, i18n check.

---

## Task 7: Frontend JS — Search, filter, sort, URL sync logic

**File:** `static/admin.js`

7a. State variables and debounce utility at top of problems block.

7b. `loadProblems()`: build query string from all state vars:
```
/admin/api/problems/{source}?page={p}&per_page={pp}&search={s}&difficulty={d}&tags={t}&tag_mode={m}&sort_by={sb}&sort_order={so}
```
Only include non-empty params.

7c. `loadTags(source)`: fetch `GET /admin/api/tags/{source}`, render checkboxes in `#tags-panel`.

7d. Search input: `addEventListener('input', debounce(fn, 300))` + Enter key listener.

7e. Difficulty select: `addEventListener('change', ...)`.

7f. Per-page select: `addEventListener('change', ...)`.

7g. Multi-select dropdown:
- Toggle panel on button click
- Close on outside click
- Checkbox change updates `currentTags` array + triggers `loadProblems()`
- Button text shows selected count or placeholder

7h. Tag mode toggle:
- Click toggles between 'any'/'all'
- Button text: "OR" / "AND"
- Triggers `loadProblems()`

7i. Sortable headers:
- Click handler on `th[data-sort]`
- Cycle: none → asc → desc → none
- Update `currentSortBy` / `currentSortOrder`
- Add/remove `.sort-asc` / `.sort-desc` classes
- Trigger `loadProblems()`

7j. URL state:
- `parseUrlState()`: on init, read `search/difficulty/tags/tag_mode/sort_by/sort_order/per_page/page` from URL
- `syncUrlState()`: called after every state change, writes to URL via `replaceState`
- Replace existing `history.replaceState` calls in pagination/source-tab with `syncUrlState()`

7k. Source tab switch: call `resetFilters()` then `loadTags(newSource)` then `loadProblems()`.

**Verification:** PBT P6, manual E2E.

---

## Task 8: Frontend CSS — Filter bar and sort indicator styles

**File:** `static/admin.css`

```css
.filter-bar { ... }
.filter-row { display: flex; gap: 0.5rem; align-items: flex-end; flex-wrap: wrap; }
select { /* dark theme select */ }
.multi-select { position: relative; }
.multi-select-btn { /* styled like input */ }
.multi-select-panel { position: absolute; z-index: 100; max-height: 250px; overflow-y: auto; ... }
.multi-select-item { display: flex; align-items: center; gap: 0.4rem; padding: 0.3rem 0.5rem; }
.multi-select-item:hover { background: var(--bg-hover); }
.tag-mode-toggle { min-width: 3rem; }
th[data-sort] { cursor: pointer; user-select: none; }
th.sort-asc::after { content: ' ▲'; font-size: 0.7em; }
th.sort-desc::after { content: ' ▼'; font-size: 0.7em; }
```

Mobile: `.filter-row` wrap to column on `max-width: 768px`.

**Verification:** Visual inspection on desktop + mobile viewport.

---

## Task 9: i18n — Add translation keys

**Files:** `static/i18n/en.json`, `static/i18n/zh-TW.json`, `static/i18n/zh-CN.json`

New keys under `problems`:
| Key | en | zh-TW | zh-CN |
|-----|-----|-------|-------|
| search_placeholder | Search by ID or title... | 搜尋 ID 或標題... | 搜索 ID 或标题... |
| difficulty_all | All Difficulties | 所有難度 | 所有难度 |
| per_page | Per page | 每頁 | 每页 |
| tag_mode.any | Match Any | 符合任一 | 匹配任一 |
| tag_mode.all | Match All | 符合全部 | 匹配全部 |
| tags_placeholder | Filter by tags... | 依標籤過濾... | 按标签筛选... |
| tags_selected | {count} tags selected | 已選 {count} 個標籤 | 已选 {count} 个标签 |
| no_results | No problems found | 找不到符合的題目 | 未找到符合的题目 |

**Verification:** Switch language in UI, verify all new strings render correctly.

---

## Execution Order

```
Task 1 (DB list_tags)  ──┐
Task 2 (DB list_problems)├──→ Task 3 (API ListQuery) ──→ Task 4 (Public handler)
                         │                             ──→ Task 5 (Admin handler)
                         │
Task 9 (i18n)  ──────────┤
Task 8 (CSS)   ──────────┼──→ Task 6 (HTML template) ──→ Task 7 (JS logic)
                         │
```

Tasks 1, 2, 8, 9 can run in parallel.
Task 3 depends on 1, 2.
Tasks 4, 5 depend on 3.
Task 6 depends on 8, 9.
Task 7 depends on 4, 5, 6.
