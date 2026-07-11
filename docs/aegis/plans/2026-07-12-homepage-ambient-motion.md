# Homepage Ambient Motion Implementation Plan

**Goal:** Extend the observatory visual system across the homepage with a
CSS-only background field, viewport-entry motion, and hover/focus data-flow
feedback.

**Plan Basis:** User-approved combination of restrained entry motion and
interactive feedback, plus an overall background. Requirement source is
`docs/aegis/specs/2026-07-12-homepage-ambient-motion-brief.md` and the approved
Algorithmic Observatory design.

**Baseline:** Preserve the server-rendered Askama boundary, existing homepage
layout, Worker-only Three.js ownership, registry-derived content, routes,
locales, auth truth, MCP/Scalar asset isolation, and reduced-motion behavior.

**Change Necessity:** A non-code path cannot add the requested visual behavior.
The minimum boundary is existing homepage markup and CSS plus render-contract
tests. Decision: `code-change`.

**Existence Check:** Reuse `templates/home.html`, `static/home.css`, and the
existing `.home-shell`; add no production owner or dependency. Decision:
`reuse-existing`.

**Architecture Integrity:** CSS owns presentation and motion; the Worker keeps
sole scene-rendering ownership; Rust remains assertion wiring only. No
responsibility overlap is introduced. Scope: presentation only.

**Complexity:** `home.css` is already large, so additions must be one
namespaced ambient-motion section with four bounded keyframe families. Markup
changes are attributes/custom properties only. Recommendation: edit in place.

## Task 1: Add The Render Contract

Modify `src/home.rs` with one focused test that requires:

- `data-motion-section` on every post-hero section;
- deterministic `--motion-order` values on repeated rows;
- CSS observatory field selectors and route keyframes;
- `@supports (animation-timeline: view())` enhancement;
- fine-pointer hover gating and `:focus-within` feedback;
- reduced-motion selectors that disable the new motion.

Run the exact test first and confirm RED.

## Task 2: Add Declarative Motion Markers

Modify `templates/home.html`:

- add `data-motion-section` to the six post-hero sections;
- add `style="--motion-order: ..."` to source, capability, endpoint, auth, and
  final-command elements where deterministic staggering is useful;
- do not add visible copy, semantic duplication, or JavaScript hooks.

## Task 3: Implement Ambient And Interactive CSS

Modify `static/home.css`:

- add homepage-only fixed decorative layers to `.home-shell`;
- add static interaction styling first, then view-timeline enhancement;
- add source, capability, endpoint, console, auth, and final-command feedback;
- gate hover-specific effects to fine pointers;
- disable route motion on mobile and all new motion under reduced motion.

## Task 4: Verify

Run:

```bash
rtk cargo test home::tests::homepage_ambient_motion_is_bounded_and_accessible -- --exact
rtk cargo test home::tests --lib
rtk cargo clippy --all-targets --all-features -- -D warnings
rtk cargo fmt --check
rtk node --check static/home-scene.js
rtk git diff --check
```

Then run the application and inspect desktop, mobile, and reduced-motion states
with browser screenshots. Confirm no horizontal overflow and unchanged hero
scene readiness.

## Risks And Retirement

- Risk: too much continuous movement. Control: only the background route has a
  slow loop; entry motion is scroll-bound; interaction motion is user-triggered.
- Risk: hidden content on unsupported browsers. Control: visible base styles,
  animation only inside `@supports`.
- Risk: mobile repaint cost. Control: disable continuously moving background
  routes at `760px` and below.
- Retirement: no old owner or compatibility path is retained; this extends the
  existing background grid and interaction rules in place.

