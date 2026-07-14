# Homepage Ambient Motion Spec Brief

Date: `2026-07-12`
Status: `approved`

## 1. Intent

Extend the Algorithmic Observatory identity below the hero with restrained,
CSS-only motion. The homepage should feel like one continuously operating
problem-intelligence system while preserving the editorial hierarchy and API
readability.

Success evidence:

- The full homepage has a coherent observatory-field background beneath every
  post-hero section.
- Sections reveal through bounded line, opacity, and short-distance motion as
  they enter the viewport on browsers that support CSS view timelines.
- Source cells, capability rows, endpoint rows, the integration console, auth
  rows, and final command provide matching hover and keyboard-focus feedback.
- Unsupported scroll-animation browsers receive the complete static layout.
- `prefers-reduced-motion: reduce` removes all new nonessential animation and
  transition behavior.

## 2. Scope And Boundaries

Canonical owners:

- `templates/home.html`: declarative motion markers only.
- `static/home.css`: homepage background, view-entry motion, hover/focus
  interactions, responsive behavior, and reduced-motion shutdown.
- `src/home.rs`: rendered-source contract assertions only.

No new JavaScript module, animation loop, canvas, image, dependency, locale
copy, API request, route, registry, or runtime owner is introduced. The
Three.js Worker scene and its successful/fallback behavior remain unchanged.

## 3. Visual Design

### 3.1 Observatory Field

The homepage-only `.home-shell` receives two pointer-transparent background
layers behind content:

- a low-opacity technical grid with sparse cyan and coral signal intersections;
- two oversized diagonal route traces whose highlights travel slowly across
  the page.

The field uses the existing graphite, paper, cyan, coral, and lime palette. It
must not use blur filters, glow blobs, raster assets, or a second scene.

### 3.2 View Entry

Every post-hero section receives `data-motion-section`. Within a supporting
`@supports (animation-timeline: view())` block, the section uses a bounded
fade/translate reveal and its top rule expands horizontally. Lists use
deterministic per-item delays through CSS custom properties.

The base style remains fully visible. Scroll-driven animation is enhancement,
not a content gate.

### 3.3 Interaction Feedback

- Source cells: local signal dot and a short horizontal scan.
- Capability rows: number, copy, and API identifier connect through a thin
  accent rule.
- Endpoint rows: method, route, and outbound control form a brief data-path
  highlight.
- Integration console and auth rows: restrained inset scan/highlight.
- Final command: a route line travels toward the primary action.

Hover behavior is limited to `(hover: hover) and (pointer: fine)`. Keyboard
focus uses `:focus-within` or `:focus-visible` equivalents where interactive
descendants exist.

## 4. Accessibility And Performance

- New decoration is pseudo-element based and pointer-transparent.
- Motion does not move focus targets, alter document flow, or create overflow.
- Reduced-motion disables view animations, background route motion, scans,
  transforms, and nonessential transitions while keeping all content visible.
- Animation is limited to opacity and transform where possible; no JavaScript
  frame loop, filter animation, or large repaints from moving full-page layers.
- Mobile keeps the static field but disables continuously moving background
  route highlights.

## 5. Verification

- Render-contract assertions require all section markers, stagger variables,
  view-timeline support, pointer-device gating, and reduced-motion shutdown.
- Rust tests, Clippy, formatting, JavaScript syntax, locale JSON, and
  `git diff --check` remain green.
- Browser screenshots cover desktop and mobile, plus reduced-motion state.
- Browser checks confirm no horizontal overflow and unchanged successful hero
  scene readiness.
