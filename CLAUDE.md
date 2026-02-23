# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust REST API server for querying competitive programming problems across multiple online judges (LeetCode, AtCoder, Codeforces, Luogu, UVa, SPOJ). Built with axum 0.8 + SQLite + sqlite-vec vector search, with Python crawler scripts for data collection.

## Commands

```bash
# Build
cargo build --release

# Run (requires config.toml with server.admin_secret set)
cargo run --release

# Lint & format
cargo clippy
cargo fmt

# Tests (none currently exist)
cargo test

# Python crawler environment
cd scripts && uv sync

# Docker
docker build -t oj-api-rs .
docker run -p 3000:3000 -v ./config.toml:/app/config.toml:ro -v oj-data:/app/data oj-api-rs
```

## Architecture

Layered architecture with clear separation:

- **Router layer** (`src/api/mod.rs`, `src/admin/mod.rs`): Route tree assembly with middleware
- **Handler layer** (`src/api/*.rs`, `src/admin/handlers.rs`, `src/admin/pages.rs`): Request handling, validation, response building
- **DB layer** (`src/db/*.rs`): Synchronous SQLite operations bridged to async via `spawn_blocking`
- **Models** (`src/models.rs`): Shared data structures + crawler argument whitelist validation
- **Auth middleware** (`src/auth/mod.rs`): Bearer token auth (toggleable) + Admin session auth

Shared state flows through `Arc<AppState>` containing RO/RW connection pools, config, and runtime state (crawler jobs, semaphores).

### Key Design Decisions

- **RO/RW pool separation**: Read pool uses `PRAGMA query_only=ON`; write pool capped at max_size=2; WAL mode for concurrent reads/writes
- **All DB calls** go through `tokio::task::spawn_blocking` to avoid blocking the async runtime
- **Crawlers** are Python subprocesses invoked via `tokio::process::Command` calling `uv run python3 <script>.py`
- **Embedding concurrency** controlled by configurable `Semaphore` (`embedding.concurrency` in config.toml); text queries generate embeddings via `embedding_cli.py --embed-text`
- **Vector search** uses over-fetch strategy (default 4x) with post-filtering, max k=200
- **Error responses** follow RFC 7807 (`application/problem+json`) — see `src/api/error.rs`
- **Crawler argument validation** uses strict whitelist (`ArgSpec` + `validate_args` in `models.rs`) to prevent injection
- **Admin auth** supports both `x-admin-secret` header and `oj_admin_session` HttpOnly cookie
- **Daily challenge fallback**: returns HTTP 202 and triggers background crawler if DB has no data, with cooldown + TOCTOU guard

### Database

SQLite with WAL mode. No migration system — tables created at startup via `CREATE TABLE IF NOT EXISTS`. Core data tables (`problems`, `daily_challenge`, `vec_embeddings`, `problem_embeddings`) are populated by Python crawlers. The `api_tokens` and `app_settings` tables are managed by the Rust app.

sqlite-vec extension provides KNN vector search with 768-dim Gemini embeddings.

### Templates & Frontend

Askama compile-time templates in `templates/` render the admin UI. Static assets in `static/` include dark-theme CSS, AJAX-driven JS, and i18n support (en, zh-TW, zh-CN).

## Configuration

All config via `config.toml` at project root, parsed by `toml` crate + `serde::Deserialize` with nested structs. Overridable via `CONFIG_PATH` env var. Required: `server.admin_secret`. Optional: `gemini.api_key` for embedding features (Python-only, Rust ignores `[gemini]` section). See `config.toml.example` for full structure and defaults. Python crawlers read the same `config.toml` via `scripts/utils/config.py`.

## API Routes

Public API at `/api/v1/*` (Bearer token auth, CORS enabled). Admin at `/admin/*` (session cookie or `x-admin-secret` header). Health check at `GET /health` (no auth). See `src/api/mod.rs` and `src/admin/mod.rs` for complete route definitions.
