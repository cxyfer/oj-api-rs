# i18n Specification

## Purpose
TBD - created by archiving change admin-i18n-and-problems-browse. Update Purpose after archive.
## Requirements
### Requirement: Language Switcher in Navigation Bar
The system SHALL provide a language switcher in the navigation bar that allows users to select between zh-TW, zh-CN, and en locales.

#### Scenario: User selects a language from the switcher
- **WHEN** user clicks the language switcher and selects a locale
- **THEN** all visible text with `data-i18n` attributes updates to the selected language immediately

#### Scenario: Language preference persists across sessions
- **WHEN** user selects a language and navigates to another page
- **THEN** the selected language remains active and is loaded from localStorage

#### Scenario: Default language on first visit
- **WHEN** user visits the admin dashboard for the first time with no saved preference
- **THEN** the system defaults to English (en)

### Requirement: Translation Files
The system SHALL provide JSON translation files for all supported locales at `static/i18n/{locale}.json`.

#### Scenario: Translation file structure
- **WHEN** the system loads a translation file
- **THEN** the file MUST contain flat namespace keys following the pattern `nav.*`, `problems.*`, `modal.*`, `common.*`

#### Scenario: All UI text has translations
- **WHEN** the system renders any admin template
- **THEN** every translatable text element MUST have a corresponding key in all three locale files (zh-TW, zh-CN, en)

### Requirement: i18n Initialization
The system SHALL load and apply the saved language preference before content becomes visible to prevent FOUC (Flash of Unstyled Content).

#### Scenario: Synchronous language loading
- **WHEN** the page loads
- **THEN** an inline script in `<head>` MUST synchronously load the locale from localStorage and apply translations before DOM ready

#### Scenario: Missing translation key fallback
- **WHEN** a translation key is missing from the loaded locale file
- **THEN** the system SHALL display the raw key string (e.g., "nav.dashboard")

### Requirement: data-i18n Attributes
The system SHALL mark all translatable text elements with `data-i18n` attributes containing the translation key.

#### Scenario: Text replacement on language change
- **WHEN** the i18n system applies a locale
- **THEN** it SHALL replace the `textContent` of all elements with `[data-i18n]` attributes using the corresponding translation from the loaded JSON

#### Scenario: Nested translation keys
- **WHEN** an element has `data-i18n="nav.dashboard"`
- **THEN** the system SHALL look up the value at `translations["nav.dashboard"]` (flat structure, not nested objects)

