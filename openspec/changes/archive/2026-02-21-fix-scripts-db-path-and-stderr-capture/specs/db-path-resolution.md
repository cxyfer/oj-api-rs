# Spec: DB Path Resolution

## R1: scripts/config.toml provides correct database path

**Given** Rust API spawns `uv run python3 leetcode.py --daily` with `current_dir("scripts/")`
**When** Python reads `database.path` from `scripts/config.toml`
**Then** the resolved absolute path equals `<project_root>/data/data.db`

### Property: Path equivalence

```
For any CWD where scripts/config.toml exists:
  resolve(config_dir / config.database_path) == resolve(RUST_DATABASE_PATH)
```

**Falsification**: Run `cd /tmp && python3 /path/to/scripts/leetcode.py --daily` — if db_path resolves to `/data/data.db` instead of `<root>/data/data.db`, the property is violated.

## R2: Config load failure does not crash

**Given** `scripts/config.toml` does not exist or contains invalid TOML
**When** `main()` attempts to load config
**Then** a warning is written to stderr AND `db_path` falls back to `<script_dir>/../data/data.db` resolved via `__file__`

### Property: Crash-freedom

```
For all config states ∈ {missing, empty, malformed, valid}:
  main() never raises an unhandled exception during config loading
```

**Falsification**: Delete `scripts/config.toml`, run `cd scripts && uv run python3 leetcode.py --daily`. Process must not crash; stderr must contain "Warning".

## R3: DB path is absolute before DB manager construction

**Given** `db_path` is obtained from config or fallback
**When** passed to `LeetCodeClient(db_path=...)`
**Then** `db_path` is an absolute path (starts with `/`)

### Property: Absolute path invariant

```
assert Path(db_path).is_absolute()
```

**Falsification**: Inject a relative path in config.toml `path = "data/data.db"` — the resolution logic must still produce an absolute path.

## R4: Idempotency of daily writes

**Given** the daily challenge for today already exists in the DB
**When** the crawler runs `--daily` again
**Then** no error is raised (the UNIQUE constraint issue is a separate pre-existing bug, but the crawler process itself should exit 0)

> Note: The UNIQUE constraint error in `database.py:1028` is tracked separately. This spec only asserts the process does not crash.
