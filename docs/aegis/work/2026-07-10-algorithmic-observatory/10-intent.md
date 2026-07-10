# Algorithmic Observatory Implementation - Intent

## TaskIntentDraft

- Requested outcome: Implement the approved Algorithmic Observatory homepage and MCP reference redesign.
- Goal: Ship a world-class, performant homepage and MCP work surface without changing Scalar or public runtime contracts.
- Success evidence:
- Rust checks pass; desktop/mobile browser evidence is clean; WebGL canvas is nonblank and adaptive; Scalar remains unchanged.
- Stop condition: Done when all plan tasks and verification evidence pass; otherwise stop as blocked, needs-verification, or scope-exceeded.
- Non-goals:
- No frontend framework, live homepage search, new marketing page, or backend endpoint.
- Scope: Public homepage, MCP reference, shared public assets, locales, render tests, and browser verification.
- Change kinds:
- feature
- Risk hints:
- WebGL performance, CSS owner split, localization overflow, stale Bento retirement.

## BaselineReadSetHint

- docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md
- docs/aegis/plans/2026-07-10-algorithmic-observatory-implementation.md
- openspec/specs/homepage-api-guide/spec.md

## BaselineUsageDraft

- Required baseline refs:
- docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md
- docs/aegis/plans/2026-07-10-algorithmic-observatory-implementation.md
- openspec/specs/homepage-api-guide/spec.md
- Acknowledged before plan:
- none
- Cited in plan:
- none
- Missing refs:
- docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md
- docs/aegis/plans/2026-07-10-algorithmic-observatory-implementation.md
- openspec/specs/homepage-api-guide/spec.md
- Advisory decision: needs-baseline-readback

## ImpactStatementDraft

- Compatibility boundary: No API, MCP transport, OpenAPI, database, auth, crawler, admin, or Scalar behavior changes.
- Affected layers:
- templates
- static-assets
- public-render-tests
- Owners:
- Askama public templates and page-specific static assets; Rust registry remains canonical.
- Invariants:
- Scalar /docs, public routes, auth semantics, registry metadata, locales, and fragment IDs remain stable.
- Non-goals:
- No frontend framework, live homepage search, new marketing page, or backend endpoint.

These records are Method Pack drafts / hints, not authoritative runtime decisions.
