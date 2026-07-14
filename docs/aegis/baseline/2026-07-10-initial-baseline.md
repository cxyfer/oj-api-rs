# oj-api-rs Initial Baseline

Date: `2026-07-10`
Status: `initial dual-baseline snapshot`

## 1. Purpose

This snapshot records the product and runtime boundaries that shape the public
homepage and MCP reference redesign. Later alignment checks should use the
OpenSpec files as requirement authority and the Rust routes, Askama templates,
and static assets as implementation evidence.

## 2. Workspace Structure

- `src/` owns the Axum application, public routes, API behavior, MCP transport,
  runtime data, and Askama render context.
- `templates/` owns server-rendered public and admin HTML.
- `static/` owns public CSS, localization JSON, and browser-side behavior.
- `openspec/specs/` owns current product requirements.
- `openspec/changes/archive/` preserves prior design and implementation history.

## 3. Current Authority Surfaces

- `AGENTS.md` and `/home/usaya/.codex/RTK.md`: agent and shell conventions.
- `README.md`: product overview, supported platforms, and integration surface.
- `CONTEXT.md`: canonical domain language.
- `openspec/specs/homepage-api-guide/spec.md`: public homepage and docs requirements.
- `src/main.rs`: canonical `/docs` Scalar ownership and router assembly.
- `src/home.rs`: canonical homepage and MCP reference render data.

## 4. Product / Requirement Baseline

### 4.1 Current Truth

- The product is Problem Intelligence Infrastructure for both people and AI systems.
- `/` is a public, curated product landing page rather than an exhaustive reference.
- `/docs` is the public Scalar API reference.
- `/docs/mcp` is the dedicated public MCP transport and tool reference.
- Visible public guide copy supports `en`, `zh-TW`, and `zh-CN`.
- The approved redesign direction is Algorithmic Observatory with a guided
  Three.js Spatial Intelligence Map.

### 4.2 Non-negotiables

1. `/docs` remains owned by Scalar and retains its current route.
2. The homepage keeps a visible primary path to `/docs` and a secondary path to `/docs/mcp`.
3. Auth guidance must remain truthful for enabled and disabled token-auth deployments.
4. Existing public routes, endpoint metadata, and MCP tool names remain unchanged.
5. Motion must degrade safely for mobile, reduced-motion, and unavailable WebGL.

### 4.3 Product Non-goals

- No new live search product flow on the homepage.
- No rewrite of Scalar or the OpenAPI document.
- No new standalone marketing pages in this workstream.
- No admin UI redesign.

## 5. Architecture / Runtime Boundary Baseline

### 5.1 Current Truth

- Axum and Askama remain the server-rendering boundary.
- `src/main.rs` owns Scalar `/docs`; the homepage must not duplicate that owner.
- `src/home.rs` owns public page render context and the Rust documentation registry.
- Three.js is a homepage-only presentation dependency and must not enter API or MCP runtime logic.
- `static/home-scene.js` owns homepage DOM discovery, localization, input filtering,
  observers, datasets, and module Worker lifecycle; it does not import Three.js.
- `static/home-scene-worker.js` is the sole Three.js runtime owner and renders to a
  transferred `OffscreenCanvas` using the self-contained `three.home.min.js` bundle.
- Unsupported Worker, OffscreenCanvas, or WebGL capability and runtime WebGL
  failure select a deterministic inline SVG with CSS-only observatory motion;
  no main-thread renderer is retained. Reduced motion keeps the complete SVG
  composition static.
- The existing documentation registry remains the source for featured endpoints and MCP tools.

### 5.2 Architecture Non-negotiables

1. Do not introduce a frontend application framework or build pipeline.
2. Do not duplicate endpoint or MCP tool metadata in a second maintained registry.
3. Do not add frontend behavior to `src/home.rs` beyond render wiring.
4. Do not load the Three.js runtime on Scalar or MCP reference pages.
5. Retire superseded Bento presentation rules rather than retaining parallel themes.

### 5.3 Architecture Non-goals

- No API schema, database, authentication, crawler, or persistence change.
- No change to MCP transport behavior.
- No compatibility shim for the old homepage visual classes.

## 6. Ownership / Contract Snapshot

- Scalar API reference -> `src/main.rs` and generated OpenAPI metadata.
- Public homepage and MCP render context -> `src/home.rs`.
- Homepage document structure -> `templates/home.html`.
- MCP reference document structure -> `templates/docs_mcp.html`.
- Shared public shell -> `templates/docs_base.html` and shared public CSS.
- Localization -> `static/i18n/en.json`, `zh-TW.json`, and `zh-CN.json`.
- Homepage scene bridge -> `static/home-scene.js`.
- Homepage declarative fallback structure and motion -> `templates/home.html`
  and `static/home.css`.
- Homepage Three.js runtime -> `static/home-scene-worker.js` and
  `static/vendor/three.home.min.js`.
- Worker boundary rationale ->
  `docs/aegis/adr/ADR-0001-homepage-threejs-worker-boundary.md`.

## 7. Current State and Risks

- `src/home.rs` is already a large maintained source file, so changes should stay wiring-only.
- A throttled profile still contains a non-scene main-thread Layout long task;
  its cause is outside the Worker migration and remains a separate performance follow-up.
- Scene-disabled differential profiling attributes less than 200 ms of the
  initial long task to the Worker bridge plus SVG/CSS fallback surface.
- The one-time full-buffer `readPixels` operation remains Worker-only but can
  cost hundreds of milliseconds under throttling.

## 8. Alignment Use

- Read the product baseline before changing visible homepage or MCP behavior.
- Read the architecture baseline before changing routes, render context, asset loading, or metadata ownership.
- Report `scope: both` if a change affects visible behavior and its owning runtime boundary.

## 9. Compatibility Boundary

The redesign may replace layout, presentation classes, and browser-side motion,
but it must preserve public route availability, Scalar ownership, localization,
auth truth, endpoint/tool metadata, fragment links, and public accessibility.
