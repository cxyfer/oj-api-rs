## ADDED Requirements

### Requirement: Plain text content renders line breaks visually
The admin detail modal SHALL render non-LeetCode problem content with visible line breaks. When `p.source !== 'leetcode'`, the content SHALL be set via `element.textContent` and the container SHALL have CSS `white-space: pre-wrap; word-break: break-word` applied. When `p.source === 'leetcode'`, content SHALL be set via `element.innerHTML` (existing behavior, unchanged).

#### Scenario: Non-LeetCode content with newlines displays line breaks
- **WHEN** admin opens the detail modal for a problem where `source` is not `leetcode` (e.g., `codeforces`, `atcoder`, `luogu`, `uva`, `spoj`)
- **AND** the `content` field contains one or more `\n` characters
- **THEN** each `\n` SHALL appear as a visible line break in the rendered modal

#### Scenario: Plain text content with HTML-special characters is not parsed as HTML
- **WHEN** admin opens the detail modal for a non-LeetCode problem
- **AND** the `content` field contains characters such as `<`, `>`, or `&`
- **THEN** those characters SHALL be displayed as literal text, NOT interpreted as HTML tags or entities

#### Scenario: LeetCode HTML content is unaffected
- **WHEN** admin opens the detail modal for a LeetCode problem
- **THEN** the content SHALL be rendered as HTML (via `innerHTML`), preserving formatting, lists, and code blocks exactly as before this change

#### Scenario: Language selection applies before rendering mode
- **WHEN** `i18n.getLanguage() !== 'en'` AND `p.content_cn` is present
- **THEN** `content_cn` SHALL be selected as the content string
- **AND** the plain/HTML rendering mode SHALL be determined by `p.source`, independent of which language field was selected

#### Scenario: PBT — round-trip invariant for plain text
- **WHEN** any arbitrary string `s` (including `\n`, `<`, `>`, `&`, Unicode) is set as `content` for a non-LeetCode problem
- **THEN** `contentElement.textContent === s` after rendering (no characters lost, escaped, or injected)
