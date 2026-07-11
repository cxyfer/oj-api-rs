# Algorithmic Observatory Implementation Plan

**Goal:** Replace the Bento homepage with the approved Algorithmic Observatory
experience, add a guided Three.js Spatial Intelligence Map, and redesign the
MCP reference as a focused developer work surface while preserving Scalar and
all public runtime contracts.

**Architecture:** Keep Axum + Askama as the rendering boundary and the existing
Rust registry as the only source for endpoint, source, and MCP tool metadata.
Split the mixed public presentation layer into shared shell, homepage, and MCP
owners. Keep the main-thread scene module as a DOM/i18n/event bridge and load a
pinned self-hosted Three.js ESM runtime only inside a homepage module Worker
using `OffscreenCanvas`; the worker reads source labels supplied by the bridge
and owns no product registry or DOM copy.

**Tech Stack:** Rust 2021, Axum 0.8, Askama 0.12, static HTML/CSS/ES modules,
Three.js `0.180.0`, module Worker, `OffscreenCanvas`, project i18n JSON,
Chromium, and Playwright browser checks.

**Baseline/Authority Refs:**

- `docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md`
- `docs/aegis/baseline/2026-07-10-initial-baseline.md`
- `CONTEXT.md`
- `openspec/specs/homepage-api-guide/spec.md`
- `src/main.rs`
- `src/home.rs`

**Compatibility Boundary:** Keep `/`, `/docs/mcp`, `/docs`, `/docs/api`,
`/openapi.json`, API routes, MCP transport, auth semantics, registry values,
fragment IDs, and the `en`/`zh-TW`/`zh-CN` locale set stable. `/docs` remains
Scalar-owned. Do not add a frontend framework, production bundler, API endpoint,
fallback theme, duplicated source/tool registry, or main-thread Three.js
compatibility renderer.

