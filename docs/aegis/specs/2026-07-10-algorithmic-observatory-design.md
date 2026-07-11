# Algorithmic Observatory Homepage and MCP Reference Design

Date: `2026-07-10`
Status: `approved design`

## 1. Summary

Redesign `/` and `/docs/mcp` as a cohesive world-class product and developer
experience for oj-api-rs. The homepage positions the service as Problem
Intelligence Infrastructure and uses a guided Three.js Spatial Intelligence
Map as its primary visual. The MCP reference becomes a quieter, denser work
surface. Scalar `/docs` remains unchanged.

## 2. Authority and Baseline References

- User-approved direction in the 2026-07-10 design conversation.
- `CONTEXT.md` for canonical product language.
- `openspec/specs/homepage-api-guide/spec.md` for the current public docs contract.
- `openspec/changes/archive/2026-04-27-show-api-usage-on-homepage/design.md`
  for the superseded Bento rationale and preserved documentation boundaries.
- `src/main.rs` for Scalar `/docs` ownership.
- `src/home.rs`, `templates/home.html`, `templates/docs_mcp.html`, and
  `static/home.css` for the current implementation boundary.

## 3. TaskIntentDraft

- Outcome: a distinctive, responsive landing page and MCP reference that make
  AI, spatial computing, and algorithmic problem intelligence feel like one product.
- Goal: improve brand clarity and product comprehension while keeping the API
  integration path immediate.
- Success evidence: approved layouts render correctly at desktop and mobile
  sizes, the WebGL scene is nonblank and responsive, existing routes and
  localized content remain correct, and performance budgets hold.
- Stop condition: `/`, `/docs/mcp`, and the preserved `/docs` boundary pass
  functional, visual, accessibility, and performance verification.
- Non-goals: live homepage search, Scalar redesign, admin redesign, new backend
  behavior, new marketing pages, or API/MCP contract changes.
- Primary risks: WebGL cost, visual dominance over copy, mixed CSS ownership,
  mobile overlap, localization expansion, and stale Bento rules.

## 4. Baseline Usage

BaselineUsageDraft:
- Required baseline refs: `CONTEXT.md`, homepage OpenSpec, `src/main.rs`, `src/home.rs`.
- Delivered context refs: current homepage, MCP template, CSS, i18n, archived design.
- Acknowledged before plan refs: all required refs above.
- Cited in design refs: all required refs above.
- Missing refs: none.
- Decision: continue.

Requirement Ready Check:
- Requirement source refs: approved conversation and homepage OpenSpec.
- Goals and scope refs: Sections 1, 3, and 5 of this design.
- User / scenario refs: developers, competitive programmers, and AI agents.
- Requirement item refs: Sections 6 through 13.
- Acceptance / verification criteria refs: Section 16.
- Open blocker questions: none.
- Decision: ready.

## 5. Scope and Impact

ImpactStatementDraft:
- Affected layers: public Askama templates, public CSS, homepage-only browser JS,
  localization JSON, homepage/MCP render tests, and public docs requirements.
- Owners: existing public page templates and Rust documentation registry;
  a main-thread scene bridge owns DOM integration and a dedicated Worker owns
  WebGL behavior.
- Invariants: route paths, Scalar ownership, endpoint registry, MCP tool names,
  auth semantics, localization set, and public access remain stable.
- Compatibility: `/docs/api` continues to redirect permanently to `/docs`.
- Non-goals: API, persistence, authentication, crawler, admin, and MCP transport changes.

## 6. Product Positioning and Copy Hierarchy

The homepage uses this hierarchy:

1. Brand H1: `OJ API`.
2. Canonical category: `Problem Intelligence Infrastructure`.
3. Primary statement: `Every judge. One problem space.`
4. Supporting copy: one shared intelligence layer for developers, competitors,
   and AI agents to resolve, retrieve, relate, and connect algorithmic problems.
5. Primary command: `Explore the API` linking to `/docs`.
6. Secondary command: `MCP for Agents` linking to `/docs/mcp`.

All visible guide copy must exist in English, Traditional Chinese, and
Simplified Chinese. Technical identifiers and code remain untranslated.

## 7. Homepage Information Architecture

### 7.1 Hero

