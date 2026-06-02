# oj-api-rs

REST API server for querying competitive programming problems across multiple online judge platforms.

Built with Rust (axum + SQLite), featuring vector similarity search, a tri-lingual admin dashboard, automated crawler management, and multi-source problem resolution.

## Supported Platforms

| Platform | Source Key | Status |
|----------|-----------|--------|
| LeetCode | `leetcode` | ✅ |
| AtCoder | `atcoder` | ✅ |
| Codeforces | `codeforces` | ✅ |
| Luogu | `luogu` | ✅ |
| UVa | `uva` | 🚧 Planned |
| SPOJ | `spoj` | ✅ |

## Tech Stack

- **Runtime** — axum 0.8 + tokio
- **Database** — SQLite (rusqlite + r2d2 connection pooling, WAL mode, RO/RW pool separation)
- **Vector Search** — sqlite-vec (768-dim Gemini embeddings, KNN with over-fetch strategy)
- **Templates** — Askama (compile-time, type-safe admin dashboard)
- **Auth** — Bearer token (toggleable via admin UI) + session-based admin auth (HttpOnly cookie)
- **MCP** — Native Streamable HTTP MCP endpoint at `/mcp` with the same token policy as `/api/v1/*`
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
GET  /api/v1/problems/{source}/{id}    # Get a single problem
POST /api/v1/problems/batch            # Batch fetch multiple problems
GET  /api/v1/problems/{source}         # List problems
GET  /api/v1/problems/tags/{source}    # List all tags for a source
GET  /api/v1/problems/difficulties/{source}  # List all difficulties for a source
```

`GET /api/v1/problems/{source}/{id}` returns the full problem record. As of 2026-03-24, the `similar_questions` field is a hydrated object array, not a slug string array:

```json
{
  "id": "1",
  "source": "leetcode",
  "slug": "two-sum",
  "similar_questions": [
    {
      "id": "15",
      "source": "leetcode",
      "slug": "3sum",
      "title": "3Sum",
      "title_cn": "三数之和",
      "difficulty": "Medium",
      "ac_rate": 35.8,
      "rating": 1456.0,
      "contest": null,
      "problem_index": null,
      "tags": ["Array", "Two Pointers"],
      "link": "https://leetcode.com/problems/3sum/"
    }
  ]
}
```

This is a breaking response-schema change for detail consumers that previously treated `similar_questions` as `string[]`.

`POST /api/v1/problems/batch` accepts a JSON array of `{source, id}` objects (max 50) and returns matched problems in `results[]` with unmatched keys in `not_found[]`. Add `?detail=true` to include full content and hydrated `similar_questions`:

<details>
<summary>GET /api/v1/problems/{source} — Query Parameters</summary>

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | integer | `1` | Page number |
| `per_page` | integer | `20` | Results per page |
| `difficulty` | string | — | Filter by difficulty |
| `tags` | string | — | Comma-separated tag filter |
| `tag_mode` | `any` \| `all` | `any` | Tag matching mode |
| `search` | string | — | Keyword search |
| `sort_by` | `id` \| `difficulty` \| `rating` \| `ac_rate` | — | Sort field |
| `sort_order` | `asc` \| `desc` | — | Sort direction |
| `rating_min` | float | — | Minimum difficulty rating |
| `rating_max` | float | — | Maximum difficulty rating |

</details>

### Daily Challenge

```
GET /api/v1/daily                     # LeetCode daily challenge
                                      # ?domain=com|cn  &date=YYYY-MM-DD
