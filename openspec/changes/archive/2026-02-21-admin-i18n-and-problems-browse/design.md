# Admin Dashboard: i18n + Problems Browse Enhancement - Design

## Context

The admin dashboard currently has:
- All UI text hardcoded in English
- Problems page with minimal browsing (URL query `?source=xxx` only)
- Session-based authentication (`admin_session` cookie)
- Askama templates + vanilla JS + static CSS (no build step)
- Backend: axum 0.8 + rusqlite + r2d2 connection pools

**Current pain points:**
1. Non-English speakers cannot use the admin dashboard effectively
2. Problems browsing requires full page reloads and lacks detail view
3. Public API (`/api/v1/problems/*`) requires bearer token auth, incompatible with admin session auth

**Constraints:**
- No frontend framework or build step (vanilla JS ES5 IIFE pattern)
- Must maintain existing code style and patterns
- Cannot break existing admin functionality

## Goals / Non-Goals

**Goals:**
- Add tri-lingual support (zh-TW, zh-CN, en) with instant switching
- Enable AJAX-based source tab switching without page reloads
- Provide problem detail modal with extended info (tags, link, summary fields)
- Create admin-scoped API endpoints with session auth
- Maintain accessibility standards (keyboard nav, ARIA, WCAG AA contrast)

**Non-Goals:**
- Displaying problem content/content_cn in modal (explicitly excluded per user decision)
- Server-side rendering of translations (client-side only)
- Advanced filtering UI beyond difficulty/tags query params
- Backward compatibility with `x-admin-secret` header (session-only)

## Decisions

### D1: Dedicated Admin API Endpoints (Not Proxy Architecture)

**Decision**: Create new admin-scoped endpoints `GET /admin/api/problems/{source}` and `GET /admin/api/problems/{source}/{id}` with session authentication.

**Rationale**:
- **Chosen approach**: Dedicated admin endpoints
  - Clean separation of concerns (admin vs public API)
  - No bearer token complexity in admin frontend
  - Simpler observability and error handling
  - Direct database access with `ro_pool`

- **Rejected alternatives**:
  1. **Proxy architecture**: Admin endpoints internally call public API with server-side bearer injection
     - ❌ More indirection, harder to debug
     - ❌ Requires managing internal bearer tokens
     - ❌ Public API rate limits would affect admin

  2. **Unified auth middleware**: Allow either bearer or session on shared endpoints
     - ❌ Increases auth complexity and security risk
     - ❌ Harder to maintain separate access control policies
     - ❌ Potential for auth bypass vulnerabilities

**Implementation**:
- Add handlers in `src/admin/handlers.rs`
- Register routes in `src/admin/mod.rs`
- Reuse existing `db::problems::list_problems` and `db::problems::get_problem`
- Use `ro_pool` + `spawn_blocking` for all reads

### D2: API Response Format - Align with Public API

**Decision**: Admin list API returns `{data: [ProblemSummary], meta: {page, per_page, total, total_pages}}`, matching public API format.

**Rationale**:
- **Chosen approach**: Consistent format with public API
  - ✅ Easier to maintain (same response builder logic)
  - ✅ Frontend can reuse pagination logic
  - ✅ Clear contract for future API consumers

- **Rejected alternative**: Simplified format (direct array)
  - ❌ Loses pagination metadata
  - ❌ Frontend must track page state separately
  - ❌ Inconsistent with existing patterns

### D3: Detail API Payload - Exclude Content Fields

**Decision**: Admin detail API returns full `Problem` object but explicitly excludes `content` and `content_cn` fields.

**Rationale**:
- **Chosen approach**: Exclude content fields
  - ✅ Reduces payload size (content can be large)
  - ✅ Matches modal display requirements (no content shown)
  - ✅ Faster response times

- **Rejected alternatives**:
  1. **Include all fields**: Return complete Problem
     - ❌ Unnecessary bandwidth usage
     - ❌ Slower response times

  2. **Dedicated DTO**: Create `AdminProblemDetail` with only modal fields
     - ❌ More code to maintain
     - ❌ Loses flexibility if modal needs change

**Implementation**: Use serde `#[serde(skip_serializing)]` or custom serializer to exclude fields.

