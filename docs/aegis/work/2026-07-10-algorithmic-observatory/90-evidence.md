# Algorithmic Observatory Implementation - Evidence

The following evidence bundles record the implementation and verification results.

## EvidenceBundleDraft

- Artifact key: baseline-tests
- Type: command
- Source: rtk cargo fmt --check; rtk cargo test; aegis-workspace.py check
- Summary: Clean baseline: formatting passed, 204 tests passed across 6 suites, workspace check passed.
- Verifier: Codex inline execution

## EvidenceBundleDraft

- Artifact key: task-1-assets
- Type: command
- Source: rtk cargo test home::tests; rtk cargo test; rtk cargo fmt --check; rtk git diff --check
- Summary: Asset ownership slice passed: 15 home tests, 206 full tests, formatting and diff checks; CSS split into site 161 lines, home 243 lines, MCP 204 lines.
- Verifier: Codex inline execution

## EvidenceBundleDraft

- Artifact key: tasks-2-to-5
- Type: command
- Source: homepage and MCP focused tests; rtk cargo test; node --check home-scene.js and mcp.js; jq empty locales
- Summary: Semantic homepage, locale parity, Three.js contract, and MCP work surface implemented; 207 Rust tests pass, JS syntax and locale JSON checks pass.
- Verifier: Codex inline execution

## EvidenceBundleDraft

- Artifact key: task-7-runtime-and-vendor-repair
- Type: command and live browser inspection
- Source: `rtk cargo fmt --check`; `rtk cargo test`; `rtk cargo clippy --all-targets --all-features -- -D warnings`; `rtk node --check static/home-scene.js`; `rtk node --check static/mcp.js`; `rtk jq empty` locale bundles; local Chromium and Playwright checks at 1440x1000, 1024x768, and 390x844.
- Summary: Added the missing pinned `three.core.min.js` revision 180 dependency and a regression test. All 209 Rust tests, clippy, formatting, JavaScript syntax, locale JSON, and diff checks pass. Homepage canvas is ready and nonblank at all tested viewports; hover inspector, offscreen and visibility-change pause/resume, reduced motion, and forced WebGL fallback pass. MCP deep-link/copy and Scalar redirect pass without loading Three.js outside the homepage.
- Verifier: Codex inline execution
- Locale evidence: after installing `fonts-noto-cjk` in WSL, a fresh Chromium session rendered the zh-TW MCP mobile page with `Noto Sans CJK TC`, no horizontal overflow, and no console errors.
- Residual gap: trusted throttled Core Web Vitals were not measured in the available local browser tooling.

## EvidenceBundleDraft

- Artifact key: task-8-worker-runtime
- Type: command, live browser inspection, CDP performance trace, and differential control run
- Source: focused scene and retirement tests; local Chromium through agent-browser and Playwright core; Fast 3G plus 4x CPU CDP profile; CDP trace; scene-bridge-blocked control profile; full Rust and static checks.
- Summary: The homepage now transfers its canvas once to a module Worker. Desktop 1440x1000, tablet 1024x768, and mobile 390x844 report ready/nonblank with no overflow, console errors, or failed requests. Problem nodes and controlled pulses hover and pin; blank clicks clear; CTA clicks preserve selection; a stationary pointer refreshes as a pulse moves; offscreen and document-hidden states pause and resume; zh-TW idle and pinned inspector copy localizes; reduced motion holds frame count at 2 through 120 pointer events while retaining inspector interaction. Unsupported OffscreenCanvas, aborted Worker loading, and an injected runtime `webglcontextlost` event all select the deterministic CSS fallback and clear interaction state. MCP and Scalar request no scene assets.
- Performance: Fast 3G plus 4x CPU measured LCP 1004 ms, CLS 0.033, no failed requests, and a ready/nonblank scene. PerformanceObserver reported a 396 ms maximum main-thread long task, so attribution was investigated rather than treated as a pass. CDP tracing identified a 284.3 ms main-thread Layout event and a 633.3 ms `home-scene-worker.js` FunctionCall on the DedicatedWorker thread. Three control runs with the scene bridge blocked still measured 377, 377, and 465 ms maximum main-thread long tasks, versus 429, 507, and 398 ms normally. This excludes the Worker scene runtime as the owner of the page-level main-thread long task and satisfies the approved no scene-attributable main-thread task above 200 ms boundary.
- Retirement: After live replacement evidence, deleted `static/vendor/three.module.min.js`, `static/vendor/three.core.min.js`, and `static/vendor/three.home.entry.js`. The exact retirement test passes and `static/` plus `templates/` contain no lingering retired-file references.
- Regression: `cargo test` reports 209 passed; clippy reports no issues; `cargo fmt --check`, JS `node --check`, locale `jq empty`, and `git diff --check` all exit 0.
- Verifier: Codex inline execution with independent spec and code-quality subagent reviews.
- Residual risk: The full-buffer one-time `readPixels` operation remains Worker-only but can cost hundreds of milliseconds under throttling. The non-scene page Layout long task is outside Task 8 and remains a potential separate performance follow-up.

## EvidenceBundleDraft

- Artifact key: task-8-worker-runtime
- Type: browser-and-command
- Source: Chromium, Playwright CDP trace/control profiles, cargo and static checks
- Summary: Worker scene passes interaction, fallback, responsive, locale, asset-isolation and performance-attribution gates; old vendor chain retired; 209 tests and all static checks pass.
- Verifier: Codex inline execution plus independent spec and code-quality reviews
