## ADDED Requirements

### Requirement: Luogu tab shows source-specific difficulty filter
When the Luogu source tab is active, the difficulty filter field (`#difficulty-filter-field`) SHALL be visible and populated with Luogu-specific options. The options SHALL be injected by `syncDifficultyFilterOptions(source)`, which is called from `updateSourceVisibility(source)`. The `<select id="problem-difficulty">` in the HTML template SHALL contain only the "All Difficulties" default option; all source-specific options are managed exclusively by JS.

**Option structure for `source === 'luogu'`** (9 options total):

| `value` attribute | Displayed label (i18n key) |
|-------------------|---------------------------|
| `""` (empty) | `problems.difficulty_all` |
| `暂无评定` | `problems.difficulty.luogu_0` |
| `入门` | `problems.difficulty.luogu_1` |
| `普及−` | `problems.difficulty.luogu_2` |
| `普及/提高−` | `problems.difficulty.luogu_3` |
| `普及+/提高` | `problems.difficulty.luogu_4` |
| `提高+/省选−` | `problems.difficulty.luogu_5` |
| `省选/NOI−` | `problems.difficulty.luogu_6` |
| `NOI/NOI+/CTSC` | `problems.difficulty.luogu_7` |

**Option structure for `source === 'leetcode'`** (4 options total):
`""` / `easy` / `medium` / `hard` (same as current behavior)

For all other sources, `#difficulty-filter-field` SHALL be hidden (`display: none`).

Selecting a difficulty option SHALL cause `loadProblems()` to append `difficulty=<encoded_value>` to the request URL. The Chinese canonical strings SHALL be encoded with `encodeURIComponent` before appending. The backend SHALL receive and filter by the exact UTF-8 string via `GET /admin/api/problems/luogu?difficulty=<value>`.

When switching source tabs, `currentDifficulty` SHALL be reset to `''` before `syncDifficultyFilterOptions` runs, ensuring the filter always starts at "All Difficulties" on a new tab.

On page load with URL parameters `?source=luogu&difficulty=入门`, `syncDifficultyFilterOptions('luogu')` SHALL be called BEFORE `diffSelect.value = currentDifficulty` to ensure the option exists when the value is restored.

On `languageChanged` event, `syncDifficultyFilterOptions(currentSource)` SHALL rebuild options with translated labels, then restore `diffSelect.value` to the previously selected value.

#### Scenario: Luogu tab shows difficulty filter with 9 options
- **WHEN** user clicks the Luogu source tab
- **THEN** `#difficulty-filter-field` SHALL be visible
- **AND** `#problem-difficulty` SHALL contain exactly 9 options: one empty-value "All Difficulties" plus 8 Luogu tier options in canonical order

#### Scenario: Selecting a Luogu difficulty filters problems
- **WHEN** user selects `入门` from the Luogu difficulty dropdown
- **THEN** `loadProblems()` SHALL send a request to `/admin/api/problems/luogu?difficulty=%E5%85%A5%E9%97%A8` (URL-encoded `入门`)
- **AND** only `入门`-level problems SHALL appear in the table

#### Scenario: Switching away from Luogu resets filter
- **WHEN** user switches from the Luogu tab to any other source tab
- **THEN** `currentDifficulty` SHALL be reset to `''`
- **AND** `#difficulty-filter-field` SHALL be hidden (for non-LeetCode destinations) or show LeetCode options (for LeetCode destination)

#### Scenario: Switching to LeetCode tab restores LeetCode options
- **WHEN** user switches from the Luogu tab to the LeetCode tab
- **THEN** `#problem-difficulty` SHALL contain exactly 4 options: empty / easy / medium / hard

#### Scenario: URL state restore works for Luogu difficulty
- **WHEN** page loads with URL `?source=luogu&difficulty=入门`
- **THEN** after initialization, `#problem-difficulty` SHALL have `入门` selected
- **AND** `loadProblems()` SHALL fire with `difficulty=入门` in the request

#### Scenario: Language change preserves Luogu difficulty selection
- **WHEN** user has selected `普及−` from the Luogu difficulty filter
- **AND** user changes the UI language
- **THEN** `#problem-difficulty` SHALL still have value `普及−` after the language change
- **AND** the displayed label text SHALL be in the new language
- **AND** no additional problems request SHALL fire solely due to the language change (unless explicitly triggered)

#### Scenario: Non-LeetCode, non-Luogu source hides difficulty filter
- **WHEN** user clicks the AtCoder or Codeforces tab
- **THEN** `#difficulty-filter-field` SHALL have `display: none`

#### Scenario: PBT — Chinese difficulty value round-trips through URL encoding
- **WHEN** any of the 8 Luogu canonical difficulty strings `d` is set as `currentDifficulty`
- **AND** `loadProblems()` constructs the request URL
- **THEN** `decodeURIComponent(url.searchParams.get('difficulty')) === d`

#### Scenario: PBT — option count invariant
- **WHEN** `syncDifficultyFilterOptions('luogu')` is called
- **THEN** `document.getElementById('problem-difficulty').options.length === 9`
- **WHEN** `syncDifficultyFilterOptions('leetcode')` is called
- **THEN** `document.getElementById('problem-difficulty').options.length === 4`
