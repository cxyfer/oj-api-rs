# Proposal: Admin Dashboard with Token Toggle

## Context

The existing admin panel requires `x-admin-secret` header for every request, making it accessible only via cURL. Users need a browser-accessible admin dashboard with a modern dark-themed UI, session-based login, token CRUD operations, and a toggle to enable/disable API bearer-token authentication globally.

## Discovered Constraints

### Hard Constraints

- **Framework**: Axum 0.8 + Askama 0.12 templating. No external frontend frameworks.
- **Database**: SQLite via r2d2 + rusqlite. All persistence through `rw_pool`.
- **Auth separation**: Admin auth (`x-admin-secret` header) and API auth (`Bearer` token) are separate middleware in `src/auth/mod.rs`.
- **Static files**: Served from `static/` at `/static`. CSS/JS must go here.
- **Template inheritance**: Askama templates in `templates/` with `base.html` as layout.
- **Pool architecture**: Read-only pool (`ro_pool`) for queries, read-write pool (`rw_pool`) for mutations. Auth middleware has its own `AuthRwPool` Extension.
- **AppState**: Shared via `Arc<AppState>` as Axum state. Config is immutable after startup.

### Soft Constraints

- Code style: minimal, no unnecessary comments or docs.
- Existing nav uses `#1a1a2e` dark background.
- Admin routes are namespaced under `/admin/`.

### Dependencies

- `bearer_auth` middleware in `src/auth/mod.rs` must be modified to check the toggle state.
- `AppState` needs a new field for the runtime toggle (thread-safe).
- `db::mod.rs` needs a new `settings` table initialization.

## Requirements

### R1: Login Page & Session Auth

**Scenario**: User navigates to `/admin/` in browser → sees login form → enters ADMIN_SECRET → receives a session cookie → all subsequent admin page/API requests are authenticated via cookie.

- Login page at `/admin/login` (excluded from auth middleware).
- Session token stored as `HttpOnly` cookie (`oj_admin_session`).
- Session validated in admin auth middleware (replaces header-only check).
- Logout endpoint clears cookie.

### R2: Settings Table & Token Auth Toggle

**Scenario**: Admin clicks toggle switch on tokens page → POST to `/admin/api/settings/token-auth` → `app_settings` table updated → `AppState` runtime flag updated → API middleware checks flag before enforcing bearer auth.

- New `app_settings` table: `key TEXT PRIMARY KEY, value TEXT NOT NULL`.
- Key `token_auth_enabled` with value `"1"` (enabled) or `"0"` (disabled). Default: `"1"`.
- Runtime: `AtomicBool` in `AppState` synced on startup and on toggle.
- `bearer_auth` middleware: if toggle is off, skip token validation and pass through.

### R3: Token CRUD UI

**Scenario**: Admin visits `/admin/tokens` → sees token list + toggle switch + "Create Token" form → can create (with optional label), revoke tokens, and toggle auth enforcement.

- Reuse existing `list_tokens`, `create_token`, `revoke_token` handlers.
- Frontend JS: AJAX calls to existing `/admin/api/tokens` endpoints.
- Display token in full on creation (one-time reveal), masked in list.

### R4: Modern Dark Theme UI

**Scenario**: All admin pages use a cohesive dark theme consistent with nav bar `#1a1a2e`.

- Pure CSS in `static/admin.css` (no Tailwind/external deps).
- Dark background, light text, accent colors for actions.
- Responsive layout, card-based sections.
- Toast/notification for success/error feedback.
- All interactive operations via JS fetch (no full page reloads for mutations).

### R5: Dashboard Overview

**Scenario**: Admin visits `/admin/` → sees summary cards (total problems, active tokens, auth status) with navigation links.

## Success Criteria

1. Navigate to `/admin/` → redirected to `/admin/login` if not authenticated.
2. Login with correct ADMIN_SECRET → cookie set, redirected to dashboard.
3. Token list page shows all tokens with create/revoke actions working via AJAX.
4. Toggle switch changes token auth enforcement; change persists across server restart.
5. With toggle OFF, API calls without Bearer token succeed (200).
6. With toggle ON, API calls without Bearer token fail (401).
7. All admin pages render correctly in modern dark theme.
8. Existing API endpoints and health check remain unaffected.

## Files to Modify

| File | Change |
|------|--------|
| `src/auth/mod.rs` | Add cookie-based admin auth, conditional bearer auth |
| `src/admin/mod.rs` | Add login/logout routes, settings API routes |
| `src/admin/pages.rs` | Add login page handler, update templates with settings |
| `src/admin/handlers.rs` | Add settings toggle handler, login/logout handlers |
| `src/db/mod.rs` | Add `ensure_app_settings_table`, new `settings` module |
| `src/db/settings.rs` | New: get/set settings CRUD |
| `src/main.rs` | Add `AtomicBool` to AppState, init settings on startup |
| `src/models.rs` | (No change needed — settings are key-value) |
| `src/config.rs` | (No change needed) |
| `templates/base.html` | Update with dark theme, add logout link |
| `templates/admin/login.html` | New: login form |
| `templates/admin/index.html` | Redesign with dashboard cards |
| `templates/admin/tokens.html` | Redesign with toggle, AJAX create/revoke |
| `templates/admin/problems.html` | Redesign with dark theme |
| `static/admin.css` | New: dark theme stylesheet |
| `static/admin.js` | New: AJAX helpers, toast notifications |

## Out of Scope

- Rate limiting on login.
- Multi-user admin accounts.
- RBAC/permission levels.
- CSRF protection (admin is single-user, cookie is HttpOnly).
