## Context

Dynamic single-problem fetching runs only after a supported problem detail request misses local storage. The current implementation starts `uv run python3 <crawler>.py --problem <id>` and wraps `cmd.output()` in a timeout. Existing long-running crawler jobs already use `spawn_with_pgid` and `kill_pgid` so timeout and cancellation terminate the subprocess tree, not only the direct child.

This change aligns the dynamic fetch path with that existing process-management pattern while preserving the public API contract: failed or timed-out dynamic fetches still fall back to the existing problem-not-found response.

## Goals / Non-Goals

**Goals:**

- Ensure dynamic single-problem crawler timeout terminates the full subprocess process group.
- Reuse existing process utilities instead of introducing a new process-management abstraction.
- Preserve successful dynamic fetch behavior, source/id validation, database re-read behavior, and not-found fallback semantics.
- Add focused regression coverage for timeout cleanup without depending on external judge network calls.

**Non-Goals:**

- Changing crawler CLI arguments or supported problem-source derivation.
- Changing public API status codes, response bodies, or endpoint paths.
- Adding a job record, progress artifact, cancellation endpoint, or admin UI for dynamic fetches.
- Reworking broad crawler job execution.

## Decisions

1. Use existing process-group helpers for dynamic fetch execution.

   `run_single_problem_crawler` will build the same command but spawn it with `crate::utils::spawn_with_pgid`. On timeout, it will call `crate::utils::kill_pgid(pid)` before completing the wait path. This matches the established crawler timeout behavior and avoids maintaining two subtly different subprocess policies.

   Alternative considered: keep `cmd.output()` with `kill_on_drop(true)`. This is simpler but does not explicitly terminate grandchildren created by `uv`, which is the defect being fixed.

2. Keep stdout discarded and stderr captured.

   Dynamic fetch does not expose job output. The implementation should preserve current output behavior: stdout can remain null, while stderr is captured for warning logs on non-success status. When moving from `cmd.output()` to explicit spawn/wait, use piped stderr and collect the child output through the spawned child wait API rather than adding persistent artifacts.

   Alternative considered: reuse the full admin crawler artifact capture pipeline. That would add unnecessary job/history surface area for a synchronous fallback path.

3. Preserve timeout result semantics.

   Timeout cleanup is an implementation guarantee; the API-visible outcome remains `None` from `fetch_problem_on_miss`, leading to the existing RFC 7807 404 response. Successful crawler completion still performs the existing DB re-read before returning any problem detail.

   Alternative considered: return a distinct timeout error. This would change API behavior and is outside the requested fix.

4. Test with an internal timeout fixture.

   Regression coverage should avoid external network calls. A test-only path can exercise the real timeout branch by running a small local child process through the dynamic crawler spawn path, then verify that process-group cleanup occurs and the fetch returns not found.

   Alternative considered: only unit-test helper functions. That would not cover the actual timeout branch that launches `uv` and manages the child process.

## Risks / Trade-offs

- Process-group helpers are Unix-specific, with no-op fallback for `kill_pgid` on non-Unix platforms → Keep existing cross-platform behavior and rely on the already established helper contract.
- Explicit spawn/wait code is slightly more verbose than `cmd.output()` → Prefer consistency with existing crawler timeout handling over brevity for subprocess-tree safety.
- Killing a process group after the child has already exited can race with normal completion → Treat missing process groups as harmless, matching existing `kill_pgid` behavior.
- Tests that spawn local processes can be timing-sensitive → Use short, deterministic fixtures and avoid network or external judge dependencies.
