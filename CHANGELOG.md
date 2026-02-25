# Changelog

## [0.1.4] - 2026-02-25

### Bug Fixes

- **docker**: remove `embeddings/` from `.dockerignore` to fix `ModuleNotFoundError` in container

### Chore

- **docker**: set `PYTHONPATH` to fix `ModuleNotFoundError` for local scripts
- **scripts**: format Python code with ruff
- **src**: format Rust code with `cargo fmt` and fix `cargo clippy` warnings

### Docs

- **readme**: add `--restart` and `--name` flags to `docker run` example
- **readme**: add development section with ruff usage
- **readme**: add rust development section with cargo commands

## [0.1.3] - 2026-02-25

### Chore

- **server**: change default listen port from 3000 to 7856

## [0.1.2] - 2026-02-25

### Features

- **embedding**: add LLM provider abstraction with Gemini and OpenAI adapters
- **admin**: add embedding management page with stats, trigger, and progress
- **api**: wrap similar endpoints with `rewritten_query` field
- **admin**: show dual progress bars for embedding pipeline

### Bug Fixes

- **embedding**: ensure progress JSON reflects final status after job completion

### Refactor

- **embedding**: remove rust-side timeout for embedding trigger

## [0.1.1] - 2026-02-24

### Features

- **api**: add `GET /status` endpoint with per-platform stats
- **crawler**: unify proxy and user-agent config via `BaseCrawler`
- **diag**: add crawler diagnostic script for UA and proxy verification

### Bug Fixes

- **resolve**: resolve LeetCode slug to numeric ID via DB lookup
- **similar**: accept `?q=` alias and strip surrounding quotes
- **i18n**: add missing `sources` object to zh-CN locale

## [0.1.0] - Initial Release
