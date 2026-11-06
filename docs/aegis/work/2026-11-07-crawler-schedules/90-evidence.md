# Crawler schedules verification evidence

Date: 2026-11-07 (Asia/Taipei)

## Automated verification

- `cargo test`: 235 passed (206 library, 13 admin integration, 14 public API
  integration, 2 health integration); exit 0.
- `cargo clippy --all-targets -- -D warnings`: clean; exit 0.
- `cargo fmt --check`: exit 0.
- `uv --directory scripts run --frozen pytest`: 101 passed; exit 0.
- `node --check static/admin.js` and
  `node --check static/crawler-schedules.js`: exit 0.
- Python compileall excluding the isolated virtual environment: exit 0.
- `git diff --check`: exit 0.
- Aegis workspace structural check: passed. This checks workspace shape only.
  The optional JSON proof-bundle generator does not support these Markdown-only
  records; no generated gate-input or proof-bundle sufficiency claim is made.

Cargo used a temporary writable CARGO_HOME because the host registry is
read-only. Each worktree retained its own target directory. No global toolchain
configuration or shared target was changed.

## Browser verification

Chromium, real Rust server, separate SQLite database and script artifact root
under `/tmp/oj-schedule-runtime`:

- Five independently editable source cards.
- Weekly form controls generate the expected five-field Cron expression.
- Basic to advanced round trip preserves generated Cron.
- Unsupported custom Cron survives switching to basic mode until a preset is
  explicitly selected.
- Invalid Cron and IANA timezone are shown as errors and do not replace the
  saved configuration.
- Save/reload preserves source enablement, timezone, Cron, and arguments.
- Switching languages preserves unsaved input.
- At 390px width the page has no horizontal overflow.
- No JavaScript page errors.
- Language changes update the preview and result labels without rebuilding
  forms.

Screenshots were inspected at `/tmp/oj-schedule-desktop.png` and
`/tmp/oj-schedule-mobile.png`. They are temporary local evidence, not source
assets.

## Scheduler runtime verification

The real scheduler triggered fixture crawlers at two consecutive Cron minute
boundaries. Both completed with `scheduled` metadata, output, and retained
history. The fixture substituted only the `uv` executable, so no online judge
request or production database write occurred.

After stopping and restarting the isolated server, the saved schedule and
retained scheduled history were restored. A subsequent preview showed exact
minute boundaries with zero seconds/nanoseconds.

A held manual fixture then occupied the same source across a Cron deadline.
The scheduled occurrence was skipped and its reason persisted; exactly one
same-source crawler remained running. The manual fixture was cancelled through
the existing endpoint and the test schedule was disabled.

## Review and boundaries

- Scheduler dispatch and settings updates share a lock, so a successful disable
  cannot be followed by dispatch using a stale setting.
- Manual, scheduled, and daily registration share a source launch lock.
- Old process completion can only mutate its own job id/runtime key.
- Shutdown prevents subsequent process launches before process-group cleanup.
- Shared terminal-state logic lives in `src/job.rs`; runtime execution has no
  dependency on admin handlers.
- No distributed scheduler claim and no live online-judge crawl acceptance.
- Existing daily-source schedule remains separate.
