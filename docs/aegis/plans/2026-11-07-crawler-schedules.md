# Per-source crawler schedules

## Goal

Add persistent per-source Cron schedules to the admin crawler backend and page.
Each supported problem source can be enabled independently, use a five-field
Cron expression, select an IANA timezone with `UTC` as the default, and pass
validated crawler arguments.

## Architecture

`src/crawler_schedule.rs` owns the schedule model, validation, persistence
serialization, next-occurrence calculation, and background scheduler. The
existing crawler handler is refactored to expose one shared process runner for
manual and scheduled jobs. `src/admin/handlers/settings.rs` remains the owner
of unrelated token settings; crawler schedule endpoints live beside the
crawler handlers. The admin template and `static/admin.js` own the two editor
modes and status display.

The scheduler stores configuration in the existing `app_settings` key
`crawler_schedules`, uses a `Notify` in `AppState` to wake on updates, and
reuses the existing job artifacts and history. The `croner` crate parses
standard five-field Cron directly with Sunday 0/7 and DOM/DOW OR semantics. `chrono-tz` resolves configured IANA timezones.

## Tech Stack

Rust 2021, Axum 0.8, Tokio, SQLite via rusqlite/r2d2, Chrono, Askama, and the
existing vanilla JavaScript admin UI. Add `croner` and `chrono-tz` dependencies.

## Baseline/Authority Refs

- `CONTEXT.md`
- `config.toml.example`
- `src/models.rs`
- `src/admin/handlers/crawler.rs`
- `src/api/daily.rs`
- `src/db/settings.rs`
- `templates/admin/crawlers.html`
- `static/admin.js`
- `docs/aegis/specs/2026-11-07-crawler-schedules-design.md`

Requirement Ready Check: ready. The approved design fixes per-source
enablement, five-field Cron, basic/advanced editing, per-source timezone, UTC
default, persistence, conflict skipping, and the scope exclusions.

## Compatibility Boundary

Manual crawler triggering and cancellation keep their current API and behavior.
Daily fallback and additional daily-source scheduling remain unchanged.
Existing crawler artifacts, status/history response fields, and public routes
remain compatible. The diagnostic crawler is not included in persisted
schedules.

## Verification

Run focused Rust unit tests for schedule parsing and persistence, admin handler
tests for schedule GET/PUT and invalid input, scheduler tests for conflict
skipping and wake-up behavior, then run:

```bash
cargo fmt --check
cargo test
python3 -m compileall scripts
```

## Change Necessity

- User-visible need: administrators need recurring source-specific crawling
  without external cron configuration.
- No-change option: documenting shell cron commands leaves no admin control,
  persistence, status, or validation.
- Minimum code boundary: schedule model/persistence, a shared crawler runner,
  scheduler startup, admin endpoints, and the existing crawler page.
- Decision: `code-change`.

## Existence Check

- Proposed new surface: one general crawler schedule owner.
- Existing reuse candidates: the daily-source scheduler and manual crawler
  handler.
- Why insufficient: the daily scheduler is source-specific and fixed-time; the
  manual handler owns HTTP response flow and a single runtime key.
- Creation proof: per-source persisted configuration and independent wake-up
  are required by the approved behavior.
- Entropy impact: extract shared process execution; do not duplicate crawler
  subprocess lifecycle logic.
- Decision: `add-with-proof`.

## Architecture Integrity Lens

- Invariant: all crawler jobs use one process/artifact/history lifecycle.
- Canonical owner: a shared crawler runner owns subprocess execution; the
  scheduler only decides when and whether to launch.
- Responsibility overlap: remove manual-only assumptions from the runner while
  keeping manual guard/cancel logic in the HTTP handler.
- Higher-level simplification: use one `CrawlerSchedule` validation path for
  both basic and advanced UI modes.
- Verdict: aligned after extracting the shared runner.

## Plan-Time Complexity Check

- Target files: `src/crawler_schedule.rs`, `src/admin/handlers/crawler.rs`,
  `src/models.rs`, `src/lib.rs`, `src/main.rs`, admin template/JS/i18n, tests.
- Pressure: crawler handler already contains a long subprocess lifecycle and
  schedule UI adds multiple async states.
- Recommendation: add a dedicated schedule owner and extract the runner;
  avoid duplicating the lifecycle in the scheduler.

## Execution route

Inline execution in the existing task-bound feature worktree. TDD Mode: off;
Decision: skipped; Strict authority: none. Tests verify behavior and regressions
without claiming a strict RED/GREEN sequence. The user authorized implementation
and the default feature-branch push/PR lifecycle; no additional execution-choice
gate is needed. The refactor also extracts `src/job.rs` for terminal-state logic
shared by crawler and embedding jobs so the runtime does not depend on admin
handlers. Baseline branch is freshly verified `origin/dev` at `81c488d`.

## Tasks

### 1. Add schedule dependencies, model, validation, and persistence

Files: `Cargo.toml`, `Cargo.lock`, `src/config.rs`, `src/models.rs`,
`src/db/settings.rs`, `src/crawler_schedule.rs`, unit tests in the new module.

Why: the backend needs one validated contract that survives restart and
supports five-field Cron plus IANA timezones.

Change necessity: source edits are required because config-file-only settings
cannot be edited from the admin page or wake the running scheduler.