- Occupies approximately `88svh` and leaves a visible hint of the next section.
- Uses a full-bleed Three.js canvas behind unframed text and navigation.
- Displays total indexed problems, supported source count, version, and current
  auth state without turning the metrics into decorative cards.
- Keeps both primary commands visible without covering scene focal points.

### 7.2 Product Narrative

The post-hero sequence is:

1. `One problem space`: normalized records across Online Judge Sources.
2. `Resolve / Retrieve / Relate / Connect`: four product capabilities in an
   editorial, unframed layout.
3. Three registry-backed featured endpoints presented as horizontal feature rows.
4. A segmented REST/MCP showcase with copyable examples.
5. A compact dual-state auth matrix covering `/`, `/health`, `/api/v1/*`,
   `/status`, and `/mcp`.
6. A full-width final command leading to `/docs`.

The homepage remains curated and does not duplicate the complete Scalar or MCP reference.

## 8. Spatial Intelligence Map

### 8.1 Meaning

The scene is a meaningful product visualization, not a generic particle field.
Five source anchors organize problem nodes into algorithmic neighborhoods.
Semantic edges represent relatedness, while controlled pulses represent
resolution, retrieval, and MCP/API access paths.

### 8.2 Interaction

- The camera follows a slow authored path with bounded pointer parallax.
- Hovering or clicking a node reveals an HTML label with source, problem title,
  algorithm family, and illustrative similarity score.
- Interaction never enables unrestricted orbit, pan, or zoom.
- Scene interaction is optional; all product meaning remains available in HTML.

### 8.3 Data Boundary

- Supported source names are rendered from the existing Rust registry into the DOM.
- The scene module reads this DOM data rather than maintaining a second source registry.
- Illustrative problem nodes and scores must be clearly presented as a visual
  model, not live search results.

### 8.4 Worker Runtime Boundary

- Three.js module parsing, scene construction, raycasting, rendering, and pixel
  sampling run in a dedicated module Worker using `OffscreenCanvas`.
- The main-thread scene module owns DOM discovery, source-name extraction,
  localization, pointer/control filtering, resize observation, visibility
  observation, and worker lifecycle only.
- The worker receives registry-derived source names and normalized pointer,
  click, resize, visibility, and reduced-motion messages. It returns ready,
  nonblank, frame-count, hover, selection, and failure messages containing only
  illustrative scene metadata.
- Hover and click selection remain observable through the existing HTML
  inspector and canvas datasets; the worker does not own localized copy or DOM.
- The main thread must not import Three.js or retain a second renderer path.
- If module Worker, `OffscreenCanvas`, canvas transfer, or WebGL initialization
  is unavailable, the page activates the existing deterministic CSS fallback
  and retains complete HTML content. It does not load Three.js on the main thread.

## 9. Visual System

- Graphite: `#07090b`.
- Warm white: `#f3f1ea`.
- Electric cyan: `#53e0e8`.
- Signal coral: `#ff6a55`.
- Acid lime: `#c7f45b`.
- Typography: `Familjen Grotesk` for display and body, `IBM Plex Mono` for
  code, metrics, and identifiers; remove Playfair Display.
- Shape language: sharp rules, compact data labels, algorithm notation,
  asymmetrical grids, and restrained transparency.
- Prohibited motifs: purple gradients, decorative glow blobs, glass-card walls,
  oversized pill containers, and purely atmospheric particles.

## 10. MCP Reference Design

- The page is a developer work surface, not a second marketing landing page.
- Use a compact hero with version and current auth status.
- Use sticky section navigation on desktop and horizontally scrollable section
  navigation on mobile.
- Present transport as a concise flow and MCP tools as unframed reference rows.
- Preserve exact tool names, inputs, REST mapping, output style, usage notes,
  fragment IDs, and keyboard-accessible `<details>` behavior.
- Add explicit copy controls to connection and JSON-RPC examples.
- Maintain readable line lengths and avoid nested cards.

## 11. Responsive and Accessibility Behavior

