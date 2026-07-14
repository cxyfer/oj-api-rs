# Observatory Animated Fallback Implementation Plan

**Goal:** Add a visually faithful declarative SVG/CSS fallback for browsers that
cannot create the Worker-owned Three.js WebGL context, while preserving the
successful Worker scene and avoiding a second JavaScript renderer.

**Architecture:** `templates/home.html` owns a deterministic inline SVG;
`static/home.css` owns all fallback presentation and motion;
`static/home-scene.js` only selects a bounded failure code and activates the
existing fallback class. Three.js remains exclusively in
`static/home-scene-worker.js`.

**Tech Stack:** Askama, Rust render-contract tests, inline SVG, CSS keyframes,
module Worker lifecycle JavaScript, Playwright/Chromium browser verification.

**Baseline/Authority Refs:**

- `docs/aegis/specs/2026-07-11-observatory-animated-fallback-brief.md`
- `docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md`
- `docs/aegis/adr/ADR-0001-homepage-threejs-worker-boundary.md`
- `docs/aegis/baseline/2026-07-10-initial-baseline.md`

**Compatibility Boundary:** Preserve routes, registry data, locales, MCP,
Scalar, auth behavior, inspector behavior, successful Worker scene appearance,
reduced-motion semantics, and the `sceneStatus`/`sceneNonblank` datasets.

**Verification:** Focused Rust RED/GREEN tests, full Rust/static checks,
desktop/tablet/mobile screenshots for successful and forced fallback paths,
pixel-change and reduced-motion stability checks, asset-isolation checks, and
the existing Fast 3G plus 4x CPU performance profile.

## Plan Basis

- Requirement source: approved animated fallback Spec Brief and explicit user
  approval on 2026-07-11.
- Requirement Ready Check: `ready`; goal, scenario, visual elements, non-goals,
  failure states, reduced motion, and acceptance evidence are explicit.
- TDD route: light contract-first TDD using the existing Rust render/source
  assertions, followed by live browser evidence.
- Change Necessity: `code-change`. Documentation or browser configuration
  guidance cannot give WebGL-incompatible visitors the approved animated scene.
- Existence Check: `reuse-existing`. The implementation extends the existing
  `.scene-fallback`, `.scene-shell`, `home.css`, and Worker lifecycle bridge.
  It creates no renderer owner, dependency, data source, or production file.

## Architecture Integrity Lens

- Invariant: Three.js parsing and rendering remain Worker-only.
- Canonical owners: SVG structure -> `templates/home.html`; declarative motion
  -> `static/home.css`; state/failure selection -> `static/home-scene.js`.
- Responsibility overlap: none; SVG/CSS is presentation, not a second scene
  simulation or interaction engine.
- Higher-level simplification: reuse the existing fallback class and DOM node.
- Retirement/falsifier: the two-line decorative fallback is superseded by the
  SVG composition; any Canvas 2D, main-thread Three.js, SMIL, fetch, or fallback
  JavaScript animation loop falsifies the design.
- Verdict: aligned, `scope: both`.

## Plan Pressure Test

- Owner/contract/retirement: owners are explicit; the old decorative fallback
  is replaced in place.
- Architecture integrity: no new runtime owner or compatibility renderer.
- Verification scope: static contract, responsive visual evidence, motion
  evidence, reduced motion, asset isolation, and performance are included.
- Task executability: implementation is one cohesive source slice followed by
  controller-owned live verification and documentation closure.
- Pressure result: proceed.

## Complexity Budget

- Artifact class: maintained template, CSS, lifecycle JS, and Rust contract tests.
- Current pressure: `home.css` and `home.rs` are established owners but already
  sizeable; additions must stay fallback-specific and test wiring-only.
- Projected pressure: moderate SVG markup and bounded CSS keyframes; minimal JS.
- Budget result: within-budget if no new file/library/renderer is introduced.
- Recommendation: edit in place and keep SVG class/data names namespaced under
  `fallback-scene`.

## Execution Readiness View

- Intent Lock: visually faithful animated fallback for unavailable Worker WebGL.
- Scope Fence: template SVG, fallback CSS, bounded diagnostic codes, assertions,
  browser/performance evidence, and ADR/work-record alignment only.
