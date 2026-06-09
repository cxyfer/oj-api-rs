## Why

Dynamic single-problem fetching currently runs crawler subprocesses through `uv run python3` with a timeout, but timeout only returns a failed API fallback result and does not explicitly terminate the subprocess tree. This can leave the Python crawler running after the API request has returned, allowing continued network work or later SQLite writes despite the timeout.

## What Changes

- Update dynamic single-problem crawler execution to use the same process-group spawning and timeout cleanup pattern used by existing crawler jobs.
- Ensure timeout handling kills the entire dynamic crawler process group before returning the existing not-found fallback behavior.
- Preserve current source validation, direct-fetch planning, successful fetch behavior, and RFC 7807 404 behavior when the crawler fails or times out.
- Add regression coverage for timeout cleanup behavior without broad crawler sync or external network dependencies.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dynamic-problem-fetch`: Clarify that bounded dynamic single-problem crawler execution terminates the crawler process tree on timeout.

## Impact

- Affected code: `src/dynamic_problem.rs` and tests.
- Existing process utilities: reuse `crate::utils::spawn_with_pgid` and `crate::utils::kill_pgid`.
- Public API behavior remains unchanged: a timed-out dynamic fetch still results in the existing problem-not-found response.
- No new external dependencies or API endpoints.