- Desktop, tablet, and mobile layouts use stable grid tracks and explicit bounds.
- No font size scales directly with viewport width.
- Mobile reduces scene nodes, edges, pixel ratio, and pointer interaction.
- The canvas is `aria-hidden`; equivalent source and capability information is in the DOM.
- All commands and disclosure controls are keyboard accessible with visible focus.
- Text and controls meet WCAG AA contrast.
- `prefers-reduced-motion: reduce` renders one stable scene frame and disables
  nonessential transitions.
- Worker, OffscreenCanvas, or WebGL failure leaves the content fully readable
  over a deterministic CSS background.

## 12. Performance Budget

- Three.js is pinned, self-hosted, and loaded only by the homepage.
- Three.js is loaded only inside the scene Worker; the main thread must not parse
  or evaluate the Three.js runtime.
- Pixel ratio cap: `1.5` desktop and `1.25` mobile.
- Scene budget: approximately 110 nodes desktop and 48 nodes mobile, with a
  bounded edge count and no post-processing pipeline.
- Target frame rate: 60 fps on capable desktop hardware and at least 30 fps on
  representative mobile hardware.
- The render loop pauses when the hero leaves the viewport or the document is hidden.
- Quality reduces when sustained frame time exceeds budget.
- HTML and critical CSS render before the scene becomes interactive.
- Under the recorded `390x844` Fast 3G plus 4x CPU profile, LCP remains at or
  below `2.5s`, CLS remains at or below `0.1`, and no main-thread long task above
  `200ms` is attributable to scene bootstrap, interaction bridging, or fallback.
- No raster hero image is required; the domain-specific scene is the primary visual asset.

## 13. Ownership and File Boundaries

- `templates/home.html`: homepage semantic structure and scene data bridge.
- `templates/docs_mcp.html`: MCP information architecture and local copy controls.
- `templates/docs_base.html`: shared public shell and page-specific asset blocks.
- `static/site.css`: shared public shell and design tokens.
- `static/home.css`: homepage-only composition and scene overlays.
- `static/mcp.css`: MCP-only reference styling.
- `static/home-scene.js`: main-thread DOM, i18n, event, observer, dataset, and
  Worker lifecycle bridge; it does not import Three.js.
- `static/home-scene-worker.js`: canonical Three.js scene, rendering, raycast,
  animation, adaptive-quality, and pixel-sampling owner.
- `static/vendor/three.home.min.js`: pinned, self-contained, tree-shaken Three.js
  revision 180 runtime loaded only by the Worker, with license notice.
- Localization JSON: visible translatable copy only.
- `src/home.rs`: wiring-only template data and render assertions.

Existence Check:
- Proposed new surface: homepage scene bridge, scene Worker, vendor runtime,
  shared/site split, and MCP stylesheet.
- Existing owner / reuse candidate: mixed `static/home.css` and template-local scripts.
- Why existing surface is insufficient: WebGL has a distinct lifecycle, while
  the current stylesheet already mixes homepage and reference-page responsibilities.
- Creation proof: the Worker is required to keep the pinned Three.js runtime off
  the main thread; page-specific loading prevents it from reaching Scalar or MCP
  pages and keeps CSS owners cohesive.
- Entropy / retirement impact: old Bento selectors and superseded shared rules
  are removed in the same change; no parallel theme remains.
- Decision: add-with-proof.

## 14. Complexity and Architecture Integrity

Complexity Budget:
- Artifact class: source and maintained presentation artifacts.
- Target files / artifacts: `src/home.rs`, public templates, public CSS, scene
  bridge/Worker JS, and i18n.
- Current pressure: `src/home.rs` is 959 lines and `static/home.css` is 619 lines
  with mixed responsibilities.
- Projected post-change pressure: adding in place would push the stylesheet near
  or beyond strong pressure while adding a new responsibility to a large Rust owner.
- Budget result: at-risk without the proposed split; within-budget with it.
- Planned governance: keep Rust changes wiring-only, split presentation owners,
  and isolate WebGL lifecycle in the Worker while the main thread remains a
  narrow bridge.

Plan-Time Complexity Check:
- Better file boundary: shared shell, homepage, MCP, main-thread scene bridge,
  and Worker rendering each have one named owner.
- Recommendation: extract shared CSS, repurpose `home.css`, and add separate MCP,
  scene bridge, and Worker rendering owners.

