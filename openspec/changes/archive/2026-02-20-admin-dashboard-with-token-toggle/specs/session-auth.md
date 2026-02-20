# Spec: Session-Based Admin Auth

## Requirements

### R1.1: Login Page Rendering
- GET `/admin/login` returns HTML login form without auth middleware.
- Form contains a password input and submit button.
- If already authenticated (valid cookie), redirect to `/admin/`.

### R1.2: Login Submission
- POST `/admin/login` with `secret` form field.
- If `secret == config.admin_secret`: generate 32-byte hex session token, store in `admin_sessions` with TTL 8 hours, set cookie `oj_admin_session`, redirect 303 to `/admin/`.
- If mismatch: re-render login page with error message.

### R1.3: Cookie Validation in admin_auth
- Middleware checks `x-admin-secret` header first (backward compat).
- If header absent/invalid, check `oj_admin_session` cookie against `admin_sessions` map.
- If session found and not expired: pass through.
- If session expired: remove from map, respond 401 (API) or redirect to `/admin/login` (pages).
- If neither header nor cookie valid: respond 401 or redirect.

### R1.4: Logout
- POST `/admin/logout` removes session from map, sets cookie `Max-Age=0`, redirects to `/admin/login`.

## PBT Properties

### P1.1: Session Uniqueness
- **Invariant**: No two active sessions share the same token.
- **Falsification**: Generate N sessions, assert all tokens are unique.

### P1.2: Expired Sessions Reject
- **Invariant**: Any session with `expires_at < now` must be rejected.
- **Falsification**: Insert session with past timestamp, attempt access, assert 401.

### P1.3: Header Backward Compat
- **Invariant**: Valid `x-admin-secret` header always grants access, regardless of cookie state.
- **Falsification**: Send request with valid header and no cookie, assert 200.

### P1.4: Cookie Scoping
- **Invariant**: Cookie `Path=/admin` ensures it is not sent to `/api/v1/*` routes.
- **Falsification**: Set cookie, request `/api/v1/problems/leetcode`, assert no cookie in request headers.
