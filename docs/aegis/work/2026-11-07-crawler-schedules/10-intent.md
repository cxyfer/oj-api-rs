# Per-source crawler schedules intent

## TaskIntentDraft

- Outcome: persistent per-source Cron schedules in the admin backend.
- Success evidence: validated GET/PUT API, restart-safe storage, scheduled
  crawler jobs with existing artifacts/history, and basic/advanced admin UI.
- Stop condition: focused and full verification pass with no compatibility
  regression.
- Non-goals: diagnostic scheduling, replacing daily-source scheduling,
  distributed queue/locks, and external config-file editing.

## BaselineReadSetHint

- `CONTEXT.md`
- `config.toml.example`
- `src/models.rs`
- `src/admin/handlers/crawler.rs`
- `src/api/daily.rs`
- `src/db/settings.rs`
- `templates/admin/crawlers.html`
- `static/admin.js`
- `docs/aegis/specs/2026-11-07-crawler-schedules-design.md`

## BaselineUsageDraft

- Required baseline refs: existing crawler lifecycle, app settings, daily
  scheduler, admin page and test builders.
- Acknowledged before plan refs: all listed refs.
- Cited in plan refs: all listed refs.
- Missing refs: none.
- Decision: continue.

## ImpactStatementDraft

- Affected layers: config, model, SQLite settings, scheduler runtime, admin
  API, template, JavaScript, translations, tests.
- Canonical owner: shared crawler runner for execution; schedule module for
  timing and persistence.
- Compatibility: manual crawler, daily fallback, artifacts, history, public API.
- Risk: async wake-up, Cron/timezone semantics, and same-source overlap.

## Execution Readiness View

- Intent Lock: independent per-source enabled schedules using five-field Cron.
- Scope Fence: supported problem sources only; no diagnostic/daily-source change.
- Baseline Lock: reuse app settings, crawler artifacts, and current admin page.
- Approved Behavior: basic/advanced editors, per-source timezone, UTC default.
- Owner/Contract: shared crawler runner; schedule module; authenticated admin API.
- Compatibility Boundary: current manual/daily paths stay behaviorally stable.
- Retirement Boundary: only the duplicated manual-only runner path is removed.
- Task Batches: model/persistence; runner/scheduler; API; UI; verification.
- Test Obligations: parser, timezone, persistence, handlers, conflict skipping,
  template compilation, full suite.
- Review Gates: focused tests after each batch and final diff review.
- Drift/Rewind Rules: stop if scope expands to durable queue or public API.
- Evidence Required: exact command output, handler responses, and manual UI flow.
- Advisory Boundary: execution guidance only; not an authoritative gate.