- Baseline Lock: Worker remains sole Three.js owner; fallback remains
  non-interactive and data-free.
- Approved Behavior: static placeholder while initializing, canvas on ready,
  animated SVG on failure, fully static SVG under reduced motion.
- Owner/Contract Constraints: no Canvas 2D, main-thread Three.js, SMIL, JS
  fallback loop, API/database access, or new source registry.
- Compatibility Boundary: successful scene and public documentation surfaces
  remain unchanged.
- Retirement Boundary: retire only the existing two-line decorative fallback.
- Task Batches: source contract/implementation; live verification; durable docs.
- Test Obligations: RED/GREEN contract, full regression, responsive screenshots,
  motion/stability proof, failure codes, asset isolation, performance profile.
- Review Gates: spec-compliance review, then code-quality review, then final
  independent review after live evidence.
- Drift/Rewind Rules: any need for a JS renderer, interaction, new dependency,
  or changed successful-scene appearance returns to design.
- Evidence Required Before Completion: commands, screenshots, pixel checks,
  performance attribution, retirement scan, ADR amendment, workspace bundle.
- Advisory Boundary: method-pack execution guidance only; not completion authority.

## Task 1: Implement the Declarative Animated Fallback

**Files:**

- Modify: `src/home.rs`
- Modify: `templates/home.html`
- Modify: `static/home.css`
- Modify: `static/home-scene.js`

**Why:** WebGL-incompatible visitors currently see only two faint diagonal
lines, which does not preserve the approved observatory visual identity.

**Change Necessity:** The minimum sufficient source boundary is the existing
fallback template/CSS plus bounded failure-state wiring. A no-change or docs-only
path cannot render the approved fallback.

**Impact/Compatibility:** Successful Worker rendering, interaction, i18n,
routes, registry data, and non-homepage assets remain unchanged. Fallback remains
non-interactive and contains no live problem data.

**Repair Track:** Replace the insufficient decorative fallback at its canonical
presentation owner. Use the existing failure class; do not add a caller-side
renderer or load Three.js on the main thread.

**Retirement Track:** The current two-gradient `.scene-fallback` background is
superseded in place. No compatibility alias or retained branch is needed.

**Verification commands:**

```bash
rtk cargo test home::tests::homepage_fallback_is_declarative_animated_and_bounded -- --exact
rtk cargo test home::tests::homepage_scene_has_bounded_accessible_contract -- --exact
rtk node --check static/home-scene.js
rtk cargo fmt --check
rtk git diff --check
```

- [ ] **Write test:** Add
  `homepage_fallback_is_declarative_animated_and_bounded` in `src/home.rs`.
  Render `HomeTemplate`, read `static/home.css` and `static/home-scene.js`, then
  assert all of the following:

  ```rust
  assert!(html.contains("data-fallback-scene"));
  assert!(html.contains("viewBox=\"0 0 1440 804\""));
  assert!(html.contains("preserveAspectRatio=\"xMidYMid slice\""));
  assert!(html.contains("data-fallback-core"));
  assert_eq!(html.matches("data-fallback-hub").count(), 5);
  assert_eq!(html.matches("data-fallback-node").count(), 30);
  assert_eq!(html.matches("data-fallback-route").count(), 4);
  assert_eq!(html.matches("data-fallback-pulse").count(), 4);
  assert!(!html.contains("<animate"));

  for marker in [
      ".scene-shell.is-webgl-fallback .fallback-scene",
      "animation-play-state: running",
      "@keyframes fallback-core-drift",
      "@keyframes fallback-hub-breathe",
      "@keyframes fallback-route-pulse",
      "stroke-dasharray",
      "stroke-dashoffset",
      "prefers-reduced-motion: reduce",
      ".fallback-scene",
      "animation: none",
  ] {
      assert!(home_css.contains(marker), "missing fallback CSS marker {marker}");
  }

  for code in [
      "worker-unsupported",
      "offscreen-unsupported",
      "worker-load",
      "worker-message",
      "canvas-transfer",
      "invalid-ready",
      "worker-runtime",
      "webgl-context-lost",
  ] {
      assert!(scene_module.contains(code), "missing failure code {code}");
  }

  for prohibited in ["getContext('2d')", "getContext(\"2d\")", "fetch("] {
      assert!(!scene_module.contains(prohibited));
  }
  assert!(!home_css.contains("url("));
  ```

