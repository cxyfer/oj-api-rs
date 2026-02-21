# Design: Fix Scripts DB Path Mismatch

## Decision Record

### D1: Path Resolution Strategy

**Choice**: Resolve `db_path` to absolute path anchored to `config.toml` file location.

**Rationale**: `config.toml` lives in `scripts/`, and Rust spawns Python with `current_dir("scripts/")`. Resolving relative to config file location makes the path deterministic regardless of how the script is invoked.

**Implementation**: In `leetcode.py` main(), after loading config, resolve `db_path` against `Path(config.config_path).parent`:

```python
config_dir = Path(config.config_path).resolve().parent
db_path = (config_dir / config.database_path).resolve()
```

If `config.toml` contains `../data/data.db` and lives at `scripts/config.toml`, this resolves to `<root>/data/data.db`.

### D2: Config Load Failure Handling

**Choice**: Catch exception in main(), fallback to `__file__`-based default, log warning to stderr.

**Rationale**: The daily fallback crawler is called from Rust API automatically. A crash due to missing config would cause the API to return perpetual `"status": "fetching"`. Graceful degradation is required.

**Implementation**:

```python
try:
    config = get_config()
    config_dir = Path(config.config_path).resolve().parent
    db_path = str((config_dir / config.database_path).resolve())
except Exception as e:
    import sys
    sys.stderr.write(f"Warning: Failed to load config, using default db path. Error: {e}\n")
    db_path = str((Path(__file__).resolve().parent / "../data/data.db").resolve())
```

The `__file__` fallback ensures correct resolution even without config.toml.

### D3: data_dir Scope

**Choice**: Not changed in this iteration. Stays hardcoded as `"data"` at call site.

**Rationale**: `data_dir` controls JSON cache files (not the DB). It resolves correctly relative to `scripts/` CWD because Rust sets `current_dir("scripts/")`. No bug here — deferred to a future consistency pass.

### D4: config.toml Content

**Choice**: Minimal — only `[database]` section.

**Rationale**: Python scripts invoked by Rust API need only the DB path. Discord tokens, LLM keys, etc. are irrelevant for crawler-only usage. A minimal config avoids exposing secrets in the scripts directory.

```toml
[database]
path = "../data/data.db"
```

## Architecture Invariants

- `db_path` MUST be resolved to absolute before passing to any DB manager constructor.
- Config loading failure MUST NOT crash the process; it MUST fallback with a warning.
- The resolved `db_path` MUST point to the same file that Rust API reads via `DATABASE_PATH` env / default `data/data.db`.
