## ADDED Requirements

### Requirement: Luogu difficulty displays as color-coded badge
Luogu problem difficulty SHALL be rendered as a color-coded badge in both the problems table row and the detail modal. A shared helper `renderDifficultyBadge(source, difficulty)` SHALL produce the badge HTML for both locations. For `source === 'luogu'`, the badge class SHALL be `.badge-luogu-{N}` where N is the tier index (0–7) derived from the canonical difficulty string. For all other sources, existing badge behavior (`badge-easy`, `badge-medium`, `badge-hard`) SHALL be preserved unchanged.

**Tier index mapping** (canonical string from `scripts/luogu.py` DIFFICULTY_MAP):

| Index | Canonical value | CSS class | Background (15% opacity) | Text color |
|-------|----------------|-----------|--------------------------|-----------|
| 0 | `暂无评定` | `.badge-luogu-0` | `rgba(191,191,191,0.15)` | `#bfbfbf` |
| 1 | `入门` | `.badge-luogu-1` | `rgba(254,76,97,0.15)` | `#fe4c61` |
| 2 | `普及−` | `.badge-luogu-2` | `rgba(243,156,17,0.15)` | `#f39c11` |
| 3 | `普及/提高−` | `.badge-luogu-3` | `rgba(255,193,22,0.15)` | `#ffc116` |
| 4 | `普及+/提高` | `.badge-luogu-4` | `rgba(82,196,26,0.15)` | `#52c41a` |
| 5 | `提高+/省选−` | `.badge-luogu-5` | `rgba(52,152,219,0.15)` | `#3498db` |
| 6 | `省选/NOI−` | `.badge-luogu-6` | `rgba(157,61,207,0.15)` | `#9d3dcf` |
| 7 | `NOI/NOI+/CTSC` | `.badge-luogu-7` | `#0e1d69` | `#ffffff` |

The badge label SHALL be the i18n translation of key `problems.difficulty.luogu_{N}`. If the key is missing, the raw `difficulty` string SHALL be used as fallback. For an unknown Luogu difficulty string (not in the 8-tier mapping), the badge SHALL render with no tier class and display the raw value — it SHALL NOT throw a JavaScript error.

#### Scenario: Luogu table row shows color-coded badge
- **WHEN** the problems table renders a row where `p.source === 'luogu'` and `p.difficulty` is a known canonical value (e.g., `普及−`)
- **THEN** the difficulty cell SHALL contain `<span class="badge badge-luogu-2">` with the i18n label for tier 2

#### Scenario: Luogu detail modal shows color-coded badge
- **WHEN** admin opens the detail modal for a Luogu problem with `difficulty === '提高+/省选−'`
- **THEN** the meta section SHALL contain `<span class="badge badge-luogu-5">` with the i18n label for tier 5

#### Scenario: Table row and detail modal badges are consistent
- **WHEN** the same Luogu problem is viewed in the table row AND in the detail modal
- **THEN** both SHALL display the identical tier class and label for that difficulty value

#### Scenario: LeetCode badges are unaffected
- **WHEN** the problems table or detail modal renders a LeetCode problem with `difficulty === 'Easy'`
- **THEN** the badge class SHALL be `badge-easy` (unchanged from before this change)

#### Scenario: Unknown Luogu difficulty does not break rendering
- **WHEN** a Luogu problem has a `difficulty` value not in the 8-tier mapping
- **THEN** the badge SHALL render with class `badge` only (no tier class) and display the raw difficulty string
- **AND** no JavaScript error SHALL be thrown

#### Scenario: PBT — all 8 canonical tiers produce distinct, valid badge classes
- **WHEN** `renderDifficultyBadge('luogu', d)` is called for each of the 8 canonical difficulty values
- **THEN** each call SHALL produce a unique class from the set `{badge-luogu-0 … badge-luogu-7}` with no duplicates and no out-of-range index

#### Scenario: PBT — i18n key coverage across all locales
- **WHEN** `i18n.t('problems.difficulty.luogu_N')` is called for N in 0..7 in each of en, zh-TW, zh-CN
- **THEN** all 24 lookups SHALL return a non-empty, non-key string (i.e., never returns the key itself)
