# Proposal: Admin UI/UX Improvements

## Context

The admin backend currently has three UI/UX issues affecting problem display quality:

1. **Newline rendering**: Non-LeetCode problem `content` fields contain `\n` literal characters that are not visually rendered as line breaks in the detail modal. LeetCode content is stored as HTML (renders correctly), but Codeforces, AtCoder, Luogu, UVa, SPOJ content is plain text.

2. **Luogu difficulty color coding**: Luogu difficulty labels are displayed as plain unstyled text. LeetCode difficulty uses `.badge-easy/.badge-medium/.badge-hard` CSS classes with color coding. Luogu has 8 difficulty tiers with official Luogu color assignments.

3. **Luogu difficulty filter**: The difficulty filter dropdown is hardcoded to Easy/Medium/Hard and hidden for all sources except LeetCode. Luogu needs a source-specific difficulty filter.

## Research Constraints

### Hard Constraints

- `difficulty` column is `TEXT` type in SQLite — no schema change needed; values are free-form strings from crawlers
- Luogu difficulty values (from `scripts/luogu.py` DIFFICULTY_MAP): `暂无评定`, `入门`, `普及−`, `普及/提高−`, `普及+/提高`, `提高+/省选−`, `省选/NOI−`, `NOI/NOI+/CTSC`
- DB difficulty filtering uses `LOWER(difficulty) = LOWER(?)` — case-insensitive exact match, already supports any string value
- Admin JS (`static/admin.js`) controls filter visibility via `updateSourceVisibility(source)` — `difficulty-filter-field` div shown only when `source === 'leetcode'`
- Detail modal content rendered via `innerHTML` in `#detail-content` div
- LeetCode content is HTML (rendered by innerHTML); other sources are plain text (rendered without whitespace preservation)
- `.badge-easy/.badge-medium/.badge-hard` CSS classes exist in `static/admin.css`; no Luogu badge classes exist

### Soft Constraints

- i18n keys follow pattern `problems.difficulty.{value}` — new Luogu difficulty keys must be added to all locale files
- Difficulty badge in detail modal: `'<span class="badge badge-' + lower + '">' + dLabel + '</span>'` — badge class uses lowercase difficulty value as suffix
- Table row difficulty badge: `badgeClass` derived from difficulty value mapping in `admin.js`
- Source tab visibility managed by CSS classes `.source-{name}` on `#problems-table`

### Dependencies

- `static/admin.js`: `updateSourceVisibility()`, `loadProblems()`, difficulty badge rendering (table rows + detail modal)
- `static/admin.css`: `.badge-*` CSS rules
- `templates/admin/problems.html`: `#difficulty-filter-field` `<select>` with hardcoded `<option>` elements
- `static/i18n/en.json`, `static/i18n/zh-TW.json`, `static/i18n/zh-CN.json`: i18n keys for difficulty labels

## Requirements

### REQ-1: Newline Rendering for Plain Text Content

**User Story**: As an admin, when I open the detail modal for a non-LeetCode problem, I can read the content with proper line breaks instead of seeing `\n` as literal characters.

**Acceptance Criteria**:
- When `content` is plain text (non-HTML), `\n` characters must be rendered as visible line breaks
- LeetCode HTML content rendering behavior must remain unchanged
- Detection of content type: check if `source === 'leetcode'` (and `source === 'luogu'` if Luogu uses HTML, otherwise treat as plain text)
- Implementation: apply CSS `white-space: pre-wrap` to `.detail-content` for non-HTML sources, or use JS to replace `\n` with `<br>` before setting `innerHTML`

**Scope**: `static/admin.js` (detail modal population logic)

---

### REQ-2: Luogu Difficulty Color Badges

**User Story**: As an admin, I can visually identify Luogu problem difficulty by color-coded badges matching Luogu's official color scheme.

**Acceptance Criteria**:
- 8 Luogu difficulty tiers rendered with distinct badge colors:

  | Difficulty | Official Color | CSS Class |
  |------------|---------------|-----------|
  | 暂无评定       | Gray `#bfbfbf`   | `.badge-luogu-0` |
  | 入门           | Red `#fe4c61`    | `.badge-luogu-1` |
  | 普及−          | Orange `#f39c11` | `.badge-luogu-2` |
  | 普及/提高−     | Yellow `#ffc116` | `.badge-luogu-3` |
  | 普及+/提高     | Green `#52c41a`  | `.badge-luogu-4` |
  | 提高+/省选−    | Blue `#3498db`   | `.badge-luogu-5` |
  | 省选/NOI−     | Purple `#9d3dcf` | `.badge-luogu-6` |
  | NOI/NOI+/CTSC | Black `#0e1d69`  | `.badge-luogu-7` |

