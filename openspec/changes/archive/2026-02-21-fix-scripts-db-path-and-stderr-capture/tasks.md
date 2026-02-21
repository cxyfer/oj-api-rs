# Tasks: Fix Scripts DB Path Mismatch

## T1: Create `scripts/config.toml`

**File**: `scripts/config.toml` (NEW)

**Content** (exact):
```toml
[database]
path = "../data/data.db"
```

No other sections. No comments beyond what's needed.

**Verification**: File exists, valid TOML, `database.path` key equals `"../data/data.db"`.

- [x] Done

---

## T2: Wire db_path from config in `scripts/leetcode.py`

**File**: `scripts/leetcode.py`

**Location**: `main()` function, before `LeetCodeClient(...)` call (~line 1311).

**Exact change**:

Replace:
```python
    client = LeetCodeClient(data_dir="data")
```

With:
```python
    fallback_db = str((Path(__file__).resolve().parent / "../data/data.db").resolve())
    try:
        config = get_config()
        config_dir = Path(config.config_path).resolve().parent
        db_path = str((config_dir / config.database_path).resolve())
        scripts_dir = Path(__file__).resolve().parent
        if Path(db_path).parent == scripts_dir / "data":
            import sys
            sys.stderr.write(
                "Warning: config database.path resolves inside scripts/data/, using fallback.\n"
            )
            db_path = fallback_db
    except Exception as e:
        import sys
        sys.stderr.write(f"Warning: Failed to load config, using default db path. Error: {e}\n")
        db_path = fallback_db

    client = LeetCodeClient(data_dir="data", db_path=db_path)
```

**Note**: `get_config` is already imported at line 11. `Path` is already imported at line 5.

**Verification**:
1. `cd scripts && uv run python3 leetcode.py --daily` — check stderr for no config error, check that `data/data.db` in project root is modified (not `scripts/data/data.db`).
2. Delete `scripts/config.toml`, run same command — check stderr for "Warning: Failed to load config" and process completes without crash.

- [x] Done (with added validation for scripts/data/ path guard per Codex review)

---

## T3: Verify `Path` import exists in `leetcode.py`

**File**: `scripts/leetcode.py`

Check if `from pathlib import Path` already exists. If not, add it near the top imports.

**Verification**: No `ImportError` when running the script.

- [x] Done (confirmed at line 5: `from pathlib import Path`)

---

## T4: Delete stale `scripts/data/data.db` (manual / optional)

**Action**: After verifying T2, the stale `scripts/data/data.db` (11MB) can be safely removed. This is a manual cleanup step, not an automated code change.

```bash
rm scripts/data/data.db
```

**Verification**: `ls scripts/data/data.db` returns "No such file". API `/api/v1/daily` still works.

- [x] Done
