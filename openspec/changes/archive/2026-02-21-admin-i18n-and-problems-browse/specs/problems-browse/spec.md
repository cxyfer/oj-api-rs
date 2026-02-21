# Problems Browse Specification

## ADDED Requirements

### Requirement: Source Tab Buttons
The system SHALL provide tab buttons for each problem source (leetcode, codeforces, atcoder) at the top of the problems page.

#### Scenario: User clicks a source tab
- **WHEN** user clicks a source tab button
- **THEN** the system SHALL fetch the problems list for that source via AJAX and re-render the table body without full page reload

#### Scenario: Active tab highlighting
- **WHEN** a source tab is selected
- **THEN** the tab SHALL have the `.active` CSS class applied and other tabs SHALL NOT have the `.active` class

#### Scenario: URL synchronization on tab switch
- **WHEN** user switches to a different source tab
- **THEN** the URL query parameter `?source=xxx` SHALL be updated using `history.replaceState` (not pushState)

#### Scenario: Pagination reset on source change
- **WHEN** user switches to a different source tab
- **THEN** the page SHALL reset to page 1 before fetching the new source's data

### Requirement: Problems List AJAX Loading
The system SHALL fetch problems list data from the admin API endpoint when switching sources or pages.

#### Scenario: Loading state during fetch
- **WHEN** a list fetch is in progress
- **THEN** the source tab buttons SHALL be disabled and a loading indicator SHALL be visible

#### Scenario: Successful list fetch
- **WHEN** the API returns a successful response with `{data: [ProblemSummary], meta: {...}}`
- **THEN** the system SHALL render the problems table with the returned data and update pagination controls

#### Scenario: Error handling for list fetch
- **WHEN** the API returns an error response
- **THEN** the system SHALL display an error toast message and keep the previous data visible

#### Scenario: Session expiry during fetch
- **WHEN** the API returns a 401 Unauthorized response
- **THEN** the system SHALL redirect to the login page

### Requirement: Request Sequencing for Race Conditions
The system SHALL handle rapid source tab switching by tracking request sequence numbers to prevent stale responses from overwriting newer data.

#### Scenario: Rapid tab switching
- **WHEN** user rapidly clicks multiple source tabs in succession
- **THEN** only the response from the most recent request SHALL update the UI state

#### Scenario: Out-of-order response arrival
- **WHEN** an older request's response arrives after a newer request's response
- **THEN** the older response SHALL be ignored and SHALL NOT update the UI

### Requirement: Problem Detail Modal
The system SHALL provide a "View" button in each problem row's Actions column that opens a modal with extended problem information.

#### Scenario: User clicks View button
- **WHEN** user clicks the "View" button on a problem row
- **THEN** a modal SHALL open with a loading state while fetching problem details

#### Scenario: Modal displays problem details
- **WHEN** the detail API returns successfully
- **THEN** the modal SHALL display: problem ID, source, slug, title, title_cn, difficulty, ac_rate, rating, contest, problem_index, tags, and link

#### Scenario: Modal does not display content fields
- **WHEN** the detail API response includes `content` or `content_cn` fields
- **THEN** these fields SHALL NOT be displayed in the modal

#### Scenario: Modal error handling
- **WHEN** the detail API returns an error (e.g., 404 Not Found)
- **THEN** the modal SHALL display an error message with a retry button

#### Scenario: Modal closing
- **WHEN** user clicks the X button, clicks the backdrop, or presses Escape key
- **THEN** the modal SHALL close and remove the `.modal-overlay` element

### Requirement: Accessibility for Problems Browse
The system SHALL implement accessibility features for keyboard navigation and screen readers.

#### Scenario: Keyboard navigation for tabs
- **WHEN** user presses Tab key
- **THEN** focus SHALL move between source tab buttons, and pressing Enter SHALL activate the focused tab

#### Scenario: Keyboard navigation for modal
- **WHEN** modal is open and user presses Escape key
- **THEN** the modal SHALL close

#### Scenario: ARIA attributes for tabs
- **WHEN** the page renders source tabs
- **THEN** tabs SHALL have appropriate ARIA attributes: `role="tablist"`, `role="tab"`, `aria-selected`, `aria-controls`

#### Scenario: ARIA attributes for modal
- **WHEN** the modal opens
- **THEN** it SHALL have `role="dialog"`, `aria-modal="true"`, and `aria-labelledby` pointing to the modal title

## Property-Based Testing Properties

### Property: Idempotency of Source Selection
**INVARIANT**: Selecting the same source tab repeatedly (without data changes) returns identical problem lists and pagination metadata.

**FALSIFICATION STRATEGY**: Generate random source selections, call list endpoint twice for same source/page, and diff returned IDs and metadata.

### Property: Round-trip URL State
**INVARIANT**: Source/page URL sync is round-trippable: UI→URL→UI preserves source, and after source switch page is restored as 1.

**FALSIFICATION STRATEGY**: Generate random source/page states, apply serializer/parser with tab-switch transitions, and compare reconstructed UI state to expected normalized state.

### Property: Modal State Exclusivity
**INVARIANT**: Modal state is mutually exclusive: exactly one of `loading`, `error`, or `content` is active at any time.

**FALSIFICATION STRATEGY**: Generate event traces (open modal, fetch start/success/fail, retry) and assert state machine invariants at each transition.

### Property: Request Sequence Monotonicity
**INVARIANT**: Applied response sequence number is non-decreasing; a stale (older) response can never overwrite state from a newer request.

**FALSIFICATION STRATEGY**: Generate concurrent requests with randomized latency and out-of-order completions; assert committed seq always equals max accepted seq.

### Property: Pagination Bounds
**INVARIANT**: UI bounds hold: page is never < 1, and accepted stale-response count is always 0.

**FALSIFICATION STRATEGY**: Fuzz URL params and rapid interaction patterns (tab thrash + pagination), inject delayed responses, and assert page lower bound plus stale-drop counter.
