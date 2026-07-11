# Observatory Animated Fallback Spec Brief

Date: `2026-07-11`
Status: `approved-design-draft`

## 1. Intent

Provide a visually faithful animated observatory background when the homepage
cannot create the Worker-owned Three.js WebGL context. The fallback preserves
the identity of the full scene without adding a main-thread Three.js renderer,
a JavaScript animation loop, database access, API requests, or interaction.

Success evidence:

- Worker/WebGL failure shows a recognizable observatory scene rather than only
  decorative diagonal lines.
- The fallback contains a wireframe core, orbital rings, source hubs, network
  nodes and edges, and controlled route pulses.
- Desktop, tablet, and mobile screenshots remain readable and visually aligned
  with the full Three.js scene.
- `prefers-reduced-motion: reduce` presents the complete fallback composition
  with no animation.
- The fallback adds no scene-attributable main-thread long task above `200ms`.

Stop condition:

- The full Worker scene, animated fallback, and reduced-motion static fallback
  each pass their targeted browser state and performance checks.

Non-goals:

- No hover, click selection, raycasting, inspector updates, or pointer parallax
  in fallback mode.
- No Canvas 2D renderer, second Three.js renderer, new rendering library, live
  problem data, database query, or API request.
- No change to public routes, registry ownership, locales, MCP, Scalar, auth,
  or the successful Worker scene appearance.

## 2. Authority and Alignment

Required baseline refs:

- `docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md`
- `docs/aegis/adr/ADR-0001-homepage-threejs-worker-boundary.md`
- `docs/aegis/baseline/2026-07-10-initial-baseline.md`

Baseline usage:

- The Worker remains the sole Three.js owner.
- `templates/home.html` remains the semantic scene structure owner.
- `static/home.css` remains the homepage presentation and declarative fallback
  owner.
- `static/home-scene.js` remains the Worker lifecycle and fallback activation
  owner; it does not become a renderer.
- Decision: aligned, `scope: both`.

Existence check:

- Proposed surface: declarative inline SVG content inside the existing
  `.scene-fallback` element.
- Existing reuse path: `.scene-fallback`, `.scene-shell`, and the current CSS
  failure class.
- Why the current surface is insufficient: it renders only two faint diagonal
  rules and does not preserve the observatory scene identity.
- Entropy impact: reuse the existing fallback owner; add no JavaScript renderer,
  library, registry, data source, or asset request.
- Decision: `reuse-existing`.

## 3. Chosen Design

### 3.1 Declarative SVG Composition

Replace the empty `.scene-fallback` content with one inline, `aria-hidden` SVG.
The SVG uses a stable `viewBox` and `preserveAspectRatio="xMidYMid slice"` so the
same composition fills desktop, tablet, and mobile hero bounds without resizing
the page layout.

The composition includes:

- one central wireframe polyhedron assembled from SVG paths and lines;
- two thin elliptical core orbits;
- five source hubs using the existing cyan, coral, lime, paper, and blue scene
  palette;
- 30 small deterministic nodes clustered around those hubs;
- low-opacity hub-to-node, hub-to-hub, and core-to-hub edges;
- four route paths with controlled pulse markers;
- no visible labels or semantic content that duplicates the HTML source list.

The fallback remains underneath the canvas at all times. Before Worker readiness
it provides a stable placeholder. When `.is-ready` is present the canvas fades
over it. When `.is-webgl-fallback` is present the SVG becomes the final scene.

### 3.2 Animation

Animation is declarative CSS/SVG only:

- the core group performs a very slow bounded rotation/drift;
- source hubs use low-amplitude opacity and scale breathing;
- four duplicated route strokes use short `stroke-dasharray` segments and
  staggered `stroke-dashoffset` keyframes to appear as moving pulses;
- ambient nodes may vary opacity, but their positions remain fixed;
- no SMIL animation element, blur filter, large shadow, continuously mutated DOM, timer, or JavaScript
  `requestAnimationFrame` is allowed.

Animation is paused by default and runs only under
`.scene-shell.is-webgl-fallback`. This prevents the hidden fallback from
consuming animation work while the Worker scene is active.

### 3.3 Reduced Motion

Under `prefers-reduced-motion: reduce`:

- all fallback animation names are set to `none`;
- the full core, hubs, edges, nodes, and route paths remain visible;
- no opacity pulse, rotation, drift, or route motion continues;
- the existing Worker reduced-motion behavior remains unchanged.

