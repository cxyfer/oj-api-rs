# Per-source crawler schedules checkpoint

- Current todo: commit/push/PR.
- Completed: per-source persistence/validation, shared runner, scheduler,
  authenticated GET/PUT/preview API, localized basic/advanced admin controls.
- Active slice: integration; tests, lint, browser, restart, and same-source
  runtime conflict verification passed.
- Worktree: existing task-bound workspace, branch
  `feat/admin-scheduled-problem-scraping`, starting HEAD `81c488d`.
- Base: fetched `origin/dev`; repository default branch `master` is not the
  development PR target. No unrelated source delta existed before the task.
- TDD route: off / skipped; behavioral and regression tests added.
- Evidence: latest Rust suite 235 passed; initial browser flow passed all
  assertions; actual timer launched two completed fixture jobs at consecutive
  minute boundaries in a temporary database. No external crawler website was
  contacted by the runtime fixture.
- Drift check: standard five-field Cron uses croner to avoid differing weekday
  and DOM/DOW semantics. Basic/advanced controls save one contract. Daily-source
  scheduler remains separate. Same-source launch checks share a registration
  lock; shutdown stops further scheduled launches.
- Architecture review: shared runner extracted from admin handlers; shared
  terminal output logic moved to `src/job.rs`; admin handlers depend on runtime
  modules, not vice versa. Existing manual ownership tests exercise the shared
  helpers rather than an unused copy.
- ADR backfill: skip a separate ADR; these are reversible feature additions and
  extractions into the existing process lifecycle, not a durable platform
  change. README plus the approved design document the current owners and
  persistence contract. Existing homepage ADR/baseline is unaffected.
- Blockers: none.
- Next: commit only task-owned paths, push feature branch, open PR against dev.
