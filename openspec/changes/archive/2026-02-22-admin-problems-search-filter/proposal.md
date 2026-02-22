# Proposal: Admin Problems Search & Filter

## Context

Admin problems browse page (`/admin/problems`) currently only supports source tab switching and pagination. Users need to search problems by ID/title and filter by tags/difficulty, with sortable columns and per-page control. These capabilities should also be exposed through the public API.

## Requirements

### R1: Unified Search
- Single `search` query parameter matches against `id`, `title`, and `title_cn` using case-insensitive LIKE.
- Debounce 300ms on frontend; empty/whitespace-only search treated as no filter.
- LIKE wildcards (`%`, `_`, `\`) in user input must be escaped.

### R2: Tags Multi-Select Filter
- New API endpoint `GET /api/v1/tags/{source}` returns all distinct tags for a given source, normalized to lowercase, sorted alphabetically.
- Admin endpoint mirrors at `GET /admin/api/tags/{source}` (same handler, different auth layer).
- Frontend renders a multi-select dropdown populated from this endpoint on source change.
- `tag_mode` query parameter: `any` (default, OR logic) or `all` (AND logic).
- Frontend toggle button next to tags dropdown switches between ANY/ALL modes.

### R3: Difficulty Dropdown
- Frontend dropdown with options: All (default), Easy, Medium, Hard.
- Backend already supports `difficulty` query parameter — no backend change needed.

### R4: Sortable Columns
- New `sort_by` query parameter with whitelist: `id`, `difficulty`, `rating`, `ac_rate`.
- New `sort_order` query parameter with whitelist: `asc`, `desc`.
- Default: `id asc` when neither parameter is provided.
- Invalid values return HTTP 400.
- Frontend: clickable table headers with sort direction indicator (arrow).

### R5: Per-Page Selector
- Frontend selector with options: 20, 50, 100.
- Backend already supports `per_page` with clamp(1, 100) — no backend change needed.

### R6: URL State Sync
- All filter/sort/pagination state serialized to URL query string via `history.replaceState`.
- On page load, parse URL query string to restore filter state.
- Source tab switch resets search, tags, difficulty, sort, and page to defaults.

### R7: Public API Parity
- `ListQuery` struct extended with `search`, `sort_by`, `sort_order`, `tag_mode` for both public and admin APIs.
- All new parameters are optional with backward-compatible defaults.

## Success Criteria

1. Typing "two-sum" in search box shows only problems with "two-sum" in id or title (within 300ms debounce).
2. Selecting tags "dp" + "greedy" in ALL mode shows only problems tagged with both; switching to ANY mode shows problems tagged with either.
3. Difficulty dropdown filters correctly and combines with search + tags.
4. Clicking "Rating" header sorts by rating desc; clicking again toggles to asc.
5. Switching per_page from 50 to 20 reloads with correct pagination.
6. Copying URL with filters applied and opening in new tab restores exact same view.
7. `GET /api/v1/problems/leetcode?search=sum&sort_by=rating&sort_order=desc` returns correct results.
8. `GET /api/v1/problems/leetcode?sort_by=invalid` returns HTTP 400.
9. All UI text available in en, zh-TW, zh-CN.
