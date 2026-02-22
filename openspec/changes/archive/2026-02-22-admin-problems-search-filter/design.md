# Design: Admin Problems Search & Filter

## 1. Backend Changes

### 1.1 Query Struct — `ListQuery` (`src/api/problems.rs`)

```rust
pub struct ListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub difficulty: Option<String>,
    pub tags: Option<String>,        // existing
    pub search: Option<String>,      // NEW
    pub sort_by: Option<String>,     // NEW: "id" | "difficulty" | "rating" | "ac_rate"
    pub sort_order: Option<String>,  // NEW: "asc" | "desc"
    pub tag_mode: Option<String>,    // NEW: "any" (default) | "all"
}
```

All new fields are `Option` — backward compatible.

### 1.2 DB Layer — `ListParams` + `list_problems()` (`src/db/problems.rs`)

```rust
pub struct ListParams<'a> {
    pub source: &'a str,
    pub page: u32,
    pub per_page: u32,
    pub difficulty: Option<&'a str>,
    pub tags: Option<Vec<&'a str>>,
    pub search: Option<&'a str>,       // NEW
    pub sort_by: Option<&'a str>,      // NEW (pre-validated)
    pub sort_order: Option<&'a str>,   // NEW (pre-validated)
    pub tag_mode: &'a str,             // NEW: "any" or "all"
}
```

**Search SQL pattern:**
```sql
AND (
    id LIKE ?{i} ESCAPE '\'
    OR COALESCE(title, '') LIKE ?{i} ESCAPE '\'
    OR COALESCE(title_cn, '') LIKE ?{i} ESCAPE '\'
)
```
- Bind value: `%<escaped>%` where escaped replaces `%` → `\%`, `_` → `\_`, `\` → `\\`.
- Single bind parameter reused for all three columns.

**Tag mode SQL:**
- `any` (OR): `(cond1 OR cond2 OR ...)` — current behavior
- `all` (AND): `(cond1 AND cond2 AND ...)` — change join operator

**Sort SQL:**
- Whitelist map (Rust `match`):
  - `"id"` → `id`, `"difficulty"` → `difficulty`, `"rating"` → `rating`, `"ac_rate"` → `ac_rate`
- Direction: `"asc"` → `ASC`, `"desc"` → `DESC`
- Concatenate as string literal into SQL (never user input).
- `difficulty` sort: `CASE WHEN LOWER(difficulty)='easy' THEN 1 WHEN LOWER(difficulty)='medium' THEN 2 WHEN LOWER(difficulty)='hard' THEN 3 ELSE 4 END`

### 1.3 Tags Endpoint — `list_tags()` (`src/db/problems.rs`)

```sql
SELECT DISTINCT LOWER(TRIM(je.value)) AS tag
FROM problems p, json_each(
    CASE WHEN p.tags IS NOT NULL AND p.tags != '' AND json_valid(p.tags)
         THEN p.tags ELSE '[]' END
) je
WHERE p.source = ?1 AND TRIM(je.value) != ''
ORDER BY tag ASC
```

Returns `Vec<String>`.

### 1.4 Validation (`src/api/problems.rs` handlers)

```rust
const VALID_SORT_BY: &[&str] = &["id", "difficulty", "rating", "ac_rate"];
const VALID_SORT_ORDER: &[&str] = &["asc", "desc"];
const VALID_TAG_MODES: &[&str] = &["any", "all"];
```

- If `sort_by` is `Some` but not in whitelist → return 400.
- If `sort_order` is `Some` but not in whitelist → return 400.
- If `tag_mode` is `Some` but not in whitelist → return 400.
- If `sort_order` is `Some` but `sort_by` is `None` → ignore `sort_order`.

### 1.5 Route Registration (`src/admin/mod.rs`, `src/api/mod.rs`)

- Admin: `GET /admin/api/tags/:source` → `handlers::get_tags_list`
- Public: `GET /api/v1/tags/:source` → new handler (same logic, different auth layer)

## 2. Frontend Changes

### 2.1 HTML Template (`templates/admin/problems.html`)

Insert filter bar between source tabs card and stats line:

```
[Source Tabs Card]
[Filter Bar Card] ← NEW
  Row 1: [Search Input] [Difficulty Dropdown] [Per-Page Dropdown]
  Row 2: [Tags Multi-Select Dropdown] [AND/OR Toggle]