- Badge display in both the problems table row and the detail modal
- Badge visual style matches existing LeetCode badge style (semi-transparent background + colored text)
- Non-Luogu sources unaffected

**Scope**: `static/admin.css` (new `.badge-luogu-*` rules), `static/admin.js` (difficulty-to-class mapping for Luogu source)

---

### REQ-3: Luogu Difficulty Filter Dropdown

**User Story**: As an admin, on the Luogu tab of the Problems page, I can filter problems by Luogu difficulty tier.

**Acceptance Criteria**:
- When source tab switches to `luogu`, the difficulty filter dropdown becomes visible with Luogu-specific options
- Dropdown options: "All Difficulties" + all 8 Luogu difficulty tiers (in canonical order)
- When source switches away from `luogu`, the difficulty filter hides (or resets to LeetCode options)
- Selecting a difficulty sends the exact Chinese string value as `difficulty` query parameter to `/admin/api/problems/luogu?difficulty=入门` (existing backend already handles this)
- Filter resets to "All" when switching sources
- i18n keys added for all 8 Luogu difficulty values

**Scope**: `templates/admin/problems.html` (new Luogu difficulty `<select>` or dynamic option injection), `static/admin.js` (`updateSourceVisibility()` extended for luogu), `static/i18n/*.json`

---

### REQ-4: Remove Luogu Rating Display

**User Story**: As an admin, the Luogu problem detail modal and table do not show a Rating field, since Luogu has no rating system.

**Acceptance Criteria**:
- Detail modal: Rating row is not rendered when `p.source === 'luogu'`
- Problems table: Rating column (`col-rating`) is hidden when source tab is `luogu`
- Other sources (LeetCode, Codeforces) rating display unaffected

**Scope**: `static/admin.js` (`showProblemDetail()` rating row guard, `updateSourceVisibility()` `showRating` condition)

---

### REQ-5: Fix Difficulty Dropdown Text Visibility (White-on-White)

**User Story**: As an admin, I can read difficulty dropdown options clearly on both LeetCode and Luogu tabs.

**Acceptance Criteria**:
- `<option>` elements inside `.filter-select` display readable text (dark background, light text) matching the dark theme
- Fix applies to all browsers where native `<option>` inherits system white background

**Scope**: `static/admin.css` (add `option` color/background rule scoped to `.filter-select`)

---

### REQ-6: NOI/NOI+/CTSC Badge Readability on Dark Theme

**User Story**: As an admin, I can clearly read the NOI/NOI+/CTSC difficulty badge text on the dark-themed admin UI.

**Acceptance Criteria**:
- `.badge-luogu-7` renders with white text (`#ffffff`) on a solid (non-transparent) dark blue background (`#0e1d69`)
- Badge remains visually distinct from other tiers
- Other badge tiers unaffected

**Scope**: `static/admin.css` (update `.badge-luogu-7` rule)

---

## Success Criteria

1. Opening detail modal for an AtCoder/Codeforces/Luogu/UVa/SPOJ problem with `\n` in content shows proper line breaks
2. Opening detail modal for a LeetCode problem still renders HTML correctly (no regression)
3. Luogu problems table shows color-coded difficulty badges using Luogu official colors
4. Luogu detail modal shows color-coded difficulty badge
5. Switching to the Luogu tab shows a difficulty dropdown with 8+ options
6. Selecting "入门" from the Luogu difficulty dropdown returns only 入门-level problems
7. Switching from Luogu tab to LeetCode tab resets difficulty filter to LeetCode options (Easy/Medium/Hard)
8. No existing LeetCode/AtCoder/Codeforces filtering behavior is broken
9. Luogu detail modal shows no Rating row; LeetCode/Codeforces still show Rating
10. Luogu problems table hides the Rating column
11. Difficulty dropdown options are readable (no white-on-white) in all browsers
12. NOI/NOI+/CTSC badge is clearly legible on the dark theme

## Out of Scope

- Backend API changes (existing `difficulty` filter parameter already works for Luogu)
- Database schema changes
- Python crawler changes
- Adding difficulty filter for AtCoder or Codeforces (neither has reliable difficulty data in current crawlers)
