# ADR-0001 - Keep the homepage Three.js runtime in a module Worker

Status: `recorded-from-work`
Date: `2026-07-11`

## Source Evidence

- Task 8 work record, browser verification, CDP trace, and scene-disabled control profiles.
## Context

A tree-shaken main-thread Three.js runtime still produced scene initialization work in the main runtime boundary and could not meet the approved scene-attributable long-task budget.

## Decision

The main homepage module owns DOM, localization, events, observers, datasets, and Worker lifecycle; a dedicated module Worker exclusively owns Three.js, OffscreenCanvas rendering, raycasting, animation, adaptive DPR, and pixel sampling. Unsupported or failed Worker/WebGL capability selects the deterministic CSS fallback with no main-thread renderer.

## Alternatives Considered

- Keep the renderer on the main thread and further reduce nodes or bundle bytes.
- Defer the same main-thread import until after initial content.
## Consequences

- Three parsing and initialization leave the main thread; interaction crosses a structured message boundary and requires explicit lifecycle, reduced-motion, and failure handling.
- Browsers without Worker/OffscreenCanvas/WebGL receive complete HTML plus CSS fallback rather than an interactive scene.
## Compatibility Boundary

Public routes, registry data, localization, inspector datasets, Scalar, MCP, auth behavior, and accessible HTML remain unchanged; Three.js remains homepage-only.

## Retirement Impact

The main-thread renderer, three.module.min.js, three.core.min.js, and the temporary bundling entry are retired; no compatibility renderer is retained.

## Baseline Sync

- Needed: needed
- Target: docs/aegis/baseline/2026-07-10-initial-baseline.md
- Action: update baseline
- Reason: The baseline must name the new main/Worker canonical owners and the sole CSS fallback boundary.

## Evidence References

- docs/aegis/work/2026-07-10-algorithmic-observatory/90-evidence.md
## Boundary

This ADR is an advisory Aegis Method Pack record. It does not grant completion authority or replace project-authoritative architecture sources.
