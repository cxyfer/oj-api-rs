# Spec: Admin Dashboard UI

## Requirements

### R3.1: Dark Theme
- All admin pages use cohesive dark theme via `static/admin.css`.
- Color palette defined as CSS variables on `:root`.
- Base template links to `/static/admin.css` and `/static/admin.js`.
- Login page is standalone (no nav bar).

### R3.2: Dashboard Page
- `/admin/` shows stat cards: total problems count, active tokens count, token auth status (enabled/disabled).
- Stats fetched server-side via Askama template.
- Cards link to respective management pages.

### R3.3: Tokens Page
- Top section: toggle switch for token auth with label showing current state.
- Create token form: optional label input + "Generate" button.
- On creation: modal/alert showing full token (one-time reveal) with copy button.
- Token list table: masked token, label, created date, last used, active status, revoke button.
- All mutations via AJAX (no full page reload).

### R3.4: Problems Page
- Dark theme applied to existing table.
- Pagination and badges preserved.
- Delete action via AJAX with confirmation.

### R3.5: Toast Notifications
- `static/admin.js` provides `toast(message, type)` function.
- Types: success (green), error (red), info (blue).
- Auto-dismiss after 3 seconds.
- Container: `<div id="toast-container">` in base.html.

### R3.6: AJAX Error Handling
- All AJAX calls check for 401 → redirect to `/admin/login`.
- Network errors show error toast.

## PBT Properties

### P3.1: Toggle UI Sync
- **Invariant**: Toggle switch visual state always matches `GET /admin/api/settings/token-auth` response.
- **Falsification**: Load page, compare toggle checked state with API response.

### P3.2: Token One-Time Display
- **Invariant**: Full token is only shown immediately after creation, never retrievable again from UI.
- **Falsification**: Create token, note full value, refresh page, assert token is masked in list.
