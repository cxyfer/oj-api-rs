# Spec: Token Auth Toggle

## Requirements

### R2.1: Settings Table
- `app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)` created on startup.
- Default row: `('token_auth_enabled', '1')`.

### R2.2: Runtime Toggle (AtomicBool)
- `AppState.token_auth_enabled: AtomicBool` initialized from DB on startup.
- `bearer_auth` middleware: `load(Acquire)` → if false, skip auth, call `next.run()`.
- If true, proceed with existing token validation.

### R2.3: Toggle API
- GET `/admin/api/settings/token-auth` → `{ "enabled": bool }`.
- PUT `/admin/api/settings/token-auth` with `{ "enabled": bool }` → write DB → `store(Release)` → `{ "enabled": bool }`.

### R2.4: Toggle UI
- Tokens page displays a toggle switch reflecting current state.
- Toggle fires PUT on change, shows toast on success/failure.

## PBT Properties

### P2.1: Toggle Persistence
- **Invariant**: After toggle + server restart simulation (re-read DB), state matches last toggle.
- **Falsification**: Toggle to OFF, read DB, assert value = "0". Toggle to ON, read DB, assert value = "1".

### P2.2: Toggle OFF Bypasses Auth
- **Invariant**: When `token_auth_enabled = false`, any request to `/api/v1/*` without Bearer token returns 200 (not 401).
- **Falsification**: Set toggle OFF, send request without Authorization header, assert status != 401.

### P2.3: Toggle ON Enforces Auth
- **Invariant**: When `token_auth_enabled = true`, request to `/api/v1/*` without Bearer token returns 401.
- **Falsification**: Set toggle ON, send request without Authorization header, assert 401.

### P2.4: DB-First Update
- **Invariant**: AtomicBool is only updated after DB write succeeds.
- **Falsification**: If DB write fails (e.g., read-only), AtomicBool retains old value.
