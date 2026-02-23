# Proposal: Unify Configuration to Single config.toml

## Context

The project currently has a split configuration system:
- **Rust app**: reads flat env vars from `.env` via `dotenvy` (`src/config.rs`)
- **Python scripts**: reads `scripts/config.toml` via `ConfigManager` (`scripts/utils/config.py`)
- Two config sources can drift out of sync (e.g., `DATABASE_PATH` vs `database.path`)
- Secrets (`ADMIN_SECRET`, `GEMINI_API_KEY`) are scattered across env vars

The goal is to consolidate everything into a single `config.toml` at the project root, eliminating `.env` and `scripts/config.toml`. The reference structure is `references/leetcode-daily-discord-bot/config.toml.example`.

## Requirements

### R1: Single Root config.toml

- One `config.toml` at project root serves both Rust and Python.
- Delete `scripts/config.toml`; Python `ConfigManager` reads `../config.toml` (relative to `scripts/` cwd) or accepts a path override.
- Provide `config.toml.example` with all keys, defaults, and comments.
- `.gitignore` must include `config.toml` (contains secrets); `.env` entry can be removed.

### R2: Rust Config Parsing with toml + serde

- Replace `dotenvy` + manual `env::var` parsing with `toml` crate + `serde::Deserialize`.
- `Config` struct derives `Deserialize` with `#[serde(default)]` for optional fields.
- Config file path: `config.toml` relative to cwd, overridable via `CONFIG_PATH` env var (single escape hatch).
- On missing file: print clear error message and exit, similar to current `ADMIN_SECRET` behavior.
- Remove `dotenvy` from `Cargo.toml` dependencies.

### R3: TOML Structure

```toml
[server]
listen_addr = "0.0.0.0:3000"
admin_secret = "changeme"
graceful_shutdown_secs = 10

[database]
path = "data/data.db"
pool_max_size = 8
busy_timeout_ms = 5000

[gemini]
api_key = ""
# base_url = ""  # optional, for proxy

[gemini.models.embedding]
name = "gemini-embedding-001"
dim = 768
task_type = "SEMANTIC_SIMILARITY"
batch_size = 32

[gemini.models.rewrite]
name = "gemini-2.0-flash"
temperature = 0.3
timeout = 60
max_retries = 2
workers = 8

[crawler]
timeout_secs = 300

[embedding]
timeout_secs = 30
over_fetch_factor = 4
concurrency = 4

[logging]
rust_log = "info"
```

- `[server]` groups listen address, admin secret, shutdown.
- `[database]` unifies Rust pool config and Python DB path (single `path` field, Rust resolves relative to cwd, Python resolves relative to config file location).
- `[gemini]` replaces both Rust `GEMINI_API_KEY` and Python `llm.gemini.*`. Flattened from `llm.gemini` since Gemini is the only LLM provider.
- `[crawler]` holds crawler timeout.
- `[embedding]` holds embed timeout, over-fetch factor, and concurrency (currently hardcoded `Semaphore::new(4)`).
- `[logging]` maps `rust_log` to `RUST_LOG` env var at startup.

### R4: Rust Config Struct Mapping

- `Config` struct mirrors TOML structure with nested sub-structs: `ServerConfig`, `DatabaseConfig`, `GeminiConfig`, `CrawlerConfig`, `EmbeddingConfig`, `LoggingConfig`.
- All fields have serde defaults matching current hardcoded values.
- `admin_secret` validation: if empty or "changeme", print fatal error and exit.
- After parsing, set `RUST_LOG` env var from `logging.rust_log` before `tracing_subscriber` init (only if not already set by env).

### R5: Python ConfigManager Adaptation

- `ConfigManager.__init__` default path changes from `"config.toml"` to auto-detect: walk up from `scripts/` to find `config.toml` at project root, or accept explicit path.
- Key path migration: `llm.gemini.*` → `gemini.*` (breaking change for Python config access).
- Update all `config.get("llm.gemini.*")` calls to `config.get("gemini.*")`.
- `config.database_path` resolves relative to config file location (not cwd).
- Remove `discord.*`, `schedule.*`, `bot.*` sections from `_apply_env_overrides` — these belong to the Discord bot project, not this one.
- Keep env var override for `GEMINI_API_KEY` → `gemini.api_key` as convenience.

### R6: Subprocess Config Passing

- Rust passes config file path to Python subprocesses via `--config <path>` CLI arg (instead of relying on cwd-relative discovery).
- `embedding_cli.py` and crawler scripts accept `--config` flag.
- Remove `cmd.env("GEMINI_API_KEY", key)` from `src/api/similar.rs` — Python reads it from the shared config.toml.

### R7: Docker Compatibility

- Dockerfile copies `config.toml.example` to `/app/config.toml.example`.
- Users mount their own `config.toml` via `-v ./config.toml:/app/config.toml:ro`.
- Update README docker run example.
- Remove `.env` references from `.dockerignore` (no longer relevant).

### R8: Migration & Cleanup

- Delete `.env.example`.
- Delete `scripts/config.toml`.
- Update `CLAUDE.md` to reflect new config approach.
- Update `README.md` setup instructions.

## Constraints

- **C1**: Python `ConfigManager` class interface (`get()`, `get_section()`, properties) must remain stable — only internal path resolution and key prefixes change.
- **C2**: All current defaults must be preserved exactly (no silent behavior changes).
- **C3**: `config.toml` must be in `.gitignore` since it contains secrets.
- **C4**: `CONFIG_PATH` env var is the only remaining env var (escape hatch for non-standard deployments).
- **C5**: Existing Python env var override for `GEMINI_API_KEY` must continue to work.

## Success Criteria

1. `cargo build --release` succeeds with only `config.toml` present (no `.env`).
2. `cargo run --release` reads all config from `config.toml` and starts server correctly.
3. Python crawlers (`uv run python3 leetcode.py --daily`) read DB path and Gemini key from root `config.toml`.
4. `embedding_cli.py --embed-text "test"` reads Gemini config from root `config.toml`.
5. `docker build && docker run -v ./config.toml:/app/config.toml:ro` works.
6. Missing `config.toml` produces clear error: "Configuration file not found: config.toml".
7. `admin_secret = "changeme"` produces fatal error on startup.
8. `CONFIG_PATH=/custom/path.toml cargo run` reads from custom path.
9. `scripts/config.toml` no longer exists.
10. `.env` and `.env.example` no longer exist.
