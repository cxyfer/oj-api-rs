# oj-api-rs

REST API server for querying competitive programming problems across multiple online judge platforms.

Built with Rust (axum + SQLite), featuring vector similarity search, a tri-lingual admin dashboard, automated crawler management, and multi-source problem resolution.

## Supported Platforms

| Platform | Source Key |
|----------|-----------|
| LeetCode | `leetcode` |
| AtCoder | `atcoder` |
| Codeforces | `codeforces` |
| Luogu | `luogu` |
| UVa | `uva` |
| SPOJ | `spoj` |

## Tech Stack

- **Runtime** — axum 0.8 + tokio
- **Database** — SQLite (rusqlite + r2d2 connection pooling, WAL mode, RO/RW pool separation)
- **Vector Search** — sqlite-vec (768-dim Gemini embeddings, KNN with over-fetch strategy)
- **Templates** — Askama (compile-time, type-safe admin dashboard)
- **Auth** — Bearer token (toggleable via admin UI) + session-based admin auth (HttpOnly cookie)
- **Crawlers** — Python scripts (`scripts/`) spawned via `tokio::process::Command`, per-source CLI argument whitelisting
- **i18n** — Client-side JSON translations (zh-TW / zh-CN / en) with `data-i18n` attributes

## Quick Start

```bash
cp .env.example .env
# Edit .env — ADMIN_SECRET is required

cargo run --release
```

The server starts at `http://0.0.0.0:3000` by default.

### Docker

```bash
docker build -t oj-api-rs .
docker run -p 3000:3000 \
  -e ADMIN_SECRET=your-secret \
  -v oj-data:/app/data \
  oj-api-rs
```

## Configuration

All settings are loaded from environment variables (auto-reads `.env`).

| Variable | Default | Description |
|----------|---------|-------------|
| `LISTEN_ADDR` | `0.0.0.0:3000` | Server bind address |
| `DATABASE_PATH` | `data/data.db` | SQLite database path |
| `ADMIN_SECRET` | *(required)* | Admin login credential |
| `GEMINI_API_KEY` | *(optional)* | Google Gemini API key for embeddings |
| `DB_POOL_MAX_SIZE` | `8` | Read-only connection pool size |
| `BUSY_TIMEOUT_MS` | `5000` | SQLite busy timeout (ms) |
| `EMBED_TIMEOUT_SECS` | `30` | Embedding CLI timeout |
| `CRAWLER_TIMEOUT_SECS` | `300` | Crawler process timeout |
| `OVER_FETCH_FACTOR` | `4` | KNN over-fetch multiplier |
| `GRACEFUL_SHUTDOWN_SECS` | `10` | Shutdown grace period |
| `RUST_LOG` | `info` | Log level filter |

## API Endpoints

All `/api/v1/*` routes require `Authorization: Bearer <token>` when token auth is enabled (toggleable from admin dashboard).

### Problems

```
GET /api/v1/problems/{source}/{id}    # Get a single problem
GET /api/v1/problems/{source}         # List problems (pagination, filter by difficulty/tags)
```

### Daily Challenge

```
GET /api/v1/daily                     # LeetCode daily challenge
                                      # ?domain=com|cn  &date=YYYY-MM-DD
```

When today's challenge is not yet in the database (domain=com only), the API returns HTTP 202 and triggers a background Python crawler. Retry after ~30 seconds to get the data.

### Similarity Search

```
GET /api/v1/similar/{source}/{id}     # Find similar problems by embedding
GET /api/v1/similar?q=<text>          # Find similar problems by text query
```

Text query mode delegates to a Python subprocess for real-time Gemini embedding generation.

### Smart Resolution

```
GET /api/v1/resolve/{query}           # Auto-detect source from URL, prefix, or ID pattern
```

Accepts URLs (`leetcode.com/problems/two-sum`), prefixed IDs (`atcoder:abc321_a`), or bare patterns (`123A` -> Codeforces, pure digits -> LeetCode).

### Health Check

```
GET /health                           # No auth required
```

Returns DB connection status, sqlite-vec extension status, and vector dimension validation.

