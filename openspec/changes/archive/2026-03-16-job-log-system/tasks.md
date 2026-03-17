## 1. Shared artifact foundation

- [x] 1.1 Add shared job artifact data models and helper utilities for canonical `scripts/logs/{job_type}/{job_id}/` paths, lossy text decoding, and metadata-in-`progress.json` persistence needed for restart reconstruction.
- [x] 1.2 Implement live tee capture and bounded tail buffering for Rust-launched subprocesses so `stdout.log` and `stderr.log` grow during execution without creating new flat log files.
- [x] 1.3 Implement startup and pre-launch retention/history reconstruction that scans canonical job directories, skips running jobs, preserves root date logs and legacy flat files, and removes expired jobs from retained history.

## 2. Crawler-domain runtime and fallback integration

- [x] 2.1 Replace singleton crawler runtime state with a keyed crawler-domain registry plus a separate manual-trigger admission guard so one manual crawler and concurrent daily fallback jobs can coexist.
- [x] 2.2 Refactor manual crawler execution to create canonical job directories, inject `OJ_JOB_*` env vars, persist hybrid crawler `progress.json`, and keep terminal transitions idempotent.
- [x] 2.3 Refactor daily fallback execution to use the same crawler-domain helper, emit `trigger=daily_fallback` metadata, and participate in shared history/output/progress surfaces.
- [x] 2.4 Restrict `POST /admin/api/crawlers/cancel` to the active manual crawler job and keep timeout/cancel/finalize races from regressing terminal state.

## 3. Embedding and Python logging integration

- [x] 3.1 Extend `scripts/utils/logger.py` to add an ANSI-clean per-job `python.log` handler while preserving ANSI-colored root date logs.
- [x] 3.2 Refactor embedding execution to use canonical job directories, live capture, and `progress.json` as the authoritative source for queued/running/final states plus succeeded/skipped/failed summary.
- [x] 3.3 Update embedding CLI and crawler-script integration points so scripts can optionally append canonical running/message progress without overwriting Rust-owned queued or terminal states.

## 4. Admin API and UI updates

- [x] 4.1 Rewrite crawler status/output/progress endpoints to serve `running_jobs` and retained history from canonical artifacts, always return `stdout`, `stderr`, and `python_log`, and return 404 after cleanup.
- [x] 4.2 Rewrite embedding status/output/progress endpoints to read canonical artifacts, return `queued` before first progress write, and hide cleaned-up jobs from history.
- [x] 4.3 Update `templates/admin/crawlers.html`, `templates/admin/embeddings.html`, and `static/admin.js` to poll running jobs, show shared crawler history with trigger labels, and add dedicated `python.log` tabs with live modal refresh.

## 5. Verification

- [x] 5.1 Add property-focused backend tests for canonical artifact creation, monotonic log growth, terminal phase monotonicity, and cleanup protection rules.
- [x] 5.2 Add reconstruction and endpoint tests covering restart-preserved history, empty-string output fields, queued-before-progress behavior, concurrent manual+daily crawler visibility, and embedding summary/status contracts.
- [x] 5.3 Run `cargo fmt`, `cargo clippy`, `cargo test`, and targeted admin manual checks for live crawler/embedding log polling, daily fallback visibility, and retention behavior.
