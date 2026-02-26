## Context

Admin UI suffers from three display defects affecting non-LeetCode problems:
1. Plain text `content` fields render `\n` as literal characters instead of line breaks
2. Luogu difficulty labels appear as unstyled text (no color-coded badges)
3. Difficulty filter dropdown is hardcoded to LeetCode tiers and hidden for Luogu

All changes are **pure frontend** (JS + CSS + HTML template + i18n JSON). The Rust backend, SQLite schema, and Python crawlers require no modification — the existing `difficulty: Option<String>` API with case-insensitive filtering already handles arbitrary string values including Chinese.

## Goals / Non-Goals

**Goals:**
- Render non-LeetCode plain text content with visible line breaks
- Display Luogu difficulty as color-coded badges (8 tiers, official colors)
- Show a Luogu-specific difficulty filter dropdown when the Luogu tab is active
- Zero regressions on existing LeetCode/AtCoder/Codeforces behavior

**Non-Goals:**
- Backend API changes
- Difficulty filter support for AtCoder or Codeforces
- Luogu difficulty sort ordering (backend `ORDER BY` CASE does not cover Chinese strings)
- Adding difficulty filter for any source beyond LeetCode and Luogu

## Decisions

### D1: Content type detection — `source === 'leetcode'` only

**Decision**: Use `p.source === 'leetcode'` as the sole gate for HTML rendering. All other sources use `textContent` + CSS `white-space: pre-wrap`.

**Alternatives considered**:
- `isHtmlSource(source)` config map — adds indirection for a single-source case; add when a second HTML source exists
- Runtime HTML sniff (`/<[a-z]/i.test(content)`) — fragile, can misfire on problem text containing angle brackets

**Rationale**: Only LeetCode is confirmed HTML. Simple boolean reduces cognitive overhead and eliminates false-positive XSS surface.

### D2: Plain text rendering — `textContent` + `pre-wrap` (not `innerHTML` + `<br>`)

**Decision**: Set `contentEl.textContent = content` and add CSS class `.detail-content-plain { white-space: pre-wrap; word-break: break-word; }` for non-LeetCode sources.

**Alternatives considered**:
- `innerHTML = content.replace(/\n/g, '<br>')` — introduces XSS if content contains `<` or `&`; requires `esc()` first which double-encodes entities already in the string

**Rationale**: `textContent` is inherently XSS-safe. `pre-wrap` preserves both newlines and wrapping without markup transformation.

### D3: Luogu badge CSS naming — numeric index classes `.badge-luogu-0..7`

**Decision**: Map each canonical Luogu difficulty string to a numeric tier index (0–7); CSS class is `.badge-luogu-{index}`.

**Alternatives considered**:
- Direct slugification of Chinese strings (e.g., `.badge-普及-`) — CSS class names containing non-ASCII or `/` require escaping and are fragile across browsers
- Single `.badge-luogu` class with inline `style` color — mixes concerns; harder to override via theme

**Rationale**: Numeric indices are unambiguous, CSS-safe, and cleanly map to the canonical ordering defined in `scripts/luogu.py` DIFFICULTY_MAP.

**Tier mapping** (canonical string → index → official color):

| Index | Canonical value | Hex |
|-------|-----------------|-----|
| 0 | `暂无评定` | `#bfbfbf` |
| 1 | `入门` | `#fe4c61` |
| 2 | `普及−` | `#f39c11` |
| 3 | `普及/提高−` | `#ffc116` |
| 4 | `普及+/提高` | `#52c41a` |
| 5 | `提高+/省选−` | `#3498db` |
| 6 | `省选/NOI−` | `#9d3dcf` |
| 7 | `NOI/NOI+/CTSC` | `#0e1d69` |

### D4: Shared badge helper — `renderDifficultyBadge(source, difficulty)`

**Decision**: Extract a single helper used by both `renderProblems()` (table row) and `showProblemDetail()` (detail modal).

