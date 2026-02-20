# Tasks: Admin Dashboard with Token Toggle

## Task 1: Database — Settings Module

**Files**: `src/db/settings.rs` (new), `src/db/mod.rs`

1. Create `src/db/settings.rs`:
   - `pub fn get_setting(pool: &DbPool, key: &str) -> Option<String>` — SELECT value FROM app_settings WHERE key = ?1
   - `pub fn set_setting(pool: &DbPool, key: &str, value: &str) -> bool` — INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2), return success
   - `pub fn get_token_auth_enabled(pool: &DbPool) -> bool` — calls `get_setting`, parse "1" as true, else false

2. In `src/db/mod.rs`:
   - Add `pub mod settings;`
   - Add `pub fn ensure_app_settings_table(pool: &DbPool)`:
     ```sql
     CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
     INSERT OR IGNORE INTO app_settings (key, value) VALUES ('token_auth_enabled', '1');
     ```

## Task 2: AppState — New Fields

**Files**: `src/main.rs`

1. Add imports: `std::sync::atomic::AtomicBool`, `std::collections::HashMap`, `tokio::sync::RwLock`.
2. Add to `AppState`:
   - `pub token_auth_enabled: AtomicBool`
   - `pub admin_sessions: RwLock<HashMap<String, i64>>`
3. After `ensure_api_tokens_table`, call `db::ensure_app_settings_table(&rw_pool)`.
4. Read initial toggle: `let auth_enabled = db::settings::get_token_auth_enabled(&rw_pool);`
5. Build AppState with:
   - `token_auth_enabled: AtomicBool::new(auth_enabled)`
   - `admin_sessions: RwLock::new(HashMap::new())`

## Task 3: Auth — Cookie-Based Admin Auth + Toggle Bearer Auth

**Files**: `src/auth/mod.rs`

1. Add `pub struct TokenAuthEnabled(pub Arc<AtomicBool>);` (Extension wrapper).
2. Modify `admin_auth`:
   - Check `x-admin-secret` header first (existing logic).
   - If header absent/invalid, extract `oj_admin_session` cookie from `Cookie` header.
   - Lookup cookie value in `admin_sessions` (need `Extension<AdminSessions>` where `AdminSessions(Arc<RwLock<HashMap<String, i64>>>)`).
   - If found and `expires_at > chrono::Utc::now().timestamp()`: pass through.
   - If expired: remove from map.
   - If neither valid: return 401.
3. Modify `bearer_auth`:
   - At top: `let enabled = token_auth.0.load(Ordering::Acquire);`
   - If `!enabled`: return `next.run(request).await` immediately.
   - Else: proceed with existing token validation.
4. Add `Extension<TokenAuthEnabled>` extraction to `bearer_auth` signature.
5. Add `Extension<AdminSessions>` extraction to `admin_auth` signature.

## Task 4: Admin — Login/Logout Handlers

**Files**: `src/admin/handlers.rs`, `src/admin/pages.rs`

1. In `pages.rs`:
   - Add `LoginTemplate` (standalone template, path = "admin/login.html").
   - `pub async fn login_page() -> impl IntoResponse` — render login form.
   - `pub async fn login_page_with_error(error: &str) -> Html<String>` — helper.

2. In `handlers.rs`:
   - `pub async fn login_submit(State, Extension<AdminSecret>, Extension<AdminSessions>, Form)`:
     - Extract `secret` from form body.
     - If matches `AdminSecret`: generate 32-byte hex token via `rand::thread_rng()`, insert `(token, now + 28800)` into sessions map, build response with `Set-Cookie` header, redirect 303 to `/admin/`.
     - If mismatch: re-render login with error.
   - `pub async fn logout(Extension<AdminSessions>, request)`:
     - Extract cookie, remove session from map.
     - Set cookie `Max-Age=0`.
     - Redirect 303 to `/admin/login`.

## Task 5: Admin — Settings Toggle Handler

**Files**: `src/admin/handlers.rs`

1. `pub async fn get_token_auth_setting(State)`:
   - Read `state.token_auth_enabled.load(Acquire)`.
   - Return `Json({"enabled": bool})`.

2. `pub async fn set_token_auth_setting(State, Json<{"enabled": bool}>)`:
   - Write DB: `db::settings::set_setting(&pool, "token_auth_enabled", if enabled { "1" } else { "0" })`.
   - If DB success: `state.token_auth_enabled.store(enabled, Release)`.
   - Return `Json({"enabled": bool})`.

## Task 6: Admin Router — Split Public/Protected

**Files**: `src/admin/mod.rs`

