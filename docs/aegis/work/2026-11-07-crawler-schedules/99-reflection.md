# Crawler schedules closeout

- Implemented the approved source-by-source Cron controls with independent
  enablement, basic/advanced editors, inherited UTC timezone, previews, and
  persistent settings.
- Kept timing, process lifecycle, terminal-state handling, and admin response
  responsibilities separate; existing crawler jobs reuse their artifacts and
  manual cancellation behavior.
- Replaced the initial parser selection before shipping because standard
  weekday numbering and DOM/DOW semantics need to match the UI contract.
- Validation covers parser/timezone/persistence, real admin routes, runner
  ownership and conflicts, browser interaction, restart recovery, and real
  timer dispatch with a fixture process.
- ADR action: skip a separate ADR for this reversible feature; current owners
  and persistence/runtime limits are documented in README and the approved
  design. The existing homepage ADR and baseline remain unchanged.
- Residual boundary: real online-judge network behavior and production deployment
  are outside this change's verification. Multi-process scheduling remains out
  of scope.
- Feature integration is push/PR against dev; merge requires separate user
  authorization. The task-bound worktree is retained for review.
