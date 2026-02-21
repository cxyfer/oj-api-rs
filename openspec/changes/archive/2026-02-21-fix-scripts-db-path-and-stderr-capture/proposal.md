# Fix: Scripts DB Path Mismatch & Stderr-Only Capture

## Context

Two related bugs caused by the crawler execution model:

1. **DB path mismatch**: Rust spawns Python crawlers with `current_dir("scripts/")`, but `LeetCodeClient(data_dir="data")` resolves `db_path` to `scripts/data/data.db` (relative to CWD). The Rust API reads from `data/data.db` (project root). Result: Python writes to a **different** database; API never sees the data.

2. **Stderr capture scope**: The admin crawlers page wants to display Python logger output. Python loggers write to **stderr** (via `logging.StreamHandler` → stderr). Current Rust code already captures stderr, but the user wants **only stderr** rendered (not stdout mixed in), with log-level-aware styling.

## Root Cause Analysis

### DB Path

| Component | CWD at execution | Relative DB path | Absolute resolved path |
|-----------|-----------------|-------------------|----------------------|
| Rust API | project root | `data/data.db` | `<root>/data/data.db` |
| Python (spawned) | `scripts/` | `./data/data.db` (default) | `<root>/scripts/data/data.db` |

`leetcode.py:1311` — `LeetCodeClient(data_dir="data")` uses default `db_path="./data/data.db"`.
No `config.toml` exists in `scripts/` → `get_config()` throws `FileNotFoundError` → logger falls back to defaults, but the `LeetCodeClient` constructor **never reads config for db_path**; it's hardcoded in the call site.

### Stderr

Python `logging.StreamHandler()` defaults to `sys.stderr`. The `ColoredFormatter` in `logger.py` embeds ANSI color codes. Rust captures both stdout and stderr via `Stdio::piped()`. Both are stored in `CrawlerJob.stdout` / `CrawlerJob.stderr`. The frontend needs to render **only** the `stderr` field with log-level color mapping.

## Requirements

### R1: Create `scripts/config.toml` with correct DB path

- **Constraint**: Must point `database.path` to `../data/data.db` (relative to `scripts/` CWD).
- **Constraint**: File must be minimal — only `[database]` section needed for crawler-only usage (no Discord token, no LLM keys).
- **Constraint**: Must follow the same schema as `references/leetcode-daily-discord-bot/config.toml.example`.

### R2: Wire `LeetCodeClient` to read DB path from config

- **Constraint**: `leetcode.py` main entrypoint (`main()`) must read `database.path` from `get_config()` and pass it to `LeetCodeClient(db_path=...)`.
- **Constraint**: `LeetCodeClient.__init__` signature already accepts `db_path` param — no API change needed.
- **Constraint**: The fallback default in `LeetCodeClient.__init__` can stay as-is for backwards compat; the fix is at the call site.
- **Constraint**: `data_dir` should also come from config or be made consistent (currently hardcoded `"data"` at call site, defaults to `"./data"` in constructor).

### R3: Stderr rendering in admin UI (frontend constraint only)

- **Constraint**: Rust backend already stores `stderr` separately in `CrawlerJob.stderr` — no backend change needed.
- **Constraint**: Frontend should render `stderr` field with log-level-aware coloring.
- **Constraint**: Python log format: `YYYY-MM-DD HH:MM:SS | LEVEL    | file:line                  | message`. Parse on `|` delimiter.
- **Constraint**: Color mapping: INFO=cyan, WARNING=yellow, ERROR=red, CRITICAL=red-bg, DEBUG=green.
- **Constraint**: Lines not matching the log pattern (e.g., "Warning: Failed to load...") should render as plain text.
- **OUT OF SCOPE for this change**: Frontend rendering. This change focuses on backend data availability.

## Success Criteria

1. `scripts/config.toml` exists with `database.path = "../data/data.db"`.
2. Running `cd scripts && uv run python3 leetcode.py --daily` writes to `<root>/data/data.db`, **not** `scripts/data/data.db`.
3. After fallback crawler runs, `/api/v1/daily` returns the challenge data (not perpetual `"status": "fetching"`).
4. `CrawlerJob.stderr` contains the Python logger output; `CrawlerJob.stdout` contains the JSON result.
5. The `scripts/data/data.db` stale copy can be safely deleted after migration.

## Risks

- **UNIQUE constraint error**: The stderr log shows `UNIQUE constraint failed: daily_challenge.date, daily_challenge.domain`. This occurs because the daily challenge was already inserted earlier (Python logs "written to database" after the error). This is a pre-existing issue in `database.py:1028` — the `update_daily` uses INSERT without `ON CONFLICT`. Not in scope but should be tracked.

## Files to Modify

| File | Change |
|------|--------|
| `scripts/config.toml` (NEW) | Minimal config with `database.path = "../data/data.db"` |
| `scripts/leetcode.py` | Read `db_path` from config at call site (line ~1311) |

## Out of Scope

- Frontend log rendering (separate change)
- Fixing the UNIQUE constraint error in `database.py`
- Changes to `atcoder.py` / `codeforces.py` (they accept `--db-path` CLI arg; different pattern)