Architecture Integrity Lens:
- Invariant: one canonical owner for route metadata, one main-thread bridge, and
  one Worker owner for WebGL behavior.
- Canonical owner / contract: Rust registry for endpoint/tool data;
  `home-scene.js` for DOM bridging; `home-scene-worker.js` for rendering.
- Responsibility overlap: prohibited between scene data and the Rust registry.
- Higher-level simplification: DOM-rendered registry values bridge Rust and JS
  without a new JSON endpoint.
- Retirement / falsifier: any remaining active Bento owner or duplicated source
  list fails the design.
- Verdict: coherent with the proposed file split.

## 15. Compatibility and Retirement

Anti-Entropy Declaration:
- Deletion Class: code-retirement.
- Old Path/Object: Bento homepage layout selectors, superseded mixed CSS rules,
  and main-thread Three.js renderer/import paths.
- New Canonical Owner: Algorithmic Observatory homepage, separated public
  styles, main-thread scene bridge, and Worker renderer.
- Expected Preserved Behavior: public routes, localized content, auth guidance,
  featured endpoints, examples, fragment links, and responsive access.
- Expected Retired Behavior: Bento first-screen composition, glass-card wall
  styling, and main-thread Three.js parsing/rendering.
- External Boundary Touched: no.
- Source-of-Truth Data Risk: none.
- User Confirmation Required: no.

Retirement Decision:
- Path: delete-first.
- Why: the old presentation is internal and has no published compatibility contract.
- Non-edits: Scalar, OpenAPI, API routes, MCP transport, persistence, and admin UI.

Baseline Role Alignment:
- Product / Requirement Baseline: approved direction supersedes only the old
  Bento layout and card-specific presentation language.
- Architecture / Runtime Boundary Baseline: existing route and registry owners remain correct.
- Result: aligned after the homepage OpenSpec update included with this design.
- scope: both.
- Next action: produce an implementation plan after user review.

## 16. Acceptance and Verification

### Functional

- `/` and `/docs/mcp` return HTTP 200 without admin authentication.
- `/docs` remains Scalar and `/docs/api` remains a permanent redirect.
- All three locales contain the same public copy key structure.
- Featured endpoint and MCP tool content still comes from the Rust registry.
- Deep links open the correct MCP detail disclosure.
- Copy controls copy the exact visible example text.

### Visual and Interaction

- The homepage canvas is nonblank and correctly framed at desktop and mobile sizes.
- The first viewport shows the brand, value proposition, primary command, and a
  visible hint of following content without overlap.
- Node hover/click, pointer parallax, authored camera movement, pause/resume,
  reduced-motion, and WebGL fallback behave as specified.
- Worker messages preserve hover/click datasets and localized inspector output;
  interactive hero controls do not trigger background selection.
- MCP reference rows, navigation, details, and code blocks remain readable at all target sizes.

### Performance and Quality

- Run focused Rust render/router tests and the full Rust test suite.
- Run formatting and clippy checks.
- Use Playwright screenshots at representative desktop, tablet, and mobile viewports.
- Perform canvas-pixel checks to prove the WebGL scene is not blank.
- Inspect overflow, console errors, network loading, reduced motion, and scene pause behavior.
- Confirm Three.js is requested by the Worker only, never evaluated on the main
  thread, and absent from MCP and Scalar pages.
- Verify Worker/OffscreenCanvas initialization failure selects the CSS fallback
  without attempting a main-thread Three.js compatibility path.
- Measure page weight and runtime performance against the budgets in Section 12.

## 17. ADR Signal

- Signal: yes, because the design introduces a pinned Three.js dependency,
  separates public presentation ownership, and moves the renderer across a
  main-thread/Worker boundary.
- Alternatives: main-thread Three.js, mobile CSS-only fallback, Canvas 2D, and
  Three.js with Worker-owned OffscreenCanvas.
- Chosen direction: Worker-owned Three.js with guided exploration,
  homepage-only loading, and CSS fallback when the worker runtime is unavailable.
- Expected completion question: whether the implemented dependency and owner
  boundaries merit an ADR backfill after verification.

## 18. Open Questions

None. Additional pages are deferred unless a later requirement demonstrates a
specific user journey that cannot be served by `/`, `/docs`, or `/docs/mcp`.