### Error Format

All errors follow [RFC 7807](https://datatracker.ietf.org/doc/html/rfc7807):

```json
{
  "type": "about:blank",
  "title": "Not Found",
  "status": 404,
  "detail": "problem not found"
}
```

## Admin Dashboard

Accessible at `/admin/` with session-based authentication (HttpOnly cookie).

- **Dashboard** — Problem counts, active tokens, auth toggle status
- **Problems** — Browse by source tabs, view problem details in modal, delete problems
- **Tokens** — Create/revoke API tokens, toggle bearer-token auth on/off globally
- **Crawlers** — Trigger crawlers with per-source CLI arguments, view real-time status and stdout/stderr output, job history
- **i18n** — Language switcher (zh-TW / zh-CN / en) in nav bar, preference persisted in localStorage

## Architecture

```
Python Crawlers (scripts/)
  leetcode.py / atcoder.py / codeforces.py
  embedding_cli.py --embed-text / --build
       |
       | SQLite WAL mode (write)
       v
  +--------------+
  |   data.db    |  (shared SQLite file)
  |  + sqlite-vec|
  +------+-------+
         | SQLite WAL mode (read)
         v
  +-------------------------------+
  |     Rust Backend (axum)       |
  |                               |
  |  +---------+  +------------+  |
  |  | API     |  | Admin      |  |
  |  | Routes  |  | Routes +   |  |
  |  | (JSON)  |  | HTML UI    |  |
  |  +----+----+  +------+-----+  |
  |       |              |        |
  |  +----v--------------v-----+  |
  |  |  rusqlite + r2d2 pool   |  |
  |  |  + sqlite-vec loaded    |  |
  |  +-------------------------+  |
  +-------------------------------+
```

```
src/
├── main.rs           # Entry point, router assembly, graceful shutdown
├── config.rs         # Environment-based configuration
├── models.rs         # Shared data structures (Problem, CrawlerJob, etc.)
├── health.rs         # Health check with DB/extension validation
├── detect.rs         # Source detection (URL, prefix, pattern inference)
├── api/              # Public REST API routes
│   ├── problems.rs   # Problem queries with pagination
│   ├── daily.rs      # Daily challenge + crawler fallback (HTTP 202)
│   ├── similar.rs    # Vector similarity search (by ID or text)
│   ├── resolve.rs    # Smart resolution
│   └── error.rs      # RFC 7807 error responses
├── auth/             # Bearer token (toggleable) + admin session middleware
├── admin/            # Dashboard handlers, pages, and API
│   ├── handlers.rs   # Crawler trigger, token CRUD, settings toggle, problem detail
│   ├── pages.rs      # HTML page handlers
│   └── mod.rs        # Admin router
└── db/               # SQLite access layer (RO/RW pool separation)
    ├── problems.rs   # Problem queries
    ├── daily.rs      # Daily challenge queries
    ├── tokens.rs     # API token management
    ├── embeddings.rs # Vector storage and KNN search
    └── settings.rs   # App-wide settings (token auth toggle)

scripts/              # Python crawlers and embedding pipeline
├── leetcode.py       # LeetCode crawler (--daily, --date, --init, --monthly, ...)
├── atcoder.py        # AtCoder crawler (--fetch-all, --resume, --contest, ...)
├── codeforces.py     # Codeforces crawler (--sync-problemset, --fetch-all, ...)
├── embedding_cli.py  # Embedding pipeline (--build, --embed-text)
├── config.toml       # DB path config for scripts
├── utils/            # Shared utilities (config, database, logger, html_converter)
└── embeddings/       # Embedding modules (generator, rewriter, searcher, storage)

templates/            # Askama HTML templates
├── base.html         # Layout with nav bar + language switcher
└── admin/            # Login, dashboard, problems, tokens, crawlers

static/               # Frontend assets
├── admin.css         # Dark theme stylesheet
├── admin.js          # AJAX helpers, toast, modal logic
├── i18n.js           # i18n loader
└── i18n/             # Translation files (en.json, zh-TW.json, zh-CN.json)
```

## License

MIT