**Verification:** Focused Rust template/router tests, full `cargo test`,
`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
locale JSON validation, static asset checks, a live local server, Chromium
screenshots at desktop/tablet/mobile viewports, canvas-pixel checks, reduced
motion and WebGL-fallback checks, MCP deep-link/copy interactions, console and
overflow inspection, Worker-only Three.js network/runtime checks, Fast 3G plus
4x CPU main-thread metrics, and confirmation that `/docs` still renders Scalar.

## Aegis Visibility

Planning is necessary because this work adds a WebGL lifecycle owner, retires a
mixed stylesheet, touches three localized presentation surfaces, and requires
visual evidence beyond ordinary Rust tests. Task 8 additionally moves the
renderer across a main-thread/Worker boundary after throttled evidence proved
the main-thread runtime exceeded the approved long-task budget.

## Plan Basis

- Fact: `/docs` is assembled by `Scalar::with_url("/docs", openapi.clone())` in
  `src/main.rs` and is outside the Askama public docs shell.
- Fact: `src/home.rs` supplies the homepage and MCP reference registry and is
  already 959 lines.
- Fact: `static/home.css` currently mixes homepage and reference-page styles.
- Fact: Chromium and Node/npm are available in the environment.
- Fact: the `390x844` Fast 3G plus 4x CPU profile measured LCP `864ms`, CLS
  `0.033`, and a `1019ms` scene-attributable main-thread long task after the
  tree-shaken subset bundle; node-count reduction cannot remove module parsing.
- Assumption: the server can start with the repository's local development
  configuration; if not, use `config.toml.example` to create a temporary config
  under `/tmp` and set `CONFIG_PATH` without editing the repository.
- Unknown until execution: whether dependency download needs network approval;
  the vendoring task isolates that approval.

BaselineUsageDraft:
- Required baseline refs: approved Aegis design, initial baseline, current OpenSpec, `src/main.rs`, `src/home.rs`.
- Delivered context refs: current templates, CSS, i18n module, route tests, and repository history.
- Acknowledged before plan refs: all required refs.
- Cited in plan refs: all required refs.
- Missing refs: none.
- Decision: continue.

Requirement Ready Check:
- Requirement source refs: approved Aegis design and updated homepage OpenSpec.
- Goals and scope refs: plan header and approved design Sections 1, 3, and 5.
- User / scenario refs: developers, competitive programmers, and AI agents.
- Requirement item refs: Tasks 1 through 8.
- Acceptance / verification criteria refs: Tasks 7-8 and approved design Section 16.
- Open blocker questions: none.
- Decision: ready.

## Change Necessity

- User-visible need: a new product position, a meaningful 3D hero, and a denser
  MCP reference cannot be produced by documentation or configuration alone.
- No-change / non-code option: retain the current Bento HTML and CSS.
- Why code change is necessary: the approved layout, WebGL lifecycle, adaptive
  motion, copy controls, responsive MCP structure, and main-thread long-task
  budget require new browser and template behavior. Deferring or further
  minifying the renderer does not remove its main-thread parse/evaluate task.
- Minimum change boundary: public Askama templates, page-specific CSS and JS,
  locale JSON, static vendor runtime, wiring-only render assertions in
  `src/home.rs`, and browser-based verification evidence.
- Decision: code-change.

## Files

**Create:**

- `static/site.css`: public shell, tokens, typography, code, focus, and shared controls.
- `static/mcp.css`: MCP reference layout and responsive behavior.
- `static/home-scene.js`: main-thread scene DOM/i18n/event/Worker bridge.
- `static/home-scene-worker.js`: Worker-owned Three.js scene lifecycle,
  rendering, raycasting, animation, and pixel sampling.
- `static/mcp.js`: MCP deep-link and copy-control behavior.
- `static/vendor/three.home.min.js`: pinned, self-contained, tree-shaken
  Three.js revision 180 runtime loaded only by the Worker.
- `static/vendor/THREE-LICENSE.txt`: upstream MIT license and pinned version note.

**Modify:**

- `templates/docs_base.html`: shared CSS and page-specific asset blocks.
- `templates/home.html`: Algorithmic Observatory structure and scene data bridge.
- `templates/docs_mcp.html`: MCP work-surface structure and copy controls.
- `static/home.css`: homepage-only layout after retiring Bento and reference rules.
- `static/i18n/en.json`: approved public copy.
- `static/i18n/zh-TW.json`: Traditional Chinese copy.
- `static/i18n/zh-CN.json`: Simplified Chinese copy.
- `src/home.rs`: wiring-only template assertions and locale/static asset checks.

**Do Not Modify:**

- `src/main.rs` unless a failing preservation test proves current Scalar routing
  differs from the approved baseline.
- `src/api/**`, `src/mcp/**`, database, auth, crawler, admin, and OpenAPI schema owners.

## Compatibility and Ripple Triage

Ripple Signal Triage:
- Upstream sources: `DocsRegistry`, runtime auth flag, package version, locale files.
- Direct consumers: homepage template, MCP template, browser scene/copy modules.
- Downstream boundaries: `/`, `/docs/mcp`, `/docs/api`, shared public shell, static assets.
- Unaffected boundary requiring proof: Scalar `/docs` and `/openapi.json`.
- Verification expansion: router status checks plus live browser inspection of all three docs routes.

## Existence and Architecture Checks

Existence Check:
- Proposed new surface: shared/site CSS, MCP CSS/JS, homepage scene JS, vendored Three.js.
- Existing owner / reuse candidate: `static/home.css` and inline template scripts.
- Why existing surface is insufficient: the current stylesheet mixes unrelated
  page responsibilities and inline scripts cannot cleanly own WebGL or copy lifecycles.
- Creation proof: each new file maps to one page or shared responsibility and
  is loaded only where needed.
- Entropy / retirement impact: the old Bento selectors, generic reference-card
  wall, and duplicate inline MCP script are removed in the same workstream.
- Decision: add-with-proof.

Architecture Integrity Lens:
- Invariant: the Rust registry remains canonical for source, endpoint, and MCP tool metadata.
- Canonical owner / contract: Askama renders semantic data; browser modules only enhance it.
- Responsibility overlap: no hard-coded supported-source list in `home-scene.js`.
- Higher-level simplification: use DOM `data-source` values as the bridge rather
  than adding a new endpoint or JSON registry.
- Retirement / falsifier: lingering active `.bento-*` selectors, Three.js loaded
  on `/docs/mcp`, or duplicate source arrays fail the plan.
- Verdict: proceed with separated owners.

Plan Pressure Test:
- Owner / contract / retirement: explicit in file map and Tasks 1, 2, and 6.
- Architecture integrity / higher-level path: registry-to-DOM bridge avoids a new backend owner.
- Verification scope: Rust, locale, browser, canvas, interaction, performance, and Scalar preservation.
- Task executability: each task has an isolated RED/GREEN assertion and commit.
- Pressure result: proceed.

Complexity Budget:
- Artifact class: Source Complexity and maintained presentation artifacts.
- Target files / artifacts: `src/home.rs`, `static/home.css`, public templates, new JS/CSS owners.
- Current pressure: `src/home.rs` is 959 lines; `static/home.css` is 619 lines and mixed-purpose.
- Projected post-change pressure: over-budget if WebGL/MCP behavior is added in
  place; within-budget if style and browser owners are separated.
- Budget result: within-budget with the planned split.
- Planned governance: wiring-only Rust edits, delete-first Bento retirement,
  page-specific JS/CSS, and no new backend surface.

Plan-Time Complexity Check:
- Target files: `src/home.rs`, `static/home.css`, `templates/home.html`, `templates/docs_mcp.html`.
- Existing size / shape signals: large Rust registry/test owner and mixed CSS responsibilities.
- Owner fit: templates own semantics; CSS owns composition; JS owns enhancement.
- Add-in-place risk: duplicate themes, hidden page coupling, and hard-to-test lifecycle code.
- Better file boundary: shared shell, homepage, MCP, and scene/copy modules.
- Recommendation: extract helper assets and keep Rust edits local and wiring-only.

## Task 1: Lock the New Asset and Contract Boundaries

**Files:**

- Modify: `src/home.rs`
- Modify: `templates/docs_base.html`
- Create: `static/site.css`
- Create: `static/mcp.css`
- Create: `static/home-scene.js`
- Create: `static/mcp.js`

**Why:** Establish page-specific owners and failing contract tests before any
visual rewrite, so later tasks cannot accidentally load Three.js on MCP/Scalar
or preserve the old mixed stylesheet.

**Change Necessity:** A new WebGL owner and separated MCP presentation cannot be
expressed through the existing single stylesheet and inline script. The minimum
boundary is shared asset blocks plus empty named owners and render assertions.

**Impact/Compatibility:** No visible design change yet. `/docs` remains outside
the shared Askama template. The temporary empty assets must return successfully
once served.

**Verification:**

```bash
rtk cargo test home::tests::public_pages_load_only_their_owned_assets -- --exact
rtk cargo test home::tests::scalar_compatibility_path_remains_a_redirect -- --exact
```

Expected before implementation: the first test fails because page-specific
assets do not exist in the templates. Expected after implementation: both pass.

- [ ] **Write test:** Add `public_pages_load_only_their_owned_assets` in
  `src/home.rs`. Render `HomeTemplate` and `McpDocsTemplate`; assert homepage
  HTML contains `/static/site.css`, `/static/home.css`, and
  `/static/home-scene.js`, MCP HTML contains `/static/site.css`,
  `/static/mcp.css`, and `/static/mcp.js`, and MCP HTML does not contain
  `three.module.min.js` or `home-scene.js`. Add a router assertion named
  `scalar_compatibility_path_remains_a_redirect` for `/docs/api` -> `/docs`.
- [ ] **Verify RED:** Run the two commands above. Confirm the asset ownership
  test fails on missing new asset references while the redirect test preserves
  the current baseline.
- [ ] **Minimal code:** Add `{% block page_styles %}` and
  `{% block page_scripts %}` to `templates/docs_base.html`, switch the shared
  stylesheet to `/static/site.css`, add the page-specific links/scripts in the
  child templates, and split the current stylesheet without visible regression:
  shared shell/tokens/code rules into `site.css`, current homepage/Bento rules
  into `home.css`, and current reference-page rules into `mcp.css`. Create the
  JS owners with strict-mode module comments only; the existing inline MCP hash
  behavior remains temporarily until Task 5 moves it.
- [ ] **Verify GREEN:** Re-run the focused tests, then run
  `rtk cargo test home::tests::docs_base_rewrites_example_origin_at_runtime`.
- [ ] **Commit:** `rtk git add src/home.rs templates/docs_base.html templates/home.html templates/docs_mcp.html static/site.css static/home.css static/mcp.css static/home-scene.js static/mcp.js && rtk git commit -m "♻️ refactor(home): split public page assets"`

## Task 2: Build the Semantic Algorithmic Observatory Homepage

**Files:**

- Modify: `templates/home.html`
- Modify: `static/site.css`
- Modify: `static/home.css`
- Modify: `src/home.rs`

**Why:** Deliver the approved content hierarchy and a resilient HTML-first
homepage before adding WebGL, ensuring the page communicates the product even
when JavaScript or WebGL is unavailable.

**Change Necessity:** The current Bento DOM cannot support the approved
full-bleed hero, unframed narrative bands, segmented integration showcase, and
next-section viewport cue. The minimum boundary is homepage markup plus shared
and homepage CSS.

**Impact/Compatibility:** Preserve registry-backed featured endpoints, total
problem/source/version/auth values, auth rows, links, and canonical example.
Replace presentation classes only; no route or API behavior changes.

**Repair Track:**

- Root cause: the current homepage is optimized as a documentation Bento grid,
  not a product landing page.
- Canonical owner: `templates/home.html` for semantic order and
  `static/home.css` for composition.
- Minimal stable repair: replace the old structure rather than layer new styles
  on `.bento-*` selectors.
- Compatibility: retain required data, links, and route text.
- Verification: render assertions plus responsive browser checks in Task 7.

**Retirement Track:**

- Old owner: `.bento-*`, `.panel`, `.bento-link`, and homepage glass-card composition.
- Active status: superseded by approved design.
- Deletion trigger: new semantic homepage render test passes.

**Verification:**

```bash
rtk cargo test home::tests::renders_algorithmic_observatory_homepage -- --exact
rtk cargo test home::tests::homepage_auth_matrix_documents_expected_routes -- --exact
rtk rg -n '\.bento-|bento-' templates/home.html static/home.css
```

Expected before implementation: the new render test fails. Expected after
implementation: both tests pass and `rg` returns no active Bento matches.

- [ ] **Write test:** Replace the old headline assertions with
  `renders_algorithmic_observatory_homepage`. Assert `OJ API`, `Every judge. One
  problem space.`, `Problem Intelligence Infrastructure`, `scene-shell`,
  `scene-source-data`, `data-source="leetcode"`, `/docs`, `/docs/mcp`, all three
  featured registry paths, all five auth rows, and one copyable REST example.
  Assert the HTML does not contain `bento-grid` or `Playfair Display`.
- [ ] **Verify RED:** Run the focused test and confirm it fails on the new hero
  copy/structure.
- [ ] **Minimal code:** Rewrite `templates/home.html` with: full-bleed hero;
  semantic source-data list generated from `docs.supported_sources`; metrics;
  primary/secondary commands; `One problem space`; four capability items;
  registry-backed endpoint rows; accessible REST/MCP segmented showcase using
  a native radio group and two CSS-selected panels; compact auth table; and final `/docs` command. Move
  global tokens/shell/code/focus styles into `site.css`; rewrite `home.css` as
  homepage-only styles. Use stable grids, max widths, and `88svh` hero bounds.
- [ ] **Verify GREEN:** Run the focused tests and `rtk rg` command. Confirm the
  test passes and there are no active Bento selectors.
- [ ] **Commit:** `rtk git add templates/home.html static/site.css static/home.css src/home.rs && rtk git commit -m "✨ feat(home): build observatory landing page"`

## Task 3: Add Localized Product Copy and Integration Controls

**Files:**

- Modify: `static/i18n/en.json`
- Modify: `static/i18n/zh-TW.json`
- Modify: `static/i18n/zh-CN.json`
- Modify: `templates/home.html`
- Modify: `src/home.rs`

**Why:** Keep the redesigned product narrative and controls complete in all
supported locales rather than leaving the new hero and section copy English-only.

**Change Necessity:** Existing locale keys describe the superseded Bento layout
and do not cover the approved positioning or segmented showcase. The minimum
boundary is the `home` locale subtree and its structural test.

**Impact/Compatibility:** Keep `common` and admin keys unchanged. Preserve
technical identifiers and code examples verbatim. Continue using the existing
language persistence behavior.

**Verification:**

```bash
rtk cargo test home::tests::docs_locale_bundles_cover_public_pages -- --exact
rtk jq empty static/i18n/en.json static/i18n/zh-TW.json static/i18n/zh-CN.json
```

Expected before implementation: locale coverage fails for new keys. Expected
after implementation: Rust coverage and JSON parsing pass.

- [ ] **Write test:** Expand `docs_locale_bundles_cover_public_pages` with JSON
  pointers for `/home/hero/category`, `/home/hero/statement`,
  `/home/hero/primary_cta`, `/home/space/title`, each capability title,
  `/home/integrations/rest`, `/home/integrations/mcp`, and `/home/final_cta/title`.
  Add a helper that compares the recursive key paths under `/home` and
  `/docs_mcp` for all three locale values.
- [ ] **Verify RED:** Run the focused test and confirm missing keys are reported.
- [ ] **Minimal code:** Replace superseded homepage strings with approved English,
  Traditional Chinese, and Simplified Chinese copy. Add i18n attributes to all
  visible nontechnical homepage text and control labels. Keep the REST/MCP
  segmented showcase native and CSS-driven through its radio group; do not add
  an inline script or another homepage UI module.
- [ ] **Verify GREEN:** Run the focused test and `jq empty` command. Cycle the
  locale manually during Task 7 visual verification.
- [ ] **Commit:** `rtk git add static/i18n/en.json static/i18n/zh-TW.json static/i18n/zh-CN.json templates/home.html src/home.rs && rtk git commit -m "🌐 feat(home): localize observatory experience"`

## Task 4: Vendor Three.js and Implement the Guided Spatial Intelligence Map

Task 4 records the original main-thread implementation. Task 8 supersedes its
runtime owner and vendor loading path after throttled evidence exposed the
main-thread long-task defect; its product behavior and scene semantics remain required.

**Files:**

- Create: `static/vendor/three.module.min.js`
- Create: `static/vendor/THREE-LICENSE.txt`
- Modify: `static/home-scene.js`
- Modify: `templates/home.html`
- Modify: `static/home.css`
- Modify: `src/home.rs`

**Why:** Add the distinctive spatial-computing experience while containing its
runtime, dependency, accessibility, and performance cost to the homepage.

**Change Necessity:** CSS cannot provide meaningful 3D semantic neighborhoods,
edges, raycast interaction, and an authored camera path. The minimum boundary
is a pinned self-hosted ESM runtime plus one scene lifecycle module.

**Impact/Compatibility:** The canvas remains decorative and `aria-hidden`.
Source names come from DOM registry values. Failure leaves HTML and the CSS
fallback visible. No remote runtime request occurs in production.

**Verification:**

```bash
curl --fail --location --show-error https://unpkg.com/three@0.180.0/build/three.module.min.js -o /tmp/three.module.min.js
curl --fail --location --show-error https://unpkg.com/three@0.180.0/LICENSE -o /tmp/THREE-LICENSE.txt
rtk cargo test home::tests::homepage_scene_has_bounded_accessible_contract -- --exact
rtk wc -c static/vendor/three.module.min.js static/home-scene.js
```

Expected before implementation: the contract test fails and vendor files are
absent. Expected after implementation: test passes; runtime is local and the
scene module remains a bounded custom owner.

- [ ] **Write test:** Add `homepage_scene_has_bounded_accessible_contract`.
  Assert the rendered canvas host is `aria-hidden="true"`; the scene script is
  `type="module"`; the template contains `data-source` values from every
  supported source; an accessible `.scene-inspector` exists outside the canvas;
  and the module source contains `IntersectionObserver`, `visibilitychange`,
  `prefers-reduced-motion`, pixel-ratio caps `1.5` and `1.25`, and a WebGL
  fallback class. Assert no Three.js script appears in MCP HTML.
- [ ] **Verify RED:** Run the focused test and confirm the lifecycle contract is missing.
- [ ] **Minimal code:** Download the exact vendor files to `/tmp`, inspect their
  headers/license, then add them through `apply_patch` or an approved mechanical
  copy. Implement deterministic seeded clusters, five registry-derived source
  anchors, approximately 110/48 desktop/mobile nodes, bounded semantic edges,
  raycasting, HTML inspector updates, slow authored camera motion, pointer
  parallax, DPR caps, adaptive quality, resize handling, `IntersectionObserver`,
  page visibility pause, one-frame reduced motion, and failure fallback.
  Import only `/static/vendor/three.module.min.js`. Do not use post-processing,
  texture downloads, orbit controls, or a hard-coded supported-source array.
- [ ] **Verify GREEN:** Run the focused test and file-size command. Start the
  live server and complete the canvas checks in Task 7 before calling this slice done.
- [ ] **Commit:** `rtk git add static/vendor/three.module.min.js static/vendor/THREE-LICENSE.txt static/home-scene.js templates/home.html static/home.css src/home.rs && rtk git commit -m "✨ feat(home): add spatial intelligence map"`

## Task 5: Redesign the MCP Reference Work Surface

**Files:**

- Modify: `templates/docs_mcp.html`
- Modify: `static/mcp.css`
- Modify: `static/mcp.js`
- Modify: `static/site.css`
- Modify: `static/i18n/en.json`
- Modify: `static/i18n/zh-TW.json`
- Modify: `static/i18n/zh-CN.json`
- Modify: `src/home.rs`

**Why:** Make `/docs/mcp` faster to scan and use repeatedly while preserving all
transport and tool detail contracts.

**Change Necessity:** The current card grid and inline script do not support the
approved sticky navigation, unframed reference rows, transport flow, or copy
feedback. The minimum boundary is the MCP template plus its page-specific CSS/JS.

**Impact/Compatibility:** Preserve all transport/tool registry content, exact
fragment IDs, collapsed `<details>`, auth status, connection example, request
example, and route behavior.

**Verification:**

```bash
rtk cargo test home::tests::renders_mcp_work_surface_with_tools_and_examples -- --exact
rtk cargo test home::tests::docs_detail_panels_render_collapsed_by_default -- --exact
```

Expected before implementation: the new work-surface test fails. Expected after
implementation: both tests pass.

- [ ] **Write test:** Rename/replace the existing MCP render test with
  `renders_mcp_work_surface_with_tools_and_examples`. Assert compact hero,
  `mcp-layout`, sticky `mcp-toc`, `transport-flow`, two transport rows, five
  tool rows, all existing fragment IDs, exactly seven reference `<details>`,
  two `data-copy-target` buttons, `aria-live` feedback, `mcp.js`, and no
  `panel reference-card` or `home-scene.js`.
- [ ] **Verify RED:** Run the focused test and confirm it fails on the new structure.
- [ ] **Minimal code:** Rewrite `templates/docs_mcp.html` as an unframed work
  surface with compact hero/status, sticky section rail, semantic transport
  flow, registry-backed reference rows, preserved `<details>`, and two copy
  buttons. Move `openByHash` into `static/mcp.js`; add hash expansion,
  `navigator.clipboard` with a selection fallback, localized copied feedback,
  and no inline event handlers. Implement desktop sticky and mobile horizontal
  TOC behavior in `mcp.css`.
- [ ] **Verify GREEN:** Run both focused tests and locale parity test from Task 3.
- [ ] **Commit:** `rtk git add templates/docs_mcp.html static/mcp.css static/mcp.js static/site.css static/i18n/en.json static/i18n/zh-TW.json static/i18n/zh-CN.json src/home.rs && rtk git commit -m "✨ feat(mcp): redesign reference work surface"`

## Task 6: Retire Superseded Presentation Paths and Harden Static Contracts

**Files:**

- Modify: `static/site.css`
- Modify: `static/home.css`
- Modify: `static/mcp.css`
- Modify: `templates/docs_base.html`
- Modify: `templates/home.html`
- Modify: `templates/docs_mcp.html`
- Modify: `src/home.rs`

**Why:** Complete the delete-first transition so the repository has one active
public visual system and no hidden fallback to the Bento/card-wall design.

**Change Necessity:** Leaving stale selectors or inline owners would preserve
two themes and undermine the file split. The minimum boundary is removal plus
negative tests.

**Impact/Compatibility:** Remove only internal presentation paths. Preserve
shared code blocks, language switching, runtime origin rewriting, fragment
links, and route semantics.

**Repair Track:**

- Root cause: historical styles accumulated multiple page responsibilities.
- Canonical owner: `site.css`, `home.css`, and `mcp.css` by responsibility.
- Minimal stable repair: move shared rules once and delete obsolete selectors.
- Compatibility: public behavior is asserted before deletion.
- Verification: lingering-reference and negative selector checks.

**Retirement Track:**

- Old owner/fallback: `.bento-*`, generic `.panel` composition, MCP inline
  `openByHash`, and Playfair font request.
- Active status: no longer needed.
- Keep reason: none; there is no external presentation contract.
- Deletion trigger: Tasks 2 and 5 tests pass.

**Verification:**

```bash
rtk cargo test home::tests::public_templates_do_not_reference_retired_presentation -- --exact
rtk rg -n 'bento-|Playfair Display|class="panel reference-card"|function openByHash' templates/docs_base.html templates/home.html templates/docs_mcp.html static/home.css static/site.css static/mcp.css static/mcp.js
rtk git diff --check
```

Expected before implementation: the negative test or `rg` finds retired
references. Expected after implementation: test passes, `rg` has no matches,
and diff check is clean.

- [ ] **Write test:** Add `public_templates_do_not_reference_retired_presentation`.
  Read the three templates and three stylesheets with `include_str!`; reject
  `bento-`, `Playfair Display`, `panel reference-card`, and inline
  `function openByHash`. Assert the shared shell still references
  `/static/i18n.js` and includes page asset blocks.
- [ ] **Verify RED:** Run the focused test and confirm at least one stale owner is found.
- [ ] **Minimal code:** Remove all superseded selectors, font requests, generic
  card-wall rules, and inline MCP lifecycle code. Consolidate repeated tokens
  and accessible control styles into `site.css` without creating nested cards.
- [ ] **Verify GREEN:** Run the focused test, `rg`, and diff check.
- [ ] **Commit:** `rtk git add templates/docs_base.html templates/home.html templates/docs_mcp.html static/site.css static/home.css static/mcp.css static/mcp.js src/home.rs && rtk git commit -m "♻️ refactor(home): retire bento presentation"`

## Task 7: Verify Runtime, Visual Quality, Accessibility, and Performance

**Files:**

- Modify if evidence finds defects: only files already owned by Tasks 1-6.
- Modify: this plan only to record completed verification checkboxes when useful.
- Modify if required: `docs/aegis/specs/2026-07-10-algorithmic-observatory-design.md`
  only to record verified implementation drift, never to relax acceptance.

**Why:** WebGL correctness, layout quality, deep-link behavior, and Scalar
preservation cannot be proven by template string assertions alone.

**Change Necessity:** This task is verification-first. Code changes are allowed
only for concrete defects discovered within the approved owner boundaries.

**Impact/Compatibility:** Verification must cover both protected and public auth
render states where relevant. Do not change production contracts to make tests pass.

**Verification commands:**

```bash
rtk cargo fmt --check
rtk cargo clippy --all-targets --all-features -- -D warnings
rtk cargo test
CONFIG_PATH=config.toml rtk cargo run
```

If `config.toml` is absent, create `/tmp/oj-api-homepage-config.toml` from
`config.toml.example`, set a temporary database path under `/tmp`, and start with:

```bash
CONFIG_PATH=/tmp/oj-api-homepage-config.toml rtk cargo run
```

Use Chromium through the browser automation capability against the printed
local URL, normally `http://127.0.0.1:7856`.

- [ ] **Write test:** Before browser work, add any missing automated assertion
  revealed by the final spec-to-test mapping. Do not add snapshot-only tests for
  behavior already covered by semantic assertions.
- [ ] **Verify RED:** For each discovered defect, capture the failing assertion,
  console message, screenshot, canvas pixel result, or interaction reproduction
  before editing.
- [ ] **Minimal code:** Fix only evidenced defects in the existing owner. Common
  allowed corrections are responsive bounds, scene framing, contrast, reduced
  motion, copy feedback, and missing semantic attributes.
- [ ] **Verify GREEN:** Complete all checks below:
  - Desktop `/` at `1440x1000`, tablet at `1024x768`, and mobile at `390x844`.
  - Desktop and mobile `/docs/mcp`, including long Chinese copy.
  - `/docs` visually identifies as Scalar and `/docs/api` redirects to it.
  - Canvas screenshot contains non-background cyan/coral/lime pixels inside the
    hero scene; the source anchors remain within the visible frame.
  - Hero brand, statement, commands, metrics, and next-section hint do not overlap.
  - Pointer movement changes the guided scene subtly; hover/click updates the
    HTML inspector without shifting layout.
  - Scrolling the hero fully offscreen stops scene frame-count growth; returning
    resumes it. Hiding the document follows the same rule.
  - Reduced-motion context renders one stable frame and disables continuous motion.
  - Forced WebGL initialization failure shows the CSS fallback and complete HTML.
  - MCP `#tool-get-problem` opens the correct `<details>` and maintains focusable navigation.
  - Both copy buttons copy exact visible code and announce localized success.
  - No horizontal overflow, clipped text, console errors, failed static requests,
    nested cards, or control overlap at target sizes.
  - Three.js loads only on `/`; `/docs/mcp` and `/docs` do not request it.
  - DPR caps and scene node budgets match the approved design.
  - Record total Three.js and custom scene byte sizes; keep the vendored
    minified runtime below 750 KB raw, custom scene code below 32 KB raw, and
    verify there is no post-processing or external texture request.
  - Measure Core Web Vitals under the available mobile throttling profile:
    target LCP at or below 2.5 seconds, CLS at or below 0.1, and no long task
    above 200 ms attributable to scene initialization. If local tooling cannot
    produce a trustworthy throttled metric, report that gap instead of claiming it passed.
- [ ] **Commit:** After all commands and browser evidence pass, commit only
  concrete fixes and any checked execution evidence in this plan:
  `rtk git add docs/aegis/plans/2026-07-10-algorithmic-observatory-implementation.md templates static src/home.rs && rtk git commit -m "🧪 test(home): verify observatory experience"`.

## Task 8: Move the Three.js Runtime Off the Main Thread

**Files:**

- Create: `static/home-scene-worker.js`
- Create: `static/vendor/three.home.min.js`
- Modify: `static/home-scene.js`
- Modify: `src/home.rs`
- Delete after replacement verification: `static/vendor/three.module.min.js`
- Delete after replacement verification: `static/vendor/three.core.min.js`
- Delete after bundle verification: `static/vendor/three.home.entry.js`

**Why:** The approved mobile Fast 3G plus 4x CPU profile measured a `1019ms`
main-thread long task after the self-contained Three.js subset loaded. Detailed
instrumentation showed renderer creation, scene build, first render, and pixel
sampling still execute with module evaluation on the main thread. Reducing node
counts or deferring the same import cannot fix this bug class.

**Change Necessity:** A documentation-only change would leave the performance
contract false. Further minification reduced bytes by `25.4%` but did not bring
the long task below `200ms`. The minimum sufficient boundary is a narrow
main-thread DOM bridge plus one module Worker that owns Three.js and the
transferred `OffscreenCanvas`.

**Impact/Compatibility:** Routes, Rust registry data, locales, visible HTML,
Scalar, MCP, auth, scene budgets, inspector copy, and dataset names remain
stable. Unsupported Worker, OffscreenCanvas, canvas transfer, or WebGL selects
the existing CSS fallback; no main-thread Three.js compatibility path is kept.

**Repair Track:**

- Root cause: the main thread parses/evaluates the Three.js WebGLRenderer and
  executes scene initialization in the same runtime boundary.
- Canonical owners: `static/home-scene.js` for DOM/i18n/events/observers and
  `static/home-scene-worker.js` for Three.js/rendering/raycast/animation.
- Minimal stable repair: transfer the existing canvas once, move the current
  scene owner into the Worker, and exchange normalized structured messages.
- Compatibility: preserve inspector metadata, click pin/blank clear, hover
  precedence, CTA guard, frame-count datasets, adaptive motion, and CSS fallback.
- Verification: focused contracts, full checks, browser interaction evidence,
  Worker-only network evidence, and the identical throttled profile.

**Retirement Track:**

- Old owner: main-thread Three.js import/render lifecycle in `home-scene.js`.
- Old vendor path: `three.module.min.js -> three.core.min.js`.
- Active status: superseded only after Worker runtime and performance evidence pass.
- Deletion trigger: Worker canvas is nonblank; desktop/mobile/reduced-motion and
  fallback checks pass; the throttled profile has no scene-attributable
  main-thread long task above `200ms`.
- Retained compatibility path: none. Unsupported worker capability uses CSS,
  not the retired main-thread renderer.

**Worker Message Contract:**

- Main to worker `init`: transferred canvas, registry-derived `sourceNames`,
  width, height, DPR cap inputs, mobile flag, and reduced-motion flag.
- Main to worker `resize`: width, height, and current DPR cap inputs.
- Main to worker `pointer`: normalized x/y plus inside state.
- Main to worker `select`: normalized x/y or clear selection.
- Main to worker `rendering`: visible and document-hidden state.
- Worker to main `ready`: nonblank status and initial frame count.
- Worker to main `frame`: bounded frame-count updates used by pause/resume tests.
- Worker to main `hover` and `selection`: raw illustrative metadata only; main
  thread formats localized inspector text and owns canvas datasets.
- Worker to main `failure`: initialization/runtime failure reason; main activates
  the deterministic CSS fallback and terminates the Worker.

**Verification commands:**

```bash
rtk cargo test home::tests::homepage_scene_has_bounded_accessible_contract -- --exact
rtk cargo test home::tests::homepage_scene_vendor_is_self_contained_and_retires_full_distribution -- --exact
rtk node --check static/home-scene.js
rtk node --check static/home-scene-worker.js
rtk node --check static/vendor/three.home.min.js
rtk cargo fmt --check
rtk cargo clippy --all-targets --all-features -- -D warnings
rtk cargo test
rtk jq empty static/i18n/en.json static/i18n/zh-TW.json static/i18n/zh-CN.json
rtk git diff --check
rtk wc -c static/vendor/three.home.min.js static/home-scene.js static/home-scene-worker.js
rtk rg -n 'three\.module\.min\.js|three\.core\.min\.js|three\.home\.entry\.js|import .*three\.home' static src templates
```

- [ ] **Write test:** Expand
  `homepage_scene_has_bounded_accessible_contract` so `home-scene.js` must use
  `transferControlToOffscreen`, construct a module Worker for
  `/static/home-scene-worker.js`, handle `ready`, `frame`, `hover`, `selection`,
  and `failure`, and contain neither a Three.js import nor `new THREE`.
  Assert the Worker imports `/static/vendor/three.home.min.js`, owns the existing
  DPR/node/observer-independent render markers, and consumes `init`, `resize`,
  `pointer`, `select`, and `rendering` messages. Keep the vendor retirement test
  requiring a self-contained bundle and absence of both full-distribution files.
- [ ] **Verify RED:** Run both exact tests. Confirm the bridge/Worker ownership
  test fails because `home-scene.js` still imports Three.js and the retirement
  test fails only because old vendor files remain.
- [ ] **Minimal code:** Move all Three.js construction, raycasting, animation,
  adaptive quality, reduced-motion static rendering, and pixel sampling into
  `home-scene-worker.js`. Keep DOM lookup, source extraction, i18n formatting,
  interactive-control filtering, normalized pointer/click dispatch, datasets,
  `ResizeObserver`, `IntersectionObserver`, visibility forwarding, Worker error
  handling, and CSS fallback in `home-scene.js`. Transfer the canvas once with
  `transferControlToOffscreen()`. On unsupported capability or worker failure,
  terminate the Worker and call the sole CSS fallback without importing Three.js.
  Do not add a second source registry, main-thread renderer, remote asset,
  post-processing path, or production bundler. After live replacement evidence,
  delete `three.module.min.js`, `three.core.min.js`, and the temporary entry.
- [ ] **Verify GREEN:** Run every command above. Start the local server in the
  WSL-approved network context. Re-run desktop `1440x1000`, tablet `1024x768`,
  mobile `390x844`, zh-TW active inspector, problem/pulse pin, blank clear, CTA
  guard, stationary-pointer pulse refresh, offscreen/document pause, reduced
  motion, forced worker/WebGL failure, MCP/Scalar asset isolation, and screenshots.
  Under `390x844`, CDP Fast 3G (`150ms`, `200000` B/s down, `93750` B/s up) plus
  4x CPU, require LCP <= `2500ms`, CLS <= `0.1`, no failed requests, a nonblank
  ready canvas, and no scene-attributable main-thread long task > `200ms`.
- [ ] **Commit:**
  `rtk git add docs/aegis static/home-scene.js static/home-scene-worker.js static/vendor/three.home.min.js static/vendor/THREE-LICENSE.txt static/i18n src/home.rs && rtk git add -u static/vendor && rtk git commit -m "⚡️ perf(home): move scene rendering to worker"`.

## Retirement Decision

- Path: delete-first.
- Retired behavior: Bento layout, glass-card homepage wall, generic MCP card
  grid, Playfair font, inline MCP behavior, main-thread Three.js rendering, and
  the full-distribution Three.js module chain.
- Preserved behavior: public route access, registry-driven content, auth truth,
  locales, fragment IDs, examples, origin rewriting, and Scalar API docs.
- Compat exception: none.
- Persistent-state risk: none.

Verification Plan:
- Main-path check: live `/` and `/docs/mcp` complete their approved workflows.
- Lingering-reference check: negative Rust test and `rg` find no old owner.
- Negative check: Three.js loads only in the homepage Worker; it does not load
  on the homepage main thread, MCP, or Scalar; Worker/WebGL failure does not remove content.
- Boundary check: Scalar, redirect, API, MCP transport, auth, and registry tests remain green.

## Risks and Rollback Surface

- Vendor download may fail or be blocked: retry with explicit network approval;
  do not replace the runtime with an unpinned CDN import.
- WebGL may be blank on software-rendered Chromium: use the CSS fallback only
  as the unsupported-WebGL path, not as evidence that the scene works.
- Module Worker or OffscreenCanvas may be unavailable: activate the existing CSS
  fallback and complete HTML; do not restore the retired main-thread renderer.
- Worker message drift may break inspector or lifecycle behavior: keep the
  message kinds explicit in focused tests and verify every interaction live.
- Locale expansion may overflow compact controls: allow wrapping and stable
  height rather than shrinking type with viewport width.
- A scene performance defect may require lowering node/edge budgets; it must not
  introduce post-processing, remote assets, or hidden animation on MCP/Scalar.
- Rollback is limited to presentation assets/templates and wiring-only tests;
  there is no schema, persistence, or public API migration.

## ADR and Baseline Sync Signal

- Preserve the approved ADR signal for the pinned Three.js dependency,
  presentation owner split, and main-thread/Worker rendering boundary.
- At completion, evaluate whether verified implementation matches the design
  closely enough to backfill an ADR. Do not create an accepted ADR from this plan alone.
- Update the baseline only if verification proves a durable owner boundary that
  differs from the initial snapshot; do not update it to excuse drift.

## Execution Readiness View

- Intent Lock: implement the approved Algorithmic Observatory homepage and MCP
  work surface, not a broader docs or backend redesign.
- Scope Fence: `/`, `/docs/mcp`, public shell assets, locales, and render tests;
  Scalar and runtime contracts stay out of scope.
- Baseline Lock: approved Aegis design, homepage OpenSpec, current router and registry owners.
- Approved Behavior: guided Three.js hero, curated narrative, API primary CTA,
  dense MCP reference, adaptive motion, Worker-owned rendering, and complete
  HTML/CSS fallback.
- Owner / Contract Constraints: Rust registry canonical; browser modules enhance
  only; main thread owns DOM/i18n/observers; Worker owns Three.js; Three.js is
  homepage-Worker-only; `src/home.rs` remains wiring-only.
- Compatibility Boundary: routes, auth, locales, fragments, metadata, examples,
  and Scalar remain stable.
- Retirement Boundary: remove old Bento/card-wall/inline owners, main-thread
  renderer, full Three.js distribution, and temporary bundle entry; no renderer
  compatibility fallback.
- Task Batches: asset boundaries; semantic homepage; localization; scene; MCP;
  retirement; full verification; Worker performance migration.
- Test Obligations: RED/GREEN render assertions per task plus live visual,
  canvas, interaction, accessibility, performance, and Scalar checks.
- Review Gates: inspect after semantic homepage, scene, MCP, Worker ownership,
  and final performance evidence.
- Drift / Rewind Rules: if a task requires a backend endpoint, duplicate registry,
  frontend framework, remote production asset, or Scalar edit, stop and return
  to the approved design instead of expanding scope.
- Evidence Required Before Completion: command results, screenshots, nonblank
  worker canvas proof, interaction checks, Worker-only Three.js network evidence,
  throttled main-thread metrics, fallback evidence, and clean git diff.
- Advisory Boundary: method-pack execution guidance only; not GateDecision,
  PolicySnapshot, or completion authority.