### 3.4 Diagnostic State

Fallback activation writes a bounded diagnostic code to
`canvas.dataset.sceneFailure`. Raw GPU, driver, or browser error strings are not
stored in the DOM.

Allowed codes:

- `worker-unsupported`
- `offscreen-unsupported`
- `worker-load`
- `worker-message`
- `canvas-transfer`
- `invalid-ready`
- `worker-runtime`
- `webgl-context-lost`

Successful Worker readiness removes any stale `sceneFailure` value. Existing
`sceneStatus`, `sceneNonblank`, selection cleanup, and inspector reset behavior
remain unchanged.

## 4. State Contract

```text
initializing
  -> stable SVG placeholder, fallback animation paused

Worker ready + explicit nonblank
  -> sceneStatus=ready
  -> sceneNonblank=true
  -> canvas fades over SVG
  -> fallback animation remains paused

Worker / OffscreenCanvas / WebGL failure
  -> sceneStatus=fallback
  -> sceneNonblank=false
  -> bounded sceneFailure code
  -> canvas stays hidden
  -> SVG fallback animation runs

prefers-reduced-motion
  -> same complete SVG composition
  -> all fallback animation disabled
```

## 5. Responsive and Visual Constraints

- Desktop fallback positions the core in the visual scene region to the right
  of the primary hero copy and preserves all five source hubs where possible.
- Tablet preserves the core, at least four hubs, and visible route pulses.
- Mobile prioritizes the core, two or more hubs, and route motion while the
  existing dark overlay protects copy and metrics.
- No SVG content may overlap page controls incoherently, create horizontal
  overflow, alter hero dimensions, or reduce text contrast below the current
  design.
- Colors, line weights, opacity hierarchy, and motion speed should visually
  match the Three.js scene rather than introduce a separate fallback theme.

## 6. Ownership and Complexity

Files expected to change:

- `templates/home.html`: declarative SVG structure only.
- `static/home.css`: fallback presentation, animations, responsive treatment,
  and reduced-motion rules.
- `static/home-scene.js`: normalized failure-code selection only.
- `src/home.rs`: render/ownership/failure-contract assertions only.
- the existing Algorithmic Observatory design, plan, work evidence, and ADR as
  required for durable alignment.

Complexity budget:

- No new production file or dependency.
- No new runtime owner, render loop, event listener family, or data flow.
- SVG markup must remain deterministic and presentation-only.
- CSS animations must be bounded to the fallback state and use a small number
  of named keyframes.
- Result: within budget if the existing owners are reused.

## 7. Verification

Static checks:

- Render assertions require the fallback SVG, core, hubs, routes, pulse classes,
  bounded failure codes, and reduced-motion selectors.
- Negative assertions reject Canvas 2D, main-thread Three.js imports, fallback
  `requestAnimationFrame`, network fetches, and a second source registry.
- Run Rust tests, clippy, formatting, JavaScript syntax, locale JSON, Aegis
  workspace checks, and `git diff --check`.

Browser checks:

- Verify the successful Worker scene remains ready, nonblank, interactive, and
  visually unchanged at `1440x1000`, `1024x768`, and `390x844`.
- Force Worker load, OffscreenCanvas, and WebGL failures and capture fallback
  screenshots at the same viewports.
- Confirm fallback animation changes visible pixels over time without changing
  layout bounds.
- Confirm reduced-motion fallback screenshots remain pixel-stable over time.
- Confirm failure mode has no page errors, failed required requests, or
  horizontal overflow.
- Confirm MCP and Scalar continue to load no homepage scene assets.

Performance checks:

- Under the existing `390x844` Fast 3G plus 4x CPU profile, preserve LCP at or
  below `2.5s`, CLS at or below `0.1`, and no scene-attributable main-thread
  long task above `200ms` in either successful Worker or fallback mode.
- Confirm fallback mode starts no JavaScript animation loop and does not load
  the Three.js Worker or bundle after an unsupported capability is detected.

## 8. ADR Signal

ADR action candidate: amend
`docs/aegis/adr/ADR-0001-homepage-threejs-worker-boundary.md` after verified
implementation. The core decision remains unchanged; the amendment should
record that the deterministic CSS fallback now contains declarative SVG/CSS
motion while retaining no main-thread Three.js or second JavaScript renderer.
