## 1. Project Scaffold & Configuration

- [x] 1.1 Initialize Cargo workspace with `Cargo.toml` (axum, tokio, rusqlite/bundled, r2d2, sqlite-vec, serde, serde_json, regex, zerocopy 0.7, askama, tower-http, tracing, tracing-subscriber, chrono, rand, uuid)
- [x] 1.2 Create `src/config.rs` — load all env vars (LISTEN_ADDR, DATABASE_PATH, ADMIN_SECRET, GEMINI_API_KEY, DB_POOL_MAX_SIZE, BUSY_TIMEOUT_MS, EMBED_TIMEOUT_SECS, CRAWLER_TIMEOUT_SECS, OVER_FETCH_FACTOR, GRACEFUL_SHUTDOWN_SECS, RUST_LOG) with defaults; fail-fast on missing ADMIN_SECRET
- [x] 1.3 Create `src/models.rs` — define `Problem`, `ProblemSummary`, `DailyChallenge`, `ApiToken`, `CrawlerJob` structs with serde Serialize/Deserialize; handle `tags`/`similar_questions` JSON-to-Vec deserialization with fallback to empty array

## 2. Database Layer

- [x] 2.1 Create `src/db/mod.rs` — sqlite-vec registration via `sqlite3_auto_extension`, r2d2 pool builder (read-only pool with `query_only=ON`, read-write pool), WAL mode + busy_timeout per connection init
- [x] 2.2 Create `src/db/problems.rs` — `get_problem(source, id)`, `list_problems(source, filters, page, per_page)` with COUNT query for meta, `insert_problem`, `update_problem`, `delete_problem` (cascade delete from vec_embeddings + problem_embeddings in transaction)
- [x] 2.3 Create `src/db/daily.rs` — `get_daily(domain, date)` query from `daily_challenge` table
- [x] 2.4 Create `src/db/embeddings.rs` — `get_embedding(source, id)` returning `Vec<f32>` (binary LE parse with JSON fallback), `knn_search(embedding, k)` returning `(source, problem_id, distance)` tuples
- [x] 2.5 Create `src/db/tokens.rs` — `validate_token(token)` returning bool + update `last_used_at`, `list_tokens()`, `create_token(label)` generating 64-char hex, `revoke_token(token)` setting `is_active=0`

## 3. Source Detection

- [x] 3.1 Create `src/detect.rs` — port Python `source_detector.py` logic: URL regex (atcoder, leetcode, codeforces, luogu), prefix `source:id` split, ID pattern matching (CF, AtCoder, pure numeric, default slug), return `(source, id)`

## 4. Error Handling

- [x] 4.1 Create `src/api/error.rs` — RFC 7807 `ProblemDetail` struct with `type`, `title`, `status`, `detail`, optional `errors` array for validation; implement `IntoResponse` for axum

## 5. Authentication Middleware

- [x] 5.1 Create `src/auth/mod.rs` — Bearer token extractor middleware for `/api/v1/*` (validate against DB, update `last_used_at`, return 401 on missing/invalid/inactive); Admin secret extractor for `/admin/*` (compare X-Admin-Secret header, return 401 on mismatch)

## 6. Public API Routes

- [x] 6.1 Create `src/api/problems.rs` — `GET /api/v1/problems/{source}/{id}` (full problem with parsed tags/similar_questions); `GET /api/v1/problems/{source}` (paginated list with tags/difficulty filter, per_page clamped 1-100, default 20)
- [x] 6.2 Create `src/api/resolve.rs` — `GET /api/v1/resolve/{query}` (URL-decode input, run detect, lookup problem in DB, return `{source, id, problem}` with problem=null if not found)
- [x] 6.3 Create `src/api/daily.rs` — `GET /api/v1/daily` (validate domain com/cn, validate date format YYYY-MM-DD + range [2020-04-01, today UTC], default today, query DB)
- [x] 6.4 Create `src/api/similar.rs` — by-problem mode: `GET /api/v1/similar/{source}/{id}` (fetch embedding, KNN with over-fetch, similarity=1-distance, filter threshold/source, exclude seed, truncate limit); by-text mode: `GET /api/v1/similar?query=` (validate >=3 chars, spawn Python subprocess with timeout/semaphore, parse JSON embedding, KNN)
- [x] 6.5 Create `src/api/mod.rs` — compose public API router with Bearer auth layer + CORS (allow all origins)

## 7. Admin Routes

- [x] 7.1 Create `src/admin/handlers.rs` — `POST /admin/api/problems` (201), `PUT /admin/api/problems/{source}/{id}` (200), `DELETE /admin/api/problems/{source}/{id}` (204 + cascade); token CRUD: `GET /admin/api/tokens`, `POST /admin/api/tokens` (201), `DELETE /admin/api/tokens/{token}` (204)
- [x] 7.2 Create `src/admin/handlers.rs` crawler endpoints — `POST /admin/api/crawlers/trigger` (async spawn with Arc<Mutex> lock, 202 + job_id / 409 if running), `GET /admin/api/crawlers/status`
- [x] 7.3 Create Askama templates (`templates/base.html`, `templates/admin/index.html`, `templates/admin/problems.html`, `templates/admin/tokens.html`) with auto-escape; problem content rendered in `<iframe sandbox>`
- [x] 7.4 Create `src/admin/pages.rs` — `GET /admin/` (index), `GET /admin/problems` (problem list page with pagination)
- [x] 7.5 Create `src/admin/mod.rs` — compose admin router with admin secret auth layer (no CORS)

## 8. Health Check & Startup

- [x] 8.1 Create health check handler — `GET /health` (no auth): check DB conn, `vec_version()`, `vec_length()`==768; return 200/503 with JSON status
- [x] 8.2 Implement startup self-check in `main.rs` — verify DB connectivity, sqlite-vec loaded, vec dimension; exit(1) on failure

## 9. Application Entry Point

- [x] 9.1 Create `src/main.rs` — initialization sequence: load config → init tracing → register sqlite-vec → build pools → startup self-check → build AppState (pools, config, crawler lock, embed semaphore) → assemble routers (health + public API + admin + static files) → start axum server with graceful shutdown (SIGTERM/SIGINT, configurable timeout, kill_on_drop for subprocesses)

## 10. Python Prerequisite

- [x] 10.1 Add `--embed-text` CLI flag to `embedding_cli.py` — invoke `EmbeddingRewriter.rewrite()` + `EmbeddingGenerator.embed()`, stdout JSON `{"embedding": [...], "rewritten": "..."}`

## 11. Deployment

- [x] 11.1 Create `Dockerfile` — multi-stage build (rust:1-bookworm builder → debian:bookworm-slim runtime with python3), copy binary + templates + static + references, expose 3000
- [x] 11.2 Create `.env.example` with all env vars and their defaults