```

When today's challenge is not yet in the database, the API triggers a background Python crawler and waits up to 10 s for it to complete before responding. If the crawler succeeds within the window the response is HTTP 200; otherwise HTTP 202 is returned.

Pass `?async=true` to skip waiting and receive HTTP 202 immediately.

The successful daily challenge response uses the same hydrated `similar_questions: ProblemSummary[]` structure as `GET /api/v1/problems/{source}/{id}`.

### Similarity Search

```
GET  /api/v1/similar/{source}/{id}    # Find similar problems by embedding
POST /api/v1/similar                  # Find similar problems by text query
```

Text query mode delegates to a Python subprocess for real-time Gemini embedding generation. Surrounding double quotes in the `query` body field are automatically stripped.

<details>
<summary>Parameters</summary>

| Endpoint | Parameter | Location | Type | Default | Description |
|----------|-----------|----------|------|---------|-------------|
| `GET /api/v1/similar/{source}/{id}` | `limit` | query | integer | `10` | Max results (max `50`) |
| `GET /api/v1/similar/{source}/{id}` | `threshold` | query | float | `0.0` | Minimum similarity score |
| `GET /api/v1/similar/{source}/{id}` | `source` | query | string | — | Comma-separated source filter (e.g. `leetcode,atcoder`) |
| `POST /api/v1/similar` | `query` | JSON body | string | — | Required text query (3–2000 chars); `q` is also accepted |
| `POST /api/v1/similar` | `limit` | JSON body | integer | `10` | Max results (max `50`) |
| `POST /api/v1/similar` | `threshold` | JSON body | float | `0.0` | Minimum similarity score |
| `POST /api/v1/similar` | `source` | JSON body | string | — | Comma-separated source filter (e.g. `leetcode,atcoder`) |

</details>

### Smart Resolution

```
GET /api/v1/resolve/{query}           # Auto-detect source from URL, prefix, or ID pattern
```

Accepts URLs (`leetcode.com/problems/two-sum`), prefixed IDs (`atcoder:abc321_a`), or bare patterns (`123A` -> Codeforces, pure digits -> LeetCode). LeetCode URL slugs are automatically resolved to numeric problem IDs via DB lookup.

When `problem` is present, it follows the same detail schema as `GET /api/v1/problems/{source}/{id}`, including hydrated `similar_questions` summary objects.

### System Status

```
GET /status                           # Requires Bearer token (same as /api/v1/*)
```

Returns API version and per-platform statistics (total problems, missing content, not-embedded counts).

### MCP (HTTP)

```
POST /mcp                            # Streamable HTTP MCP endpoint
GET  /mcp                            # SSE resume / session stream
```

- Reuses the same server process and listener as the REST API.
- Uses the same Bearer-token gate as `/api/v1/*` and `/status`: when token auth is enabled, `/mcp` also requires `Authorization: Bearer <token>`.
- Exposes 5 tools: `resolve_problem`, `get_problem`, `get_daily_challenge`, `find_similar_problems`, `get_platform_status`.

Example client config (HTTP MCP):

```json
{
  "servers": {
    "oj": {
      "type": "http",
      "url": "https://your-host.example.com/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_TOKEN"
      }
    }
  }
}
```

#### Client configuration examples

<details>
<summary><b>Claude Code</b></summary>

With Bearer token auth:

```bash
export OJ_API_TOKEN="YOUR_TOKEN"
claude mcp add --transport http oj https://your-host.example.com/mcp \
  --header "Authorization: Bearer $OJ_API_TOKEN"
```

Without token auth:

```bash
claude mcp add --transport http oj https://your-host.example.com/mcp
```

If you want the server available outside the current project, add `--scope user`:

```bash
claude mcp add --scope user --transport http oj https://your-host.example.com/mcp
```
</details>

<details>
<summary><b>Codex</b></summary>

With Bearer token auth:

```bash
export OJ_API_TOKEN="YOUR_TOKEN"
codex mcp add oj --url https://your-host.example.com/mcp --bearer-token-env-var OJ_API_TOKEN
```

Without token auth:

```bash
codex mcp add oj --url https://your-host.example.com/mcp
```

Confirm the server is registered:

```bash
codex mcp list
```
</details>

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
├── leetcode.py       # LeetCode crawler (--sync-problemset, --daily, --date, ...)
├── atcoder.py        # AtCoder crawler (--sync-problemset, --fetch-contest, ...)
├── codeforces.py     # Codeforces crawler (--sync-problemset, --fetch-contest, ...)
├── luogu.py          # Luogu/SPOJ crawler (--sync-problemset, --training-list, ...)
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

## Crawler CLI Operations

Crawler scripts use three canonical operation flags where the platform supports them:

| Operation | Purpose | Supported sources |
| --- | --- | --- |
| `--sync-problemset` | Fetch initial problem metadata and skip existing problems unless a source-specific overwrite flag is used. | LeetCode, AtCoder, Codeforces, Luogu, SPOJ |
| `--fetch-contest` | Fetch contest/archive problems and their content. | AtCoder, Codeforces |
| `--fill-missing-content` | Fill content for existing metadata-only problems. | LeetCode, AtCoder, Codeforces, Luogu, SPOJ |

AtCoder and Codeforces contest fetching resumes by default from their JSON progress files. Use `--no-resume` with `--fetch-contest` to ignore saved progress and rescan contests.

Legacy operation flags remain compatibility aliases for existing jobs: LeetCode `--init`, Luogu `--sync`, SPOJ `--sync-spoj`, AtCoder `--sync-kenkoooo` / `--sync-history`, and AtCoder/Codeforces `--fetch-all` / `--resume`. Prefer the canonical flags in new commands and admin-triggered runs.

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