- [ ] **Verify RED:** Run the exact fallback test. Confirm it fails first on
  missing `data-fallback-scene`; the existing scene contract must remain green.

- [ ] **Minimal code:** Replace the empty `.scene-fallback` with one inline SVG:
  `aria-hidden="true"`, `focusable="false"`, `viewBox="0 0 1440 804"`, and
  `preserveAspectRatio="xMidYMid slice"`. Use these stable groups/markers:

  ```html
  <div class="scene-fallback">
      <svg class="fallback-scene" data-fallback-scene aria-hidden="true"
           focusable="false" viewBox="0 0 1440 804"
           preserveAspectRatio="xMidYMid slice">
          <g class="fallback-scene__network">
              <g class="fallback-scene__edges">...</g>
              <g class="fallback-scene__nodes">30 circles with data-fallback-node</g>
              <g class="fallback-scene__routes">4 base paths with data-fallback-route</g>
              <g class="fallback-scene__pulses">4 duplicate paths with data-fallback-pulse</g>
              <g class="fallback-scene__core" data-fallback-core>...</g>
              <g class="fallback-scene__hubs">5 groups with data-fallback-hub</g>
          </g>
      </svg>
  </div>
  ```

  Use deterministic coordinates that visually follow the current desktop scene:
  core near `(850, 400)`; hubs near `(570, 360)`, `(940, 245)`, `(1145, 455)`,
  `(925, 660)`, and `(590, 625)`. Each hub has a filled center, ring, local
  edges, and six nearby nodes, totaling exactly 30 nodes. Use four curved route
  paths from the core toward distinct hubs and duplicate them for pulse strokes.

  Replace the two-gradient fallback CSS with namespaced SVG rules. Keep all
  animations paused by default. Run only these animations under
  `.scene-shell.is-webgl-fallback`: `fallback-core-drift`,
  `fallback-hub-breathe`, and `fallback-route-pulse`. Use four staggered pulse
  durations between 6s and 10s. Add no filter or large shadow. Under reduced
  motion, set animation to `none` for the network, core, hubs, nodes, and pulses.

  Refactor fallback activation to accept a bounded code:

  ```js
  function activateWorkerFallback(reason = 'worker-runtime') {
      if (workerFailed) return;
      workerFailed = true;
      resetInteractionState();
      if (worker) worker.terminate();
      worker = null;
      if (shell) shell.classList.remove('is-ready');
      activateFallback(shell, canvas, reason);
  }

  function workerFailureCode(reason) {
      return reason === 'webgl context lost'
          ? 'webgl-context-lost'
          : 'worker-runtime';
  }
  ```

  Split Worker and OffscreenCanvas capability branches, wrap Worker creation
  and canvas transfer separately, use closure listeners for `worker-load` and
  `worker-message`, map invalid ready to `invalid-ready`, map Worker failure via
  `workerFailureCode(message.reason)`, and delete stale `sceneFailure` on ready.
  Update `activateFallback` to set `targetCanvas.dataset.sceneFailure = reason`.
  Do not store raw errors or GPU strings in the DOM.

- [ ] **Verify GREEN:** Run every focused command above. Also search the changed
  production files:

  ```bash
  rtk rg -n "CanvasRenderingContext2D|getContext\(['\"]2d|requestAnimationFrame|fetch\(" templates/home.html static/home.css static/home-scene.js
  ```

  Expected: no Canvas 2D, fallback JS loop, or network request; the only
  `requestAnimationFrame` remains in `home-scene-worker.js`, outside this scan.

- [ ] **Commit after both review gates:**

  ```bash
  rtk git add src/home.rs templates/home.html static/home.css static/home-scene.js
  rtk git commit -m "✨ feat(home): animate compatible scene fallback"
  ```

## Task 2: Verify Runtime Compatibility and Close Durable Records

