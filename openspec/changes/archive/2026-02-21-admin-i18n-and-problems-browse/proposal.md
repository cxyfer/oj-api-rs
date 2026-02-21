# Admin Dashboard: i18n + Problems Browse Enhancement

## Context

The admin dashboard (Askama + vanilla JS) has all UI text hardcoded in English. The problems page only supports browsing via URL query parameter `?source=xxx` with minimal summary info and no way to view problem details inline.

**User need**: Add tri-lingual support (zh-TW / zh-CN / en) and enhance the problems browsing experience with source tabs + detail modal.

## Constraints

### Hard Constraints

- **HC-1**: Frontend is Askama (compile-time Rust templates) + vanilla JS + static CSS. No build step, no bundler, no framework.
- **HC-2**: Backend is axum 0.8 + rusqlite + r2d2 pool. All DB access goes through `spawn_blocking`.
- **HC-3**: `Problem` model fields: `id, source, slug, title, title_cn, difficulty, ac_rate, rating, contest, problem_index, tags, link, category, paid_only, content, content_cn, similar_questions`.
- **HC-4**: `ProblemSummary` model fields (used in admin list): `id, source, slug, title, title_cn, difficulty, ac_rate, rating, contest, problem_index, tags, link`.
- **HC-5**: Existing API: `GET /api/v1/problems/{source}/{id}` returns full `Problem`. `GET /api/v1/problems/{source}` returns paginated `ProblemSummary`.
- **HC-6**: Valid sources: `["atcoder", "leetcode", "codeforces"]` (defined in `src/api/problems.rs:33`).
- **HC-7**: Admin pages require session auth (`admin_session` cookie). Admin API routes use same auth.
- **HC-8**: Static files served from `static/` directory via `ServeDir`. Only `admin.css` and `admin.js` exist.
- **HC-9**: Template files in `templates/`: `base.html`, `admin/login.html`, `admin/index.html`, `admin/problems.html`, `admin/tokens.html`, `admin/crawlers.html`.

### Soft Constraints

- **SC-1**: Follow existing code style: vanilla JS (ES5-compatible IIFE pattern in admin.js), CSS custom properties in admin.css.
- **SC-2**: Reuse existing UI patterns: `source-btn` group (from crawlers page), `modal-overlay` (from tokens/crawlers), `badge` classes, `toast` function.
- **SC-3**: Language preference stored in `localStorage` (no backend round-trip needed).
- **SC-4**: i18n keys follow flat namespace: `"nav.dashboard"`, `"problems.title"`, etc.

## Requirements

### R1: i18n — Language Switcher & Translation Files

**Scenario**: User clicks language switcher in nav bar, all visible text updates instantly.

- Create JSON language files at `static/i18n/{locale}.json` for `zh-TW`, `zh-CN`, `en`.
- Add `data-i18n` attributes to all translatable text in Askama templates.
- Add a language switcher dropdown/select in `base.html` nav bar.
- JS loads the selected locale's JSON, replaces all `[data-i18n]` elements' `textContent`.
- Persist language preference in `localStorage('lang')`, default to `en`.
- On page load, apply saved language immediately (before content visible if possible).

### R2: Problems — Source Tab Buttons

**Scenario**: User clicks source tab (leetcode/codeforces/atcoder), problems list refreshes via AJAX without full page reload.

- Add a `source-btn-group` (Tab style) at the top of problems page with buttons for each source.
- Clicking a tab fetches the list from an admin API endpoint and re-renders the table body via JS.
- Maintain pagination state when switching sources (reset to page 1).
- Active tab highlighted with `.active` class (same pattern as crawlers page).
- URL query parameter `?source=xxx` should update for bookmarkability (using `history.replaceState`).

### R3: Problems — Detail Modal via API

**Scenario**: User clicks a "View" button on a problem row, a modal shows extended summary info fetched via API.

- Add a "View" button in the Actions column alongside the existing "Delete" button.
- On click, fetch problem detail from existing API: `GET /api/v1/problems/{source}/{id}`.
  - **Note**: This API requires bearer token when token auth is enabled. The admin frontend uses session auth, NOT bearer tokens. Need a new admin-scoped API endpoint or proxy.
- Display in a modal (reuse existing `.modal-overlay` pattern): tags, link, and all summary fields.
- No `content`/`content_cn` display (per user decision).
- Modal should be closable via X button, backdrop click, or Escape key.

### R4: Admin API — Problem Detail Endpoint

**Scenario**: Admin frontend needs to fetch a single problem's details without bearer token auth.

- Add admin API endpoint: `GET /admin/api/problems/{source}/{id}` — returns full `Problem` JSON.
- Auth: session cookie (same as other admin API routes).
- This avoids requiring the admin frontend to use bearer token auth for the public API.

## Dependencies

- R3 depends on R4 (need admin-scoped API before modal can fetch data).
- R2 depends on existing admin problems page handler (already exists as `GET /admin/problems?source=xxx&page=N`).
- R1 is independent.

## Success Criteria

1. Language switcher in nav bar allows switching between zh-TW, zh-CN, en.
2. All hardcoded text in admin templates has corresponding translations in all 3 locales.
3. Language preference persists across page navigations (localStorage).
4. Problems page shows source tab buttons; clicking changes the displayed source via AJAX.
5. Each problem row has a "View" button that opens a modal with extended info (tags, link, etc.).
6. Modal data is fetched from `GET /admin/api/problems/{source}/{id}` (session auth).
7. No regressions in existing admin functionality (tokens, crawlers, logout).

## Risks

- **R-RISK-1**: The `GET /api/v1/problems/{source}/{id}` public API requires bearer token auth when enabled. Admin uses session auth. Mitigation: create dedicated admin endpoint (R4).
- **R-RISK-2**: i18n JSON files must cover all text in all 5 templates + nav bar. Missing keys will show raw key strings. Mitigation: audit all templates systematically.

## Files to Create

| File | Purpose |
|------|---------|
| `static/i18n/en.json` | English translations |
| `static/i18n/zh-TW.json` | Traditional Chinese translations |
| `static/i18n/zh-CN.json` | Simplified Chinese translations |

## Files to Modify

| File | Change |
|------|--------|
| `templates/base.html` | Add language switcher in nav, add `data-i18n` attributes |
| `templates/admin/index.html` | Add `data-i18n` attributes to all text |
| `templates/admin/problems.html` | Add source tabs, "View" button, detail modal, `data-i18n` attributes |
| `templates/admin/tokens.html` | Add `data-i18n` attributes |
| `templates/admin/crawlers.html` | Add `data-i18n` attributes |
| `templates/admin/login.html` | Add `data-i18n` attributes |
| `static/admin.js` | i18n loader, language switcher logic, source tab AJAX, detail modal fetch |
| `static/admin.css` | Styles for language switcher, source tabs on problems page, detail modal |
| `src/admin/handlers.rs` | Add `get_problem` handler for admin API |
| `src/admin/mod.rs` | Register new admin API route |
