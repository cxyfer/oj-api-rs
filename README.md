# oj-api-rs

REST API server for querying competitive programming problems across multiple online judge platforms.

Built with Rust (axum + SQLite), featuring vector similarity search, an admin dashboard, and multi-source problem resolution.

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
- **Database** — SQLite (rusqlite + r2d2 connection pooling, WAL mode)
- **Vector Search** — sqlite-vec (768-dim embeddings, KNN)
- **Templates** — Askama (admin dashboard)
- **Auth** — Bearer token (toggleable) + session-based admin auth

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

All `/api/v1/*` routes require `Authorization: Bearer <token>` when token auth is enabled.

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

### Similarity Search

```
GET /api/v1/similar/{source}/{id}     # Find similar problems by embedding
GET /api/v1/similar?q=<text>          # Find similar problems by text query
```

### Smart Resolution

```
GET /api/v1/resolve/{query}           # Auto-detect source from URL, prefix, or ID pattern
```

Accepts URLs (`leetcode.com/problems/two-sum`), prefixed IDs (`atcoder:abc321_a`), or bare patterns (`123A` → Codeforces).

### Health Check

```
GET /health                           # No auth required
```

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

Accessible at `/admin/` with session-based authentication.

- **Dashboard** — problem count, active tokens, auth status
- **Problems** — browse, delete problems by source
- **Tokens** — create/revoke API tokens, toggle token auth on/off
- **Crawlers** — trigger and monitor crawler jobs

## Architecture

```
src/
├── main.rs          # Entry point, router assembly, graceful shutdown
├── config.rs        # Environment-based configuration
├── models.rs        # Shared data structures
├── health.rs        # Health check with DB/extension validation
├── detect.rs        # Source detection (URL, prefix, pattern inference)
├── api/             # Public REST API routes
│   ├── problems.rs  # CRUD queries
│   ├── daily.rs     # Daily challenge
│   ├── similar.rs   # Vector similarity search
│   ├── resolve.rs   # Unified resolution
│   └── error.rs     # RFC 7807 error responses
├── auth/            # Bearer token + admin session middleware
├── admin/           # Dashboard handlers and HTML pages
└── db/              # SQLite access layer (RO/RW pool separation)
    ├── problems.rs  # Problem queries
    ├── daily.rs     # Daily challenge queries
    ├── tokens.rs    # API token management
    ├── embeddings.rs# Vector storage and KNN search
    └── settings.rs  # App-wide settings
```

## License

MIT
