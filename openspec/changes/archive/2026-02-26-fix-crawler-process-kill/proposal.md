# Proposal: Fix crawler process not killed on timeout

## Context

When running `luogu --fill-missing-content` via admin API, the Rust server reports `TimedOut` but the Python crawler process continues running in the background. Logs keep printing after the timeout fires.

## Root Cause

`kill_on_drop(true)` only sends SIGKILL to the **direct child** (the `uv` process). However, `uv run python3 <script>.py` spawns a child process tree:

```
uv (PID X)          ← receives SIGKILL
  └─ python3 (PID Y)  ← orphaned, keeps running
```

Since no **process group** is set, SIGKILL only hits `uv`, and `python3` becomes an orphan adopted by init — continuing to run indefinitely.

This affects **all 4 spawn sites** in the codebase:

| Location | Trigger | Has Timeout |
|---|---|---|
| `src/admin/handlers.rs:333` | Admin crawler | Yes (configurable) |
| `src/admin/handlers.rs:698` | Embedding job | **No** |
| `src/api/daily.rs:117` | Daily fallback | Yes (configurable) |
| `src/api/similar.rs:178` | Embedding query | Yes (fixed) |

Additionally:
- There is no explicit `child.kill()` call on timeout — it relies entirely on `Child` being dropped.
- Embedding jobs (`handlers.rs:728`) have **no timeout at all**.
- There is no admin API to manually cancel a running crawler.

## Requirements

### R1: Kill entire process tree on timeout (all 4 spawn sites)
- Extract a shared helper that spawns `uv` in its own process group (`pre_exec` with `libc::setpgid(0, 0)`)
- On timeout (or cancel), send SIGKILL to the **negative PID** (`kill(-pgid, SIGKILL)`) to kill all descendants
- Explicit `child.kill()` before drop as defense-in-depth
- Apply to all 4 spawn sites: admin crawler, embedding job, daily fallback, embedding query

### R2: Add admin cancel endpoint
- `POST /admin/api/crawlers/cancel` — kills the currently running crawler
- Sets status to a new `Cancelled` variant
- Reuses the same process-group kill logic from R1

### R3: Add timeout to embedding jobs
- Apply the same `tokio::time::timeout` pattern used by crawlers to embedding jobs (`handlers.rs:728`)
- Use a configurable timeout (default: same as crawler timeout)

## Success Criteria

1. After timeout fires, `ps aux | grep luogu` shows **no** surviving Python processes
2. Admin can cancel a running crawler via API and the process tree is fully terminated
3. Embedding jobs respect timeout and do not run indefinitely
4. All existing crawler/embedding functionality remains unchanged when operating within timeout

## Scope

- `src/admin/handlers.rs` — spawn logic, cancel endpoint, embedding timeout
- `src/admin/mod.rs` — route registration for cancel endpoint
- `src/models.rs` — add `Cancelled` status variant
- `src/api/daily.rs` — apply process group kill to daily fallback spawns
- `src/api/similar.rs` — apply process group kill to embedding spawns (if applicable)