### D4: Advanced Filtering Support

**Decision**: Support `difficulty` and `tags` query parameters in admin list API.

**Rationale**:
- **Chosen approach**: Support filters
  - ✅ Aligns with public API capabilities
  - ✅ Provides flexibility for admin users
  - ✅ Minimal additional complexity (reuse existing filter logic)

- **Rejected alternative**: Source + page only
  - ❌ Limited admin functionality
  - ❌ Would require future enhancement anyway

### D5: Error Response Format - RFC7807

**Decision**: All admin API errors return RFC7807 Problem JSON format with `Content-Type: application/problem+json`.

**Rationale**:
- **Chosen approach**: RFC7807 standard
  - ✅ Consistent with public API
  - ✅ Machine-readable error structure
  - ✅ Industry standard for REST APIs

- **Rejected alternative**: Simple `{error: "message"}` format
  - ❌ Less structured
  - ❌ Harder to handle programmatically
  - ❌ Inconsistent with existing patterns

**Error codes**:
- `400`: Invalid source, invalid query params
- `401`: Missing/expired session
- `404`: Problem not found
- `500`: Database errors (no internal details exposed)

### D6: i18n Architecture - Client-Side with Flat Namespace

**Decision**: Use flat namespace JSON files (`nav.dashboard`, `problems.title`) loaded synchronously via inline script in `<head>`.

**Rationale**:
- **Chosen approach**: Flat namespace + inline sync loading
  - ✅ Simple key lookup (no nested object traversal)
  - ✅ Prevents FOUC (Flash of Unstyled Content)
  - ✅ No server-side rendering complexity
  - ✅ Easy to maintain and audit

- **Rejected alternatives**:
  1. **Nested namespace**: `{nav: {dashboard: "..."}}`
     - ❌ More complex lookup logic
     - ❌ Harder to validate completeness

  2. **Async loading**: Load i18n after page render
     - ❌ Causes visible text flashing
     - ❌ Poor UX

  3. **CSS hiding**: Hide body until i18n loads
     - ❌ Blank screen during load
     - ❌ Worse perceived performance

**Implementation**:
- JSON files at `static/i18n/{locale}.json`
- Inline script in `base.html` `<head>` loads from localStorage
- All templates get `data-i18n` attributes
- JS replaces `textContent` on locale change

### D7: Loading States - Comprehensive Coverage

**Decision**: Implement loading indicators for tab switching, modal opening, and button clicks.

**Rationale**:
- **Tab switching**: Show spinner, disable buttons during fetch
- **Modal**: Show skeleton/loading state before data arrives
- **Buttons**: Disable during request to prevent double-clicks

**Implementation**:
- Reuse existing `.spinner` CSS class
- Add `disabled` attribute to buttons during requests
- Modal shows loading state before API response

### D8: Error Handling Strategy

**Decision**: Three-tier error handling: toast notifications, session expiry redirect, and retry mechanism.

**Rationale**:
- **Toast notifications**: Use existing `toast()` function for transient errors
- **Session expiry**: Detect 401 responses and redirect to `/admin/login`
- **Retry mechanism**: Provide retry button in modal error state

**Implementation**:
- Centralized error handler in `admin.js`
- Check response status codes
- Display user-friendly messages (not raw API errors)

### D9: Accessibility Implementation

**Decision**: Implement keyboard navigation, ARIA attributes, and WCAG AA color contrast.

**Rationale**:
- **Keyboard navigation**: Tab, Enter, Escape for all interactive elements
- **ARIA attributes**: `role`, `aria-label`, `aria-hidden`, `aria-modal`, `aria-selected`
- **Color contrast**: Ensure all text meets WCAG AA standards (4.5:1 for normal text)

**Implementation**:
- Add event listeners for keyboard events
- Add ARIA attributes to tabs (`role="tablist"`, `role="tab"`)
- Add ARIA attributes to modal (`role="dialog"`, `aria-modal="true"`)
- Audit CSS colors for contrast compliance

### D10: Race Condition Handling - Request Sequencing

**Decision**: Track request sequence numbers and ignore out-of-order responses.