[Stats Line]
[Table with sortable headers]
[Pagination]
```

### 2.2 JavaScript (`static/admin.js`)

**State variables:**
```js
var currentSearch = '';
var currentDifficulty = '';
var currentTags = [];
var currentTagMode = 'any';
var currentSortBy = '';
var currentSortOrder = '';
var currentPerPage = 50;
```

**Key functions:**
- `loadProblems()`: build query string from all state vars
- `loadTags(source)`: fetch tags from API, populate dropdown
- `debounce(fn, ms)`: utility for search input
- `parseUrlState()`: read URL query params into state vars on init
- `syncUrlState()`: write state vars to URL via replaceState
- `resetFilters()`: clear all filters (on source tab switch)

**Multi-select dropdown (vanilla JS):**
- Custom component: button that opens a dropdown panel with checkboxes
- Selected tags shown as comma-separated text on button, or count badge
- Click outside closes dropdown
- Keyboard: Enter/Space toggles checkbox, Escape closes

**Sort indicators:**
- Table headers get `cursor: pointer` and `data-sort` attribute
- Active sort column shows `▲` or `▼` suffix
- Click toggles: none → asc → desc → none

**Debounce:**
- 300ms delay on `input` event
- Immediate trigger on Enter key

### 2.3 CSS (`static/admin.css`)

New styles needed:
- `.filter-bar`: flex row with gap, wrapping on mobile
- `.search-input`: text input styled like existing inputs
- `.select-dropdown`: native select styled for dark theme
- `.multi-select`: custom multi-select container
- `.multi-select-btn`: trigger button
- `.multi-select-panel`: absolute positioned checkbox list
- `.multi-select-item`: checkbox + label row
- `.tag-mode-toggle`: small AND/OR toggle button
- `th[data-sort]`: pointer cursor + sort arrow pseudo-element
- `th.sort-asc::after`, `th.sort-desc::after`: arrow indicators

### 2.4 i18n Keys

```
problems.search_placeholder     → "Search by ID or title..."
problems.difficulty_all         → "All Difficulties"
problems.per_page               → "Per page"
problems.tag_mode.any           → "Match Any"
problems.tag_mode.all           → "Match All"
problems.tags_placeholder       → "Filter by tags..."
problems.tags_selected          → "{count} tags selected"
problems.no_results             → "No problems found"
```

## 3. PBT Properties

| # | Property | Invariant | Falsification |
|---|----------|-----------|---------------|
| P1 | Search idempotency | Same search term always returns same results | Run same query twice, diff results |
| P2 | Tag mode partition | ANY results ⊇ ALL results for same tag set | Select 2+ tags, verify ANY count ≥ ALL count |
| P3 | Sort stability | Sorting by X asc then desc returns exact reverse (for unique values) | Sort rating asc, reverse, compare with rating desc |
| P4 | Pagination totality | Sum of all pages' item counts = meta.total | Iterate all pages, count items |
| P5 | Filter composition | search ∩ difficulty ∩ tags ⊆ unfiltered results | Apply all filters, verify each result matches all criteria |
| P6 | URL round-trip | Serialize state → URL → parse URL → state matches original | Generate random filter combos, serialize/parse, compare |
| P7 | Empty search = no filter | search="" returns same results as no search parameter | Compare total counts |
| P8 | Sort whitelist enforcement | sort_by=invalid → 400 | Send random strings as sort_by |
| P9 | Tag listing completeness | Every tag in list_tags() appears on at least one problem | For each tag, query with that tag, verify count > 0 |
| P10 | Case insensitivity | search="ABC" and search="abc" return same results | Compare result sets |