**Rationale**: Both locations currently duplicate identical badge-building logic (lines 1127–1135 and 1282–1288). Divergence there caused inconsistent display; a shared helper enforces a single code path.

### D5: Difficulty filter options — pure JS injection via `syncDifficultyFilterOptions(source)`

**Decision**: The `<select id="problem-difficulty">` in the template retains only the "All Difficulties" `<option>`. All source-specific options are injected by a new JS helper `syncDifficultyFilterOptions(source)` called from `updateSourceVisibility()`.

**Alternatives considered**:
- Keep LeetCode options in template HTML, inject only Luogu options — dual ownership of option state; harder to maintain when a third source needs a filter

**Rationale**: Single source of truth for all options. Template stays dumb; all filter state lives in JS. Scales cleanly if AtCoder/Codeforces ever gain difficulty data.

### D6: Source tab switch — hard reset difficulty to empty

**Decision**: When switching source tabs, `resetFilters()` (already called in `activateTab()`) clears `currentDifficulty` to `''`. `syncDifficultyFilterOptions(source)` rebuilds options first, then `diffSelect.value = currentDifficulty` (empty = "All").

**Rationale**: Cross-source difficulty values are semantically incompatible. Preserving a value across sources would silently send a nonsense query. Hard reset is safe and predictable.

### D7: Initialization order fix for URL state restore

**Decision**: In the initial load sequence, call `syncDifficultyFilterOptions(currentSource)` **before** `diffSelect.value = currentDifficulty` (line 1420). Without this, setting `value` on a select with no matching option is a no-op.

**Rationale**: Bug found during analysis — current code at line 1420 restores `diffSelect.value` before options exist for Luogu source.

### D8: i18n key scheme — `problems.difficulty.luogu_0` through `luogu_7`

**Decision**: Use ASCII-only keys `luogu_0..luogu_7` in all three locale files. The `value` attribute of Luogu `<option>` elements is the canonical Chinese string; the displayed label is `i18n.t('problems.difficulty.luogu_N')`.

**Alternatives considered**:
- Using Chinese strings as JSON keys — non-ASCII JSON keys are technically valid but problematic in some build toolchains and diff/search tooling

**Rationale**: Separation of display key (numeric, ASCII) from query value (Chinese canonical string) avoids encoding issues and keeps i18n lookup deterministic.

### D9: Language switch — rebuild Luogu options on `languageChanged`

**Decision**: The `languageChanged` event handler (line 1445) must also call `syncDifficultyFilterOptions(currentSource)` and re-apply the current selection. The injected `<option>` elements use runtime `i18n.t()` calls (not `data-i18n` attributes), so static i18n re-translation won't update them.

**Rationale**: Dynamic DOM elements are not covered by the static `data-i18n` scan that the i18n module performs on `languageChanged`.

## Risks / Trade-offs

| Risk | Mitigation |
|------|-----------|
| Luogu difficulty strings in DB contain encoding variants (e.g., different minus sign U+2212 vs U+002D) | JS mapping uses the exact canonical strings from `scripts/luogu.py` DIFFICULTY_MAP; unmapped values fall back to index `null` → no badge class → safe no-op display |
| `white-space: pre-wrap` on detail-content conflicts with LeetCode `<pre>` blocks | Class `.detail-content-plain` applied only when `source !== 'leetcode'`; existing `.detail-content pre` styles unaffected |
| rebuilding `<select>` options causes loss of `change` event listener | Only `option` children are replaced (not the `<select>` node itself), so event listeners on the select are preserved |
| Missing i18n key in one locale file shows raw key string | All three locale files updated in a single commit; PBT invariant verifies 8 keys × 3 files |

## Migration Plan

1. Deploy is a static file update (JS, CSS, HTML template, JSON). No server restart required if assets are served from disk; Docker rebuild required if embedded in binary.
2. No data migration.
3. Rollback: revert the three static files to prior commit.

## Open Questions

- None. All ambiguities resolved during multi-model analysis and user confirmation.