**Rationale**:
- **Chosen approach**: Sequence number tracking
  - ✅ Simple to implement (increment counter)
  - ✅ No need for AbortController polyfill (ES5 compat)
  - ✅ Works with rapid tab switching

- **Rejected alternatives**:
  1. **AbortController**: Cancel old requests
     - ❌ Requires polyfill for ES5
     - ❌ More complex error handling

  2. **Simple button disable**: Prevent rapid clicks
     - ❌ Doesn't handle programmatic state changes
     - ❌ Poor UX (user must wait)

**Implementation**:
```javascript
var requestSeq = 0;
function fetchProblems(source) {
  var currentSeq = ++requestSeq;
  fetch(url).then(function(data) {
    if (currentSeq === requestSeq) {
      // Only apply if this is still the latest request
      updateUI(data);
    }
  });
}
```

### D11: URL Synchronization - replaceState

**Decision**: Use `history.replaceState` (not `pushState`) to update URL when switching tabs.

**Rationale**:
- **Chosen approach**: `replaceState`
  - ✅ Doesn't pollute browser history
  - ✅ Back button returns to previous page (not previous tab)
  - ✅ URL remains bookmarkable

- **Rejected alternative**: `pushState`
  - ❌ Each tab switch adds history entry
  - ❌ Back button cycles through tabs (annoying UX)

### D12: Pagination Behavior on Source Change

**Decision**: Reset to page 1 when switching source tabs.

**Rationale**:
- **Chosen approach**: Reset to page 1
  - ✅ Prevents empty results (different sources have different page counts)
  - ✅ Matches user expectations
  - ✅ Simpler state management

- **Rejected alternative**: Preserve page number
  - ❌ May land on empty page
  - ❌ Confusing UX

## Risks / Trade-offs

### R1: i18n Translation Completeness
**Risk**: Missing translation keys will display raw key strings (e.g., "nav.dashboard").

**Mitigation**:
- Systematic audit of all templates for `data-i18n` attributes
- Validation script to check key coverage across all locale files
- Fallback to English if key missing in selected locale

### R2: SQLite Contention Under Load
**Risk**: Concurrent admin operations and browse reads may cause SQLite busy errors.

**Mitigation**:
- Use `ro_pool` (not `rw_pool`) for all read operations
- Set appropriate busy timeout in connection config
- Monitor for `SQLITE_BUSY` errors in production

### R3: Request Sequencing Edge Cases
**Risk**: Sequence number overflow (unlikely but possible after 2^53 requests).

**Mitigation**:
- Use JavaScript's safe integer range (Number.MAX_SAFE_INTEGER)
- Reset counter on page load (not persistent)
- Overflow would take years of continuous rapid clicking

### R4: Accessibility Compliance
**Risk**: Cannot guarantee full WCAG compliance without manual testing.

**Mitigation**:
- Implement keyboard navigation and ARIA attributes
- Audit color contrast programmatically
- Document that full compliance requires manual testing with assistive technologies

### R5: Browser Compatibility
**Risk**: ES5 IIFE pattern may not work in very old browsers.

**Mitigation**:
- Target modern browsers (last 2 versions)
- No polyfills for ancient IE versions
- Admin dashboard is internal tool (controlled environment)

## Migration Plan

### Deployment Steps

1. **Backend deployment**:
   - Deploy new admin API endpoints
   - No database migrations required (read-only operations)
   - Backward compatible (existing admin pages still work)

2. **Frontend deployment**:
   - Add i18n JSON files to `static/i18n/`
   - Update templates with `data-i18n` attributes
   - Update `admin.js` and `admin.css`
   - No breaking changes to existing functionality

3. **Verification**:
   - Test language switching in all admin pages
   - Test source tab switching and pagination
   - Test modal opening and error states
   - Verify session expiry handling

### Rollback Strategy

- **Backend**: Remove new routes from `src/admin/mod.rs` (no data changes)
- **Frontend**: Revert template and static file changes
- **Zero data loss**: All changes are read-only

### Feature Flags

Not required - changes are additive and backward compatible.

## Open Questions

None - all ambiguities resolved through multi-model analysis and user decisions.
