# Design: Admin Dashboard with Token Toggle

## Architecture Decisions

### AD1: Session Management — In-Memory with RwLock

**Choice**: `tokio::sync::RwLock<HashMap<String, i64>>` in `AppState`, key = session token, value = expires_at timestamp.

**Rationale**: Single-machine deployment, single admin user. No external deps needed. Server restart = re-login (acceptable).

**Session lifecycle**:
- Login: CSPRNG 32-byte hex token → insert into map → `Set-Cookie: oj_admin_session=<token>; Path=/admin; HttpOnly; SameSite=Lax; Max-Age=28800`
- Validate: read cookie → lookup in map → check `expires_at > now`
- Logout: remove from map → clear cookie with `Max-Age=0`
- Expiry cleanup: lazy — remove on failed validation

**Fallback**: `x-admin-secret` header still accepted (backward compat with cURL).

### AD2: Token Auth Toggle — AtomicBool + SQLite

**Choice**: `std::sync::atomic::AtomicBool` in `AppState` for runtime, `app_settings` table for persistence.

**Ordering**: `store(Ordering::Release)` / `load(Ordering::Acquire)`.

**Flow**:
1. Startup: read `token_auth_enabled` from `app_settings` → init `AtomicBool`
2. Toggle API: write DB first → on success, update `AtomicBool`
3. `bearer_auth`: `load()` → if false, skip validation and call `next.run()`

### AD3: Admin Router Split

**Choice**: Split `admin_router()` into public (login) and protected (everything else) sub-routers.

```
/admin/login     GET/POST  → no auth middleware
/admin/logout    POST      → no auth middleware (cookie cleared)
/admin/*         GET/POST  → admin_auth middleware (cookie OR header)
/admin/api/*     *         → admin_auth middleware (cookie OR header)
```

### AD4: Cookie Auth Priority

When both `x-admin-secret` header and `oj_admin_session` cookie are present:
1. Check header first (backward compat)
2. If header absent/invalid, check cookie
3. If both fail → 401 for API routes, redirect to `/admin/login` for page routes

### AD5: Frontend — Pure CSS/JS Dark Theme

**No external dependencies**. Single `admin.css` + single `admin.js`.

**Color palette** (CSS variables):
```css
--bg-body: #0f0f1a;
--bg-surface: #1a1a2e;
--bg-hover: rgba(255,255,255,0.05);
--color-primary: #7c4dff;
--color-text: #e0e0e0;
--color-muted: #a0a0a0;
--color-success: #00c853;
--color-danger: #ff1744;
--color-border: rgba(255,255,255,0.1);
```

### AD6: Database Schema

```sql
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('token_auth_enabled', '1');
```

### AD7: AppState Changes

```rust
pub struct AppState {
    pub ro_pool: db::DbPool,
    pub rw_pool: db::DbPool,
    pub config: config::Config,
    pub crawler_lock: tokio::sync::Mutex<Option<models::CrawlerJob>>,
    pub embed_semaphore: Semaphore,
    // NEW
    pub token_auth_enabled: std::sync::atomic::AtomicBool,
    pub admin_sessions: tokio::sync::RwLock<std::collections::HashMap<String, i64>>,
}
```

## API Surface

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/admin/login` | none | Login page |
| POST | `/admin/login` | none | Validate secret, set cookie |
| POST | `/admin/logout` | none | Clear session + cookie |
| GET | `/admin/` | cookie/header | Dashboard |
| GET | `/admin/problems` | cookie/header | Problems list |
| GET | `/admin/tokens` | cookie/header | Tokens page |
| GET | `/admin/api/tokens` | cookie/header | List tokens JSON |
| POST | `/admin/api/tokens` | cookie/header | Create token |
| DELETE | `/admin/api/tokens/{token}` | cookie/header | Revoke token |
| GET | `/admin/api/settings/token-auth` | cookie/header | Get toggle state |
| PUT | `/admin/api/settings/token-auth` | cookie/header | Set toggle state |
| POST | `/admin/api/problems` | cookie/header | Create problem |
| PUT | `/admin/api/problems/{source}/{id}` | cookie/header | Update problem |
| DELETE | `/admin/api/problems/{source}/{id}` | cookie/header | Delete problem |
| POST | `/admin/api/crawlers/trigger` | cookie/header | Trigger crawler |
| GET | `/admin/api/crawlers/status` | cookie/header | Crawler status |

## File Changes

### New Files
- `src/db/settings.rs` — get/set settings CRUD
- `templates/admin/login.html` — login form (standalone, no base.html)
- `static/admin.css` — dark theme stylesheet
- `static/admin.js` — AJAX helpers, toast, interactions

### Modified Files
- `src/main.rs` — AppState fields, init settings, init sessions
- `src/auth/mod.rs` — cookie validation in admin_auth, toggle check in bearer_auth
- `src/admin/mod.rs` — split router, add login/logout/settings routes
- `src/admin/handlers.rs` — login/logout/settings handlers
- `src/admin/pages.rs` — login page, dashboard stats, pass toggle state to tokens
- `src/db/mod.rs` — add settings module, ensure_app_settings_table
- `templates/base.html` — dark theme, CSS/JS links, logout button
- `templates/admin/index.html` — dashboard cards
- `templates/admin/tokens.html` — toggle switch, AJAX create/revoke
- `templates/admin/problems.html` — dark theme update
