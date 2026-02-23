## Context

The project has a split configuration system: Rust reads flat env vars from `.env` via `dotenvy`, Python reads `scripts/config.toml` via `ConfigManager`. Two config sources can drift out of sync. Secrets are scattered across env vars. The Python `ConfigManager` carries dead code from a Discord bot project.

Three Rust call sites spawn Python subprocesses (`src/api/similar.rs`, `src/api/daily.rs`, `src/admin/handlers.rs`), all using `cmd.current_dir("scripts/")`. The `similar.rs` handler manually passes `GEMINI_API_KEY` via `cmd.env()`.

## Goals / Non-Goals

**Goals:**
- Single `config.toml` at project root as sole config source for both runtimes
- Rust config via `toml` crate + `serde::Deserialize` with nested structs and defaults
- Python `ConfigManager` reads shared config via fixed relative path `../config.toml`
- Remove `dotenvy` dependency and `.env` workflow
- Clean up Discord bot dead code from Python `ConfigManager`
- Docker compatibility via volume mount

**Non-Goals:**
- Rust does NOT parse `[gemini]` section — all Gemini config is Python-only
- No per-model api_key/base_url fallback logic in Rust
- No config hot-reload — restart required for config changes
- No migration tooling — manual copy from `.env` to `config.toml`

## Decisions

### D1: TOML + serde over dotenvy

**Choice**: Replace `dotenvy` + `env::var` with `toml` crate + `serde::Deserialize`.

**Alternatives considered**:
- Keep split config (.env + scripts/config.toml): minimal churn but drift risk remains
- All-env-vars for both runtimes: poor fit for nested model config (gemini.models.embedding.*)
- Two separate TOML files: still duplicates shared fields

**Rationale**: Single TOML naturally supports nested structures needed by Python's Gemini model config. Both runtimes read the same file, eliminating drift.

### D2: Lenient deserialization (no deny_unknown_fields)

**Choice**: Rust ignores unknown TOML keys/sections.

**Rationale**: The shared config.toml contains Python-only sections (`[gemini]`, potentially `[similar]` in future). Strict mode would break Rust whenever Python adds a new section. Lenient mode allows each runtime to read only what it needs.

### D3: Fixed relative path for Python config discovery

**Choice**: Python `ConfigManager` defaults to `../config.toml` (relative to `scripts/` cwd).

**Alternatives considered**:
- `--config` CLI arg: requires modifying all 3 Rust subprocess call sites and Python argparse
- `OJ_CONFIG_PATH` env var: works but less explicit than filesystem convention
- Parent directory walk: over-engineered for a known project structure

**Rationale**: All Python subprocesses are launched with `cmd.current_dir("scripts/")`. The project root is always one level up. Simple, predictable, no Rust changes needed for the default case. `CONFIG_PATH` env var forwarded only when set on the parent process.

### D4: Rust does not parse [gemini] section

**Choice**: Remove `gemini_api_key` from Rust `Config` struct entirely.

**Rationale**: Rust never calls Gemini API directly. The only usage was `cmd.env("GEMINI_API_KEY", key)` in `similar.rs` to pass the key to Python. After this change, Python reads the key from config.toml itself. Keeping Gemini config in Rust would be dead code.

### D5: admin_secret warning-only for unsafe values

**Choice**: Emit `tracing::warn!` for empty or "changeme" admin_secret, but still start.

**Alternatives considered**:
- Fatal exit: safer but breaks local development workflow where users may not care about the secret

**Rationale**: Current behavior already allows startup with any ADMIN_SECRET value. Warning preserves dev ergonomics while making the risk visible.

### D6: Database path resolution relative to config file directory

**Choice**: Both Rust and Python resolve `database.path` relative to the config file's parent directory.

**Rationale**: Rust runs from project root, Python runs from `scripts/`. If both resolve relative to their own cwd, they'd point to different files. Resolving relative to config file location guarantees both point to the same DB. For the default case (`data/data.db` with config at project root), this is equivalent to current behavior.

### D7: Embedding concurrency configurable with bounds

**Choice**: `embedding.concurrency` field (default 4) with validation range 1..=32.

**Rationale**: Currently hardcoded as `Semaphore::new(4)`. Making it configurable allows tuning without code changes. Bounds prevent misconfiguration: 0 would deadlock, >32 risks process explosion and Gemini rate limits.

## Risks / Trade-offs

- **[Python singleton timing]** → `get_config()` is a cached singleton. If any module calls it at import time before the default path is set, it could load from wrong location. Mitigation: audit all import-time `get_config()` calls; `ConfigManager.__init__` now uses `../config.toml` as default, which is correct for all subprocess launches from `scripts/`.

- **[Key migration breakage]** → `llm.gemini.*` → `gemini.*` affects all Python config reads. Mitigation: update all call sites in one atomic change; `get_embedding_model_config()` and `get_rewrite_model_config()` are the primary consumers.

- **[Docker startup failure]** → Config file is now required (no env-var-only mode). Users who relied on `-e ADMIN_SECRET=x` alone will get a startup error. Mitigation: clear error message with instructions; `config.toml.example` included in image.

- **[Path resolution edge case]** → If `CONFIG_PATH` points to a file in a different directory, relative `database.path` resolves relative to that directory, not project root. Mitigation: document this behavior; users of `CONFIG_PATH` should use absolute `database.path`.

## Migration Plan

1. Create `config.toml.example` at project root with all sections and defaults
2. Update Rust `src/config.rs`: new structs, TOML loader, remove dotenvy
3. Update `Cargo.toml`: add `toml`, remove `dotenvy`
4. Update `src/main.rs`: new config init flow, RUST_LOG from config, Semaphore from config
5. Update `src/api/similar.rs`: remove `cmd.env("GEMINI_API_KEY", key)`, remove `gemini_api_key` usage
6. Update Python `scripts/utils/config.py`: new default path, key migration, dead code removal
7. Update all Python scripts using old `llm.gemini.*` keys
8. Update Dockerfile: copy `config.toml.example`, remove `.env` references
9. Delete `.env.example`, `scripts/config.toml`
10. Update `CLAUDE.md`, `README.md`

**Rollback**: Revert the commit. No data migration involved — config is stateless.

## Open Questions

None — all ambiguities resolved during planning phase.