1. Split `admin_router()`:
   ```rust
   pub fn admin_router() -> Router<Arc<AppState>> {
       let public = Router::new()
           .route("/admin/login", get(pages::login_page).post(handlers::login_submit))
           .route("/admin/logout", post(handlers::logout));

       let protected = Router::new()
           .route("/admin/", get(pages::index))
           .route("/admin/problems", get(pages::problems_page))
           .route("/admin/tokens", get(pages::tokens_page))
           .route("/admin/api/tokens", get(handlers::list_tokens).post(handlers::create_token))
           .route("/admin/api/tokens/{token}", delete(handlers::revoke_token))
           .route("/admin/api/settings/token-auth", get(handlers::get_token_auth_setting).put(handlers::set_token_auth_setting))
           .route("/admin/api/problems", post(handlers::create_problem))
           .route("/admin/api/problems/{source}/{id}", put(handlers::update_problem).delete(handlers::delete_problem))
           .route("/admin/api/crawlers/trigger", post(handlers::trigger_crawler))
           .route("/admin/api/crawlers/status", get(handlers::crawler_status))
           .route_layer(middleware::from_fn(crate::auth::admin_auth));

       public.merge(protected)
   }
   ```

## Task 7: main.rs — Wire Extensions

**Files**: `src/main.rs`

1. Add Extensions for new auth types:
   ```rust
   .layer(Extension(auth::AdminSessions(Arc::new(state.admin_sessions.clone()))))
   // No — admin_sessions is inside Arc<AppState> already. Use Extension wrapping Arc.
   ```
   Actually: `admin_sessions` is in `AppState` which is `Arc`-wrapped. The middleware needs access. Options:
   - Pass `Extension(auth::AdminSessions(Arc::clone(&sessions_arc)))` where sessions_arc is created before AppState.
   - Or create the RwLock separately, clone Arc into AppState and Extension.

   **Concrete plan**: Create `admin_sessions` and `token_auth_enabled` as standalone `Arc` before building AppState:
   ```rust
   let admin_sessions = Arc::new(RwLock::new(HashMap::new()));
   let token_auth_enabled = Arc::new(AtomicBool::new(auth_enabled));
   ```
   Store clones in AppState (change fields to `Arc<...>`), and also add as Extensions.

2. Add Extension layers:
   ```rust
   .layer(Extension(auth::AdminSessions(admin_sessions.clone())))
   .layer(Extension(auth::TokenAuthEnabled(token_auth_enabled.clone())))
   ```

## Task 8: Templates — Login Page

**Files**: `templates/admin/login.html` (new)

Standalone HTML (does NOT extend base.html):
- Dark themed, centered login card.
- Password input for admin secret.
- Submit button.
- Error message display (passed via template variable).
- Links `/static/admin.css`.

## Task 9: Templates — Base + Dashboard + Dark Theme

**Files**: `templates/base.html`, `templates/admin/index.html`

1. `base.html`:
   - Replace inline `<style>` with `<link rel="stylesheet" href="/static/admin.css">`.
   - Add `<script src="/static/admin.js" defer></script>`.
   - Add `<div id="toast-container"></div>` before `</body>`.
   - Nav: add logout link `<a href="#" onclick="fetch('/admin/logout',{method:'POST',credentials:'same-origin'}).then(()=>location='/admin/login')">Logout</a>`.

2. `index.html`:
   - Dashboard with stat cards (total_problems, active_tokens, token_auth_enabled).
   - Template receives these values from handler.
   - Cards are links to respective pages.

3. Update `pages.rs` `index()` to query stats and pass to template.

## Task 10: Templates — Tokens Page Redesign

**Files**: `templates/admin/tokens.html`

1. Top section: toggle switch for token auth.
   - `<label class="switch"><input type="checkbox" id="token-auth-toggle" {{checked}}><span class="slider"></span></label>`
   - JS binds change event → PUT `/admin/api/settings/token-auth`.

2. Create token form: label input + "Generate Token" button.
   - JS: POST `/admin/api/tokens` → show modal with full token + copy button.

3. Token table: masked tokens, labels, dates, active status, revoke buttons.
   - Revoke: JS DELETE → remove row → toast.

## Task 11: Templates — Problems Page Dark Theme

**Files**: `templates/admin/problems.html`

- Apply dark theme classes (handled by CSS).
- Delete links: convert to JS AJAX with confirmation dialog.

## Task 12: Static Assets — CSS + JS

**Files**: `static/admin.css` (new), `static/admin.js` (new)

1. `admin.css`:
   - CSS variables for color palette.
   - Body, nav, container, card, table, form, badge, pagination, toggle switch, toast, modal styles.
   - Login page specific styles.
   - Responsive breakpoints.

2. `admin.js`:
   - `api(url, options)` — fetch wrapper with credentials + 401 handling.
   - `toast(message, type)` — create/show/auto-dismiss notifications.
   - Token auth toggle binding.
   - Token create/revoke AJAX.
   - Problem delete AJAX.
   - Copy-to-clipboard utility.

## Dependency Order

```
Task 1 (DB settings) → Task 2 (AppState) → Task 3 (Auth) → Task 7 (Wire)
Task 4 (Login handlers) → Task 6 (Router split)
Task 5 (Settings handler) → Task 6 (Router split)
Task 8 (Login template)
Task 12 (CSS/JS) → Task 9 (Base+Dashboard) → Task 10 (Tokens) → Task 11 (Problems)
```

Parallelizable: Tasks 1,8,12 can start simultaneously. Tasks 4,5 can run in parallel after Task 3. Tasks 9,10,11 are sequential after 12.