**Files:**

- Modify after evidence: `docs/aegis/adr/ADR-0001-homepage-threejs-worker-boundary.md`
- Modify: `docs/aegis/baseline/2026-07-10-initial-baseline.md`
- Modify: `docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md`
- Modify: `docs/aegis/work/2026-07-11-observatory-animated-fallback/20-checkpoint.md`
- Modify: `docs/aegis/work/2026-07-11-observatory-animated-fallback/90-evidence.md`
- Modify: `docs/aegis/work/2026-07-11-observatory-animated-fallback/99-reflection.md`

**Why:** The fallback is a compatibility behavior and durable runtime boundary;
completion requires live visual/performance proof and synchronized authority
records.

**Change Necessity:** Documentation changes are evidence-dependent closure for
an implemented architecture behavior, not speculative design expansion.

**Impact/Compatibility:** Amend the existing ADR rather than creating a second
decision owner. The core Worker-only Three.js decision remains unchanged.

**Verification commands:**

```bash
rtk cargo test
rtk cargo clippy --all-targets --all-features -- -D warnings
rtk cargo fmt --check
rtk node --check static/home-scene.js
rtk node --check static/home-scene-worker.js
rtk jq empty static/i18n/en.json static/i18n/zh-TW.json static/i18n/zh-CN.json
rtk git diff --check
rtk uv run python /home/usaya/.codex/aegis/scripts/aegis-workspace.py bundle --root /home/usaya/workspace/github/oj-api-rs/.codex/worktrees/style/homepage-refresh --work 2026-07-11-observatory-animated-fallback
rtk uv run python /home/usaya/.codex/aegis/scripts/aegis-workspace.py check --root /home/usaya/workspace/github/oj-api-rs/.codex/worktrees/style/homepage-refresh
```

- [ ] **Write browser harness:** Create a disposable `/tmp` Playwright script
  using the installed `playwright-core` and Chromium. It must test successful
  Worker mode plus forced `worker-load`, `offscreen-unsupported`, and injected
  `webglcontextlost` modes at `1440x1000`, `1024x768`, and `390x844`.

- [ ] **Verify fallback motion and state:** For each forced fallback, assert
  `sceneStatus=fallback`, `sceneNonblank=false`, the expected bounded
  `sceneFailure`, no selection dataset, no overflow, no page error, and no
  required request failure except the deliberately aborted Worker request.
  Capture scene-shell screenshots twice at least 900ms apart and require
  different hashes/pixels in normal motion.

- [ ] **Verify reduced motion and successful path:** Under reduced motion,
  capture the forced fallback twice at least 900ms apart and require stable
  pixels. Re-run the successful Worker scene and its existing interaction,
  ready/nonblank, pause/resume, and asset request assertions. Confirm MCP and
  Scalar request no scene assets.

- [ ] **Verify performance and update records:** Under `390x844`, Fast 3G
  (`150ms`, `200000` B/s down, `93750` B/s up) plus 4x CPU, measure successful
  and forced fallback modes. Require LCP <= `2500ms`, CLS <= `0.1`, no
  scene-attributable main-thread long task > `200ms`, and no fallback
  JavaScript animation loop. Record exact measured values and screenshots in
  the work evidence. Amend ADR-0001 to record declarative SVG/CSS motion,
  update the baseline/design current state, and run full commands above.

- [ ] **Commit after final independent review:**

  ```bash
  rtk git add docs/aegis
  rtk git commit -m "📝 docs(home): record animated fallback evidence"
  ```

## Risks and Rewind Rules

- If SVG/CSS cannot visually match the scene without a JavaScript renderer,
  stop and return to design; do not introduce Canvas 2D implicitly.
- If fallback animation creates a scene-attributable main-thread task above
  `200ms`, first reduce SVG/CSS paint complexity. Do not move Three.js back to
  the main thread.
- If mobile copy becomes less readable, adjust fallback opacity/composition
  inside `home.css`; do not change hero content or layout ownership.
- If normal Worker screenshots or interaction differ, rewind the fallback
  changes until the successful path is unchanged.
- Raw GPU/driver strings must not enter DOM datasets or persisted evidence.
