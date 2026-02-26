## 1. CSS — Luogu Badge Classes

- [x] 1.1 In `static/admin.css`, after the existing `.badge-hard` rule (line 83), add 8 new `.badge-luogu-{N}` rules (N=0..7) with the official Luogu colors using 15% opacity backgrounds matching existing badge style

## 2. CSS — Plain Text Content Style

- [x] 2.1 In `static/admin.css`, add `.detail-content-plain { white-space: pre-wrap; word-break: break-word; }` rule after the existing `.detail-content` block (around line 406)

## 3. i18n — Luogu Difficulty Keys

- [x] 3.1 In `static/i18n/en.json`, add keys `luogu_0` through `luogu_7` under `problems.difficulty` with English-readable labels (e.g., `"luogu_0": "Unrated"`, `"luogu_1": "Beginner"`, etc.)
- [x] 3.2 In `static/i18n/zh-TW.json`, add same 8 keys with Traditional Chinese labels matching the canonical difficulty strings
- [x] 3.3 In `static/i18n/zh-CN.json`, add same 8 keys with Simplified Chinese labels matching the canonical difficulty strings

## 4. HTML Template — Simplify Difficulty Select

- [x] 4.1 In `templates/admin/problems.html`, remove the static `<option>` elements for `easy`, `medium`, `hard` from `#problem-difficulty`, leaving only the "All Difficulties" default option (value="")

## 5. JS — Shared Helpers

- [x] 5.1 In `static/admin.js`, add a `LUOGU_DIFFICULTY_TIERS` constant array mapping the 8 canonical Chinese difficulty strings to their tier indices (0–7), placed before the Problems Page initialization block
- [x] 5.2 Add `renderDifficultyBadge(source, difficulty)` helper that returns badge HTML: for `source === 'luogu'` uses tier index → `.badge-luogu-{N}` + `luogu_N` i18n key; for other sources uses existing lowercase → `.badge-{lower}` + `problems.difficulty.{lower}` logic; falls back to raw string if no i18n key found
- [x] 5.3 Add `syncDifficultyFilterOptions(source)` helper that rebuilds `#problem-difficulty` options: for `source === 'leetcode'` injects 4 options (empty + easy/medium/hard); for `source === 'luogu'` injects 9 options (empty + 8 tier options with Chinese `value` and i18n label text); for other sources clears to only the empty option

## 6. JS — Replace Duplicate Badge Logic

- [x] 6.1 In `renderProblems()` (around line 1127–1135), replace the inline `difficultyBadge` construction with a call to `renderDifficultyBadge(p.source, p.difficulty)`
- [x] 6.2 In `showProblemDetail()` (around line 1282–1288), replace the inline difficulty badge construction in `metaHtml` with a call to `renderDifficultyBadge(p.source, p.difficulty)`

## 7. JS — Plain Text Content Rendering

- [x] 7.1 In `showProblemDetail()` (around line 1311–1314), replace `contentEl.innerHTML = content` with: if `p.source === 'leetcode'` set `innerHTML`; else set `textContent = content` and add class `detail-content-plain` to `contentEl`; ensure the class is removed when LeetCode content is rendered

## 8. JS — Luogu Difficulty Filter Visibility

- [x] 8.1 In `updateSourceVisibility(source)` (line 1087), change the `diffField` visibility condition from `source === 'leetcode'` to `source === 'leetcode' || source === 'luogu'`
- [x] 8.2 In `updateSourceVisibility(source)`, call `syncDifficultyFilterOptions(source)` after updating `diffField` visibility

## 9. JS — Initialization Order Fix

- [x] 9.1 In the initial load sequence (around lines 1415–1442), insert a call to `syncDifficultyFilterOptions(currentSource)` BEFORE the line `diffSelect.value = currentDifficulty` (line 1420), so Luogu options exist when URL state is restored

## 10. JS — Language Change Handler

- [x] 10.1 In the `languageChanged` event handler (line 1445–1448), after `updateTagsBtnText()`, call `syncDifficultyFilterOptions(currentSource)` then restore `diffSelect.value = currentDifficulty` to preserve the current selection with updated label text

## 11. CSS — Fix Badge-Luogu-7 Dark Theme Readability

- [x] 11.1 In `static/admin.css`, update `.badge-luogu-7` rule: change `background` from `rgba(14,29,105,0.15)` to `#0e1d69` (solid) and `color` from `#0e1d69` to `#ffffff`, so NOI/NOI+/CTSC badge is legible on the dark theme

## 12. CSS — Fix Difficulty Dropdown Option Text Visibility

- [x] 12.1 In `static/admin.css`, add rule `.filter-select option { background: var(--bg-surface); color: var(--color-text); }` after the `.filter-select:focus` line to ensure `<option>` elements inherit dark theme colors in all browsers

## 13. JS — Remove Rating Display for Luogu Source

- [x] 13.1 `updateSourceVisibility()` already excludes `'luogu'` from `showRating` (condition is `source === 'leetcode' || source === 'codeforces'`) — rating column already hidden for Luogu; no change needed
- [x] 13.2 In `showProblemDetail()` in `static/admin.js` (line 1327), added `p.source !== 'luogu'` guard: `if (p.rating && p.source !== 'luogu') rows += ...`

