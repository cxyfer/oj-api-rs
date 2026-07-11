# Observatory Animated Fallback - Evidence

## EvidenceBundleDraft

- Artifact key: task-1-source-and-review
- Type: command and independent review
- Source: focused Rust contract tests, executable Node VM lifecycle test,
  JavaScript syntax, formatting, diff checks, spec reviewer, and code-quality
  reviewer.
- Summary: The declarative SVG/CSS fallback, bounded diagnostic codes, and
  terminal Worker failure state passed both review gates. Review findings fixed
  a queued-message race and removed 30 unnecessary node opacity animations.
- Commit: `e4faef3` (`feat(home): animate compatible scene fallback`).
- Verifier: Codex inline execution plus independent subagent reviews.

## EvidenceBundleDraft

- Artifact key: responsive-worker-and-fallback
- Type: local Chromium and Playwright browser inspection
- Source: `/tmp/observatory-browser-verify.cjs` and
  `/tmp/observatory-browser-results.json` against `http://127.0.0.1:7856`.
- Summary: Desktop `1440x1000`, tablet `1024x768`, and mobile `390x844`
  successful paths reported `sceneStatus=ready`, `sceneNonblank=true`, no page
  errors, no failed required requests, and zero horizontal overflow. The SVG
  opacity was zero after Worker readiness. Desktop click selected `problem-81`;
  offscreen frame count held at `25 -> 25` and resumed to `44`.
- Forced fallback: aborted Worker loading, missing OffscreenCanvas, and an
  injected Worker `webglcontextlost` event produced `worker-load`,
  `offscreen-unsupported`, and `webgl-context-lost` respectively at all three
  viewports. Each fallback pair captured 1050 ms apart had different pixel
  hashes and retained zero overflow with no selection state.
- Reduced motion: desktop, tablet, and mobile fallback screenshot pairs were
  pixel-identical after 1050 ms; all fallback CSS animation was disabled.
- Visual inspection: core, rings, five source hubs, network edges, fixed nodes,
  and route pulses remain legible on desktop/tablet. Mobile prioritizes the core
  and nearby hubs behind the copy and metrics without incoherent overlap.
- Asset isolation: `/docs` and `/docs/mcp` requested no `home-scene.js`, Worker,
  or `three.home.min.js` asset.
- Screenshots: `/tmp/observatory-success-{desktop,tablet,mobile}.png` and
  `/tmp/observatory-{failure}-{viewport}-{motion|reduce}-{a|b}.png`.
- Verifier: Codex inline execution and direct screenshot inspection.

## EvidenceBundleDraft

- Artifact key: fast-3g-performance
- Type: Playwright CDP profile, PerformanceObserver, timeline trace, and
  scene-disabled differential control
- Source: `/tmp/observatory-perf-verify.cjs`,
  `/tmp/observatory-perf-results.json`, `/tmp/observatory-trace-diagnose.cjs`,
  and `/tmp/observatory-success-trace.json`.
- Profile: viewport `390x844`; latency `150 ms`; download `200000 B/s`;
  upload `93750 B/s`; connection type `cellular3g`; CPU throttle `4x`.
- Summary: Successful Worker mode measured LCP `1096 ms` and CLS `0.0329`;
  forced OffscreenCanvas fallback measured LCP `1012 ms` and CLS `0.0329`.
  Unsupported capability loaded neither the Worker nor Three.js vendor asset.
- Long-task attribution: the full success path maximum main-thread task was
  `463 ms`, fallback was `397 ms`, and a control with both scene bridge and SVG
  removed was still `314 ms`. Differential increments were `149 ms` for success
  and `83 ms` for fallback, both below the `200 ms` scene-attributable budget.
  Timeline tracing attributed the greater-than-`200 ms` task to initial
  Layout/style/paint work. The trace contained 17 exact
  `/static/home-scene.js` FunctionCall events, bounded to a maximum of
  `1,909 us` (about `1.9 ms`); they neither formed nor occupied a
  greater-than-`200 ms` long task. The page-level layout task is therefore not
  owned by the scene and remains the existing baseline risk.
- Verifier: Codex inline execution with an explicit control profile.

## EvidenceBundleDraft

- Artifact key: full-regression
- Type: command
- Source: `rtk cargo test`; `rtk cargo clippy --all-targets --all-features -- -D warnings`;
  `rtk cargo fmt --check`; Node syntax checks for the bridge and Worker; locale
  JSON checks; `rtk git diff --check`.
- Summary: All 211 Rust tests passed across six suites. Clippy reported no
  issues; formatting, JavaScript syntax, locale JSON, and diff checks passed.
- Verifier: Codex inline execution.

## EvidenceBundleDraft

- Artifact key: final-browser-performance
- Type: browser-command-trace
- Source: Playwright responsive/failure/reduced-motion harness, Fast 3G plus 4x CPU CDP profile, scene-disabled control, and full Rust/static checks
- Summary: All Worker and fallback states passed at 1440x1000, 1024x768, and 390x844; reduced motion was pixel-stable; LCP was 1096/1012 ms, CLS 0.0329, and scene-attributable long-task increments were 149/83 ms; 211 tests passed.
- Verifier: Codex inline execution plus independent spec and quality reviews
