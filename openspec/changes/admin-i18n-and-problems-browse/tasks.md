# Implementation Tasks

## 1. Backend - Admin API Endpoints

- [x] 1.1 Add `get_problems_list` handler in `src/admin/handlers.rs` for `GET /admin/api/problems/{source}`
- [x] 1.2 Add `get_problem_detail` handler in `src/admin/handlers.rs` for `GET /admin/api/problems/{source}/{id}`
- [x] 1.3 Implement source validation helper function (validate against `["atcoder", "leetcode", "codeforces"]`)
- [x] 1.4 Implement RFC7807 error response builder for admin API
- [x] 1.5 Add custom serializer for Problem to exclude `content` and `content_cn` fields
- [x] 1.6 Register new routes in `src/admin/mod.rs` with session auth middleware
- [x] 1.7 Ensure all handlers use `ro_pool` + `spawn_blocking` for database access

## 2. Frontend - i18n Infrastructure

- [x] 2.1 Create `static/i18n/en.json` with all translation keys (nav.*, problems.*, modal.*, common.*)
- [x] 2.2 Create `static/i18n/zh-TW.json` with Traditional Chinese translations
- [ ] 2.3 Create `static/i18n/zh-CN.json` with Simplified Chinese translations
- [x] 2.4 Add inline script in `templates/base.html` `<head>` to synchronously load locale from localStorage
- [x] 2.5 Add language switcher dropdown in `templates/base.html` navigation bar
- [x] 2.6 Implement i18n loader function in `static/admin.js` (loadLocale, applyTranslations)
- [x] 2.7 Implement language switcher event handler in `static/admin.js`

## 3. Frontend - Template i18n Attributes

- [ ] 3.1 Add `data-i18n` attributes to all text in `templates/base.html`
- [ ] 3.2 Add `data-i18n` attributes to all text in `templates/admin/index.html`
- [ ] 3.3 Add `data-i18n` attributes to all text in `templates/admin/problems.html`
- [ ] 3.4 Add `data-i18n` attributes to all text in `templates/admin/tokens.html`
- [ ] 3.5 Add `data-i18n` attributes to all text in `templates/admin/crawlers.html`
- [ ] 3.6 Add `data-i18n` attributes to all text in `templates/admin/login.html`

## 4. Frontend - Problems Page Source Tabs

- [ ] 4.1 Add source tab button group HTML in `templates/admin/problems.html`
- [ ] 4.2 Add CSS styles for source tabs in `static/admin.css` (reuse `.source-btn` pattern)
- [ ] 4.3 Implement source tab click handler in `static/admin.js` with request sequencing
- [ ] 4.4 Implement AJAX list fetch function with loading state management
- [ ] 4.5 Implement table body re-render function for problems list
- [ ] 4.6 Implement URL sync with `history.replaceState` on tab switch
- [ ] 4.7 Implement pagination reset to page 1 on source change

## 5. Frontend - Problem Detail Modal

- [ ] 5.1 Add modal HTML structure in `templates/admin/problems.html` (reuse `.modal-overlay` pattern)
- [ ] 5.2 Add "View" button in Actions column of problems table
- [ ] 5.3 Add CSS styles for detail modal in `static/admin.css`
- [ ] 5.4 Implement modal open handler in `static/admin.js`
- [ ] 5.5 Implement AJAX detail fetch function with loading/error states
- [ ] 5.6 Implement modal content render function (display tags, link, summary fields)
- [ ] 5.7 Implement modal close handlers (X button, backdrop click, Escape key)

## 6. Frontend - Error Handling

- [ ] 6.1 Implement toast error display for API failures (reuse existing `toast` function)
- [ ] 6.2 Implement session expiry detection (401 response) with redirect to login
- [ ] 6.3 Implement retry mechanism for network errors
- [ ] 6.4 Add error state display in modal for detail fetch failures

## 7. Frontend - Loading States

- [ ] 7.1 Add loading spinner HTML/CSS for tab switching
- [ ] 7.2 Implement button disabled state during AJAX requests
- [ ] 7.3 Add skeleton/loading state for modal content
- [ ] 7.4 Ensure loading indicators are properly removed on success/error

## 8. Frontend - Accessibility

- [ ] 8.1 Add ARIA attributes to source tabs (`role="tablist"`, `role="tab"`, `aria-selected`, `aria-controls`)
- [ ] 8.2 Add ARIA attributes to modal (`role="dialog"`, `aria-modal="true"`, `aria-labelledby`)
- [ ] 8.3 Implement keyboard navigation for tabs (Tab, Enter)
- [ ] 8.4 Implement Escape key handler for modal close
- [ ] 8.5 Verify color contrast meets WCAG AA standards in `static/admin.css`
- [ ] 8.6 Add focus management (trap focus in modal, restore focus on close)

## 9. Testing & Validation

- [ ] 9.1 Test admin list API with all query param combinations (page, per_page, difficulty, tags)
- [ ] 9.2 Test admin detail API with valid/invalid source and ID combinations
- [ ] 9.3 Test RFC7807 error responses for all error scenarios (400, 401, 404, 500)
- [ ] 9.4 Test i18n switching between all three locales (zh-TW, zh-CN, en)
- [ ] 9.5 Test localStorage persistence across page navigations
- [ ] 9.6 Test source tab switching with rapid clicks (race condition handling)
- [ ] 9.7 Test modal open/close with keyboard and mouse interactions
- [ ] 9.8 Test session expiry handling during AJAX requests
- [ ] 9.9 Verify no regressions in existing admin pages (tokens, crawlers, logout)
- [ ] 9.10 Test with screen reader for accessibility compliance

## 10. Documentation & Cleanup

- [ ] 10.1 Verify all translation keys are present in all three locale files
- [ ] 10.2 Verify all `data-i18n` attributes have corresponding translation keys
- [ ] 10.3 Remove any debug logging or console.log statements
- [ ] 10.4 Verify code follows existing style (ES5 IIFE pattern, CSS custom properties)
