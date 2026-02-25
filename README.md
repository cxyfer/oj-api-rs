# oj-api-rs

REST API server for querying competitive programming problems across multiple online judge platforms.

Built with Rust (axum + SQLite), featuring vector similarity search, a tri-lingual admin dashboard, automated crawler management, and multi-source problem resolution.

## Supported Platforms

| Platform | Source Key | Status |
|----------|-----------|--------|
| LeetCode | `leetcode` | ✅ |
| AtCoder | `atcoder` | ✅ |
| Codeforces | `codeforces` | ✅ |
| Luogu | `luogu` | 🚧 Planned |
| UVa | `uva` | 🚧 Planned |
| SPOJ | `spoj` | 🚧 Planned |

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
cp config.toml.example config.toml
# Edit config.toml — set server.admin_secret

cargo run --release
```

The server starts at `http://0.0.0.0:7856` by default.

### Docker

```bash
# Pull pre-built image
docker pull ghcr.io/cxyfer/oj-api-rs:latest

# Or build locally
docker build -t oj-api-rs .

docker run -d --name oj-api-rs --restart unless-stopped \
  -p 7856:7856 \
  -v ./config.toml:/app/config.toml:ro \
  -v ./data:/app/data \
  ghcr.io/cxyfer/oj-api-rs:latest
```

## Configuration

All settings are loaded from `config.toml` at the project root (overridable via `CONFIG_PATH` env var). See `config.toml.example` for all options and defaults.

```toml
[server]
listen_addr = "0.0.0.0:7856"
admin_secret = "changeme"       # required — warning emitted if empty or "changeme"
graceful_shutdown_secs = 10

[database]
path = "data/data.db"           # resolved relative to config file directory
pool_max_size = 8
busy_timeout_ms = 5000

# LLM provider configuration
# Supported providers: "gemini", "openai"
[llm]
provider = "gemini"
api_key = ""
# base_url = ""                 # optional, for proxy or custom endpoint

[llm.models.embedding]
name = "gemini-embedding-001"
dim = 768
task_type = "SEMANTIC_SIMILARITY"
batch_size = 32

[llm.models.rewrite]
name = "gemini-2.0-flash"
temperature = 0.3
timeout = 60
max_retries = 2
workers = 8

[crawler]
timeout_secs = 300
# user_agent = "Mozilla/5.0 (compatible; OJ-API-Bot/1.0)"
# proxy = "http://127.0.0.1:7890"

[embedding]
timeout_secs = 30               # per-query embed-text timeout (similar search)
over_fetch_factor = 4
concurrency = 4                 # 1..=32

[logging]
rust_log = "info"
level = "INFO"
```

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
                                      # ?query=<text> also accepted
```

Text query mode delegates to a Python subprocess for real-time Gemini embedding generation. Surrounding double quotes in the query value (e.g. `%22two-sum%22`) are automatically stripped.

### Smart Resolution

```
GET /api/v1/resolve/{query}           # Auto-detect source from URL, prefix, or ID pattern
```

Accepts URLs (`leetcode.com/problems/two-sum`), prefixed IDs (`atcoder:abc321_a`), or bare patterns (`123A` -> Codeforces, pure digits -> LeetCode). LeetCode URL slugs are automatically resolved to numeric problem IDs via DB lookup.

### System Status

```
GET /status                           # Requires Bearer token (same as /api/v1/*)
```

Returns API version and per-platform statistics (total problems, missing content, not-embedded counts).

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
├── config.rs         # TOML-based configuration (config.toml + serde)
├── models.rs         # Shared data structures (Problem, CrawlerJob, etc.)
├── health.rs         # Health check with DB/extension validation
├── detect.rs         # Source detection (URL, prefix, pattern inference)
├── api/              # Public REST API routes
│   ├── problems.rs   # Problem queries with pagination
│   ├── daily.rs      # Daily challenge + crawler fallback (HTTP 202)
│   ├── similar.rs    # Vector similarity search (by ID or text)
│   ├── resolve.rs    # Smart resolution (with LeetCode slug-to-ID lookup)
│   ├── status.rs     # System status (version + per-platform stats)
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

## Development

### Rust

```bash
# Build
cargo build --release

# Lint
cargo clippy

# Format
cargo fmt
```

### Python Scripts

```bash
cd scripts && uv sync --dev

# Format
uv run ruff format .

# Lint
uv run ruff check .
```

## License

MIT
