## 1. Config Example & Dependencies

- [x] 1.1 Create `config.toml.example` at project root with all sections ([server], [database], [gemini], [gemini.models.embedding], [gemini.models.rewrite], [crawler], [embedding], [logging]), defaults, and comments
- [x] 1.2 Add `toml` crate to `Cargo.toml` dependencies
- [x] 1.3 Remove `dotenvy` from `Cargo.toml` dependencies

## 2. Rust Config Struct & Loader

- [x] 2.1 Rewrite `src/config.rs`: define nested structs (`ServerConfig`, `DatabaseConfig`, `CrawlerConfig`, `EmbeddingConfig`, `LoggingConfig`) with `#[serde(default)]` and default fns matching current hardcoded values
- [x] 2.2 Implement TOML file loader: resolve path from `CONFIG_PATH` env var or `./config.toml`, read file, deserialize with `toml::from_str`, fail fast on missing/invalid file
- [x] 2.3 Implement `validate()`: warn on empty/"changeme" admin_secret, reject `embedding.concurrency` outside 1..=32
- [x] 2.4 Remove `gemini_api_key` field from Config struct entirely
- [x] 2.5 Resolve `database.path` relative to config file parent directory (canonicalize to absolute path)
- [x] 2.6 Store resolved config file path in `AppState` for subprocess forwarding

## 3. Rust Main & Integration

- [x] 3.1 Update `src/main.rs`: replace `Config::from_env()` with new TOML loader
- [x] 3.2 Set `RUST_LOG` env var from `logging.rust_log` before `tracing_subscriber` init (only if not already set)
- [x] 3.3 Initialize `embed_semaphore` from `embedding.concurrency` instead of hardcoded `Semaphore::new(4)`

## 4. Rust Subprocess Call Sites

- [x] 4.1 `src/api/similar.rs`: remove `cmd.env("GEMINI_API_KEY", key)` and all `gemini_key` references
- [x] 4.2 All 3 subprocess call sites (`similar.rs`, `daily.rs`, `admin/handlers.rs`): forward `CONFIG_PATH` env var to child process only when set on parent

## 5. Python ConfigManager Adaptation

- [x] 5.1 Change `ConfigManager.__init__` default path from `"config.toml"` to `"../config.toml"`
- [x] 5.2 Add `CONFIG_PATH` env var check: precedence is explicit arg > `CONFIG_PATH` env > default `../config.toml`
- [x] 5.3 Implement `database_path` property to resolve relative paths against config file parent directory
- [x] 5.4 Migrate key paths: `llm.gemini.*` → `gemini.*` in `gemini_api_key`, `gemini_base_url`, `get_embedding_model_config()`, `get_rewrite_model_config()` properties
- [x] 5.5 Update `_apply_env_overrides`: map `GEMINI_API_KEY` → `("gemini", "api_key")`, remove `DISCORD_TOKEN`, `GOOGLE_API_KEY`, `GOOGLE_GEMINI_API_KEY`, `POST_TIME`, `TIMEZONE` mappings
- [x] 5.6 Remove dead code: `discord_token`, `post_time`, `timezone`, `log_directory`, `get_cache_expire_seconds`, `get_llm_model_config` properties/methods
- [x] 5.7 Remove `data_dir` property (unused, `[data]` section eliminated)

## 6. Python Script Call Sites

- [x] 6.1 Audit and update all `config.get("llm.gemini.*")` calls across `scripts/` to use `gemini.*` paths
- [x] 6.2 Verify `embedding_cli.py`, crawler scripts (`leetcode.py`, `atcoder.py`, `codeforces.py`, etc.) work with new config paths

## 7. Docker

- [x] 7.1 Update `Dockerfile`: COPY `config.toml.example` to `/app/config.toml.example`
- [x] 7.2 Remove `.env` references from `.dockerignore` if present
- [x] 7.3 Update README docker run example to use `-v ./config.toml:/app/config.toml:ro`

## 8. Cleanup & Documentation

- [x] 8.1 Delete `.env.example`
- [x] 8.2 Delete `scripts/config.toml`
- [x] 8.3 Add `config.toml` to `.gitignore`, remove `.env` entry if present
- [x] 8.4 Update `CLAUDE.md`: replace all `.env`/dotenvy references with config.toml
- [x] 8.5 Update `README.md`: setup instructions, config section, docker examples

## 9. Verification

- [x] 9.1 `cargo build --release` succeeds without dotenvy
- [x] 9.2 `cargo clippy` passes clean
- [x] 9.3 Manual test: start server with config.toml, verify all endpoints work
- [x] 9.4 Manual test: Python crawler reads DB path and Gemini key from root config.toml
- [x] 9.5 Manual test: missing config.toml produces clear error message
- [x] 9.6 Manual test: admin_secret = "changeme" emits warning but starts