Impact: add `CrawlerTrigger::Scheduled`, `CrawlerSchedule` and default UTC
configuration. Keep unknown persisted source entries ignored when loading, and
reject invalid new updates atomically.

Verification:

```bash
cargo test crawler_schedule
```

Steps:

- [x] Write unit tests for five-field normalization, UTC/default values,
  timezone parsing, next occurrences, and JSON round trips.
- [x] Run `cargo test crawler_schedule` and inspect validation results.
- [x] Implement the model, validation, persistence helpers, and dependencies.
- [x] Run `cargo test crawler_schedule` and confirm all focused tests pass.
- [x] Commit with the repository's gitmoji Conventional Commit style.

### 2. Extract a shared crawler runner and add scheduler startup

Files: `src/admin/handlers/crawler.rs`, `src/admin/handlers/common.rs`,
`src/crawler_schedule.rs`, `src/lib.rs`, `src/main.rs`, test state builders.

Why: scheduled and manual jobs must share timeout, process-group cleanup,
artifacts, output capture, progress, and history behavior.

Change necessity: duplicating the existing subprocess lifecycle would create
divergent job behavior; the minimum stable repair is one shared runner with
manual guard behavior kept at the handler boundary.

Impact: manual endpoint response and cancellation remain unchanged. Scheduled
jobs use `scheduled:<source>` runtime keys and skip if any same-source job is
running. Scheduler shutdown is covered by existing process-group cleanup.

Verification:

```bash
cargo test --lib
```

Steps:

- [x] Add runner-focused tests for scheduled trigger metadata, artifact
  persistence, same-source conflict checks, and manual compatibility.
- [x] Run the focused tests and inspect runner compatibility.
- [x] Extract the generic launch/finalize path and implement the scheduler
  wait/notify loop.
- [x] Run focused tests and confirm manual and scheduled paths pass.
- [x] Commit the runner/scheduler slice.

### 3. Add authenticated schedule API and OpenAPI coverage

Files: `src/admin/mod.rs`, `src/admin/handlers/schedule.rs`,
`src/admin/handlers/tests.rs`, `tests/admin_test.rs`.

Why: the admin UI needs a stable read/update contract with clear validation
errors and atomic persistence.

Change necessity: no existing endpoint exposes persistent crawler settings.

Impact: add `GET /admin/api/crawler-schedules` and
`PUT /admin/api/crawler-schedules`; existing crawler endpoints stay unchanged.

Verification:

```bash
cargo test --test admin_test
cargo test --lib admin::handlers::tests
```

Steps:

- [x] Add handler tests for defaults, valid updates, invalid Cron/timezone/args,
  and unchanged state after rejected updates.
- [x] Run the focused tests and inspect endpoint behavior.
- [x] Implement request/response types, validation, persistence, and scheduler
  notification.
- [x] Run the focused tests and confirm they pass.
- [x] Commit the API slice.

### 4. Add basic/advanced schedule controls and localized labels

Files: `templates/admin/crawlers.html`, `static/crawler-schedules.js`, `static/admin.js`, `static/admin.css`,
`static/i18n/zh-TW.json`, `static/i18n/zh-CN.json`, `static/i18n/en.json`.

Why: administrators need understandable recurring schedules without giving up
raw Cron control.

Change necessity: a backend-only endpoint cannot provide the requested admin
workflow.

Impact: preserve existing manual crawler controls, status polling, history, and
log modal. Add per-source schedule rows, enable toggles, basic form generation,
advanced Cron input, timezone selection, next-run preview, and save/error
states.

Verification:

```bash
cargo test
```

Then manually open `/admin/crawlers` and verify both editor modes, disabled
rows, invalid Cron feedback, and save/reload persistence.

Steps:

- [x] Add DOM structure and translation keys for schedule rows and editor
  states.
- [x] Run the existing Rust suite to preserve the template compilation
  baseline.
- [x] Implement API loading, basic-to-Cron conversion, advanced validation,
  timezone controls, and update rendering.
- [x] Run the suite and manually verify the admin flow.
- [x] Commit the UI slice.

### 5. Run complete verification and inspect the final diff

Files: no product files unless verification exposes a directly related defect.

Why: the feature crosses persistence, async scheduling, subprocess execution,
API contracts, and UI state.

Verification:

```bash
cargo fmt --check
cargo test
python3 -m compileall scripts
git diff --check
git status --short
```

Confirm the final diff contains no secrets, no duplicate scheduler lifecycle,
and no changes to the public API or daily fallback behavior.

## Risks and rollback

- Cron parser field differences are covered by direct five-field parsing and
  tests for field counts, Sunday aliases, and DOM/DOW matching.
- DST behavior is delegated to the configured IANA timezone and surfaced by
  next-run previews.
- A malformed persisted value must disable only the affected source and log a
  warning; it must not prevent startup.
- Rollback removes the scheduler startup, endpoints, UI, and setting key while
  leaving existing crawler artifacts and jobs readable.

## Retirement

No existing owner is deleted. The fixed additional daily-source scheduler
remains active because it serves a distinct daily ingestion contract. The
manual-only subprocess path is retired after the shared runner is verified;
its guard and cancellation behavior remain at the handler boundary.

## Execution result

All slices were verified together and will be recorded in one coherent feature
commit. The final evidence and runtime boundaries are recorded in
`docs/aegis/work/2026-11-07-crawler-schedules/90-evidence.md`.
