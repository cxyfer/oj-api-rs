# Observatory Animated Fallback - Intent

## TaskIntentDraft

- Requested outcome: Implement the approved visually faithful SVG and CSS fallback for unavailable Worker WebGL.
- Goal: Preserve observatory visual identity across WebGL-incompatible browsers without main-thread Three.js or a JavaScript animation loop.
- Success evidence:
- Forced fallback screenshots show the core, hubs, network, and moving route pulses at desktop, tablet, and mobile; reduced motion is static; performance and regression checks pass.
- Stop condition: Done when source tests, two-stage review, live fallback/full-scene browser checks, performance attribution, ADR amendment, and workspace verification pass; otherwise report blocked, needs-verification, or scope-exceeded.
- Non-goals:
- No Canvas 2D, main-thread Three.js, new dependency, live problem data, hover/click fallback, or API/database work.
- Scope: Homepage fallback SVG structure, CSS animation, normalized diagnostic codes, render assertions, live browser verification, and Aegis alignment records.
- Change kinds:
- compatibility
- Risk hints:
- Fallback animation could accidentally run behind the successful scene, add main-thread paint pressure, obscure mobile copy, or create a duplicate renderer owner.

## BaselineReadSetHint

- docs/aegis/specs/2026-07-11-observatory-animated-fallback-brief.md
- docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md
- docs/aegis/adr/ADR-0001-homepage-threejs-worker-boundary.md
- docs/aegis/baseline/2026-07-10-initial-baseline.md

## BaselineUsageDraft

- Required baseline refs:
- docs/aegis/specs/2026-07-11-observatory-animated-fallback-brief.md
- docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md
- docs/aegis/adr/ADR-0001-homepage-threejs-worker-boundary.md
- docs/aegis/baseline/2026-07-10-initial-baseline.md
- Acknowledged before plan:
- none
- Cited in plan:
- none
- Missing refs:
- docs/aegis/specs/2026-07-11-observatory-animated-fallback-brief.md
- docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md
- docs/aegis/adr/ADR-0001-homepage-threejs-worker-boundary.md
- docs/aegis/baseline/2026-07-10-initial-baseline.md
- Advisory decision: needs-baseline-readback

## ImpactStatementDraft

- Compatibility boundary: Routes, registry, locales, MCP, Scalar, auth, successful Worker scene, inspector behavior, and reduced-motion semantics remain unchanged.
- Affected layers:
- homepage template and presentation
- scene lifecycle diagnostic contract
- Owners:
- templates/home.html and static/home.css own declarative fallback; static/home-scene.js only activates it and records bounded failure codes.
- Invariants:
- Three.js remains Worker-only; fallback has no JS render loop, interaction, network request, or second data registry.
- Non-goals:
- No Canvas 2D, main-thread Three.js, new dependency, live problem data, hover/click fallback, or API/database work.

These records are Method Pack drafts / hints, not authoritative runtime decisions.
