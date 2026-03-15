from __future__ import annotations

import json
import os
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional

TERMINAL_PHASES = {"completed", "failed", "cancelled", "timed_out"}


def _progress_path_from_env() -> Optional[Path]:
    value = os.getenv("OJ_PROGRESS_PATH")
    if value is None:
        return None
    value = value.strip()
    if not value:
        return None
    return Path(value)


def _read_existing_progress(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as f:
            data = json.load(f)
        return data if isinstance(data, dict) else {}
    except FileNotFoundError:
        return {}
    except (json.JSONDecodeError, OSError):
        return {}


def append_crawler_progress(message: Optional[str] = None) -> None:
    path = _progress_path_from_env()
    if path is None:
        return

    path.parent.mkdir(parents=True, exist_ok=True)
    existing = _read_existing_progress(path)
    existing_phase = existing.get("phase")
    if existing_phase in TERMINAL_PHASES:
        return

    payload = dict(existing)
    payload["phase"] = "running"
    if message is not None:
        payload["message"] = message
    payload["updated_at"] = datetime.now(timezone.utc).isoformat()

    fd, tmp_name = tempfile.mkstemp(dir=path.parent, suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump(payload, f, indent=2, ensure_ascii=False, sort_keys=True)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp_name, path)
    except Exception:
        try:
            os.unlink(tmp_name)
        except OSError:
            pass
        raise
