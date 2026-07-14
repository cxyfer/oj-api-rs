# homepage-api-guide Specification

## Purpose
Define the public documentation experience for `oj-api-rs`, including the curated homepage at `/`, the interactive API docs at `/docs`, the compatibility redirect at `/docs/api`, and the MCP reference at `/docs/mcp`.

## Requirements

### Requirement: Curated public docs homepage at the root path
The system SHALL expose a public homepage at `/` that renders an HTML documentation-style landing page introducing the OJ API service. This route SHALL remain public, separate from `/admin/*`, and focused on orientation rather than exhaustive reference detail.

#### Scenario: Anonymous user visits the root path
- **WHEN** a client sends `GET /` without admin credentials
- **THEN** the system returns HTTP 200 with an HTML documentation homepage describing the API service

#### Scenario: Homepage remains separate from admin shell
- **WHEN** the homepage is rendered for `/`
- **THEN** it SHALL NOT require admin session state or render admin navigation intended for `/admin/*`

### Requirement: Homepage preserves a constrained content contract
The homepage SHALL present a curated set of information only: a service summary, a guided Spatial Intelligence Map, a dual-state auth matrix, exactly three featured endpoint highlights, at least one ready-to-copy integration example, and visible navigation to the detailed docs pages.

#### Scenario: Homepage shows the selected featured endpoints
- **WHEN** a user opens `/`
- **THEN** the page highlights exactly these three endpoints as curated feature rows:
  - `GET /api/v1/problems/{source}/{id}`
  - `GET /api/v1/daily`
  - `POST /api/v1/similar`

#### Scenario: Homepage exposes detailed-doc entry points
- **WHEN** a user scans the homepage for next steps
- **THEN** the page provides visible links to `GET /docs` and `GET /docs/mcp`
- **AND** `/docs/api` serves as the compatibility entry point that redirects to `/docs`

#### Scenario: Homepage includes one canonical example
- **WHEN** a user reviews the homepage quick-start area
- **THEN** the page shows at least one concrete, copyable request example

### Requirement: Homepage includes a dual-state auth matrix
The homepage SHALL explain auth behavior using a matrix that documents both token-auth-disabled and token-auth-enabled deployments.

#### Scenario: User reviews auth rules
- **WHEN** a user opens `/`
- **THEN** the auth matrix includes separate rows for `/`, `/health`, `/api/v1/*`, `/status`, and `/mcp`
- **AND** the matrix shows how access changes when token auth is disabled versus enabled

### Requirement: Homepage uses the selected Algorithmic Observatory direction
The homepage SHALL use the selected Algorithmic Observatory direction with a full-bleed, guided Three.js Spatial Intelligence Map rather than the superseded Bento Editorial layout.

#### Scenario: User sees the first screen on desktop
- **WHEN** a user opens `/` on a desktop-sized viewport
- **THEN** the initial viewport presents the OJ API brand, Problem Intelligence Infrastructure positioning, primary `GET /docs` command, deployment metrics, and a nonblank guided Spatial Intelligence Map
- **AND** the viewport leaves a visible hint of the following content without requiring the auth matrix, featured endpoints, or exhaustive route catalogs to compete with the hero

#### Scenario: Homepage stays curated on smaller screens
- **WHEN** the homepage is rendered on tablet or mobile widths
- **THEN** the same curated content remains available in stacked or simplified layouts with a reduced scene budget and no overlapping text or controls
- **AND** detailed reference content remains outside `/`

### Requirement: Homepage motion remains adaptive and accessible
The homepage SHALL keep the Three.js scene within explicit rendering budgets and SHALL preserve complete content access when motion is reduced or WebGL is unavailable.

#### Scenario: User prefers reduced motion
- **WHEN** the browser reports `prefers-reduced-motion: reduce`
- **THEN** the homepage renders a stable scene frame and disables nonessential continuous motion

#### Scenario: WebGL is unavailable
- **WHEN** the browser cannot initialize the Three.js renderer
- **THEN** the homepage retains readable HTML content, navigation, calls to action, and a deterministic CSS background

#### Scenario: Scene leaves the viewport
- **WHEN** the hero is no longer visible or the document is hidden
- **THEN** the render loop pauses until rendering is needed again

### Requirement: Interactive API docs at `/docs`
The system SHALL expose a public interactive API documentation page at `/docs` backed by the generated OpenAPI spec. This page SHALL document all public non-MCP HTTP endpoints and all JSON admin API endpoints, while excluding admin HTML page routes.

#### Scenario: User opens the API docs page
- **WHEN** a client sends `GET /docs`
- **THEN** the system returns HTTP 200 with an interactive API docs page

#### Scenario: Legacy API docs path redirects
- **WHEN** a client sends `GET /docs/api`
- **THEN** the system returns a permanent redirect to `/docs`

#### Scenario: Public API coverage is documented
- **WHEN** a user reviews the API docs page
- **THEN** the documented public endpoints include:
  - `GET /api/v1/problems/{source}/{id}`
  - `POST /api/v1/problems/batch`
  - `GET /api/v1/problems/{source}`
  - `GET /api/v1/random`
  - `GET /api/v1/problems/tags/{source}`
  - `GET /api/v1/problems/difficulties/{source}`
  - `GET /api/v1/resolve/{*query}`
  - `GET /api/v1/daily`
  - `GET /api/v1/similar/{source}/{id}`
  - `POST /api/v1/similar`
  - `GET /status`
  - `GET /health`

#### Scenario: Admin JSON coverage is documented
- **WHEN** a user reviews the API docs page
- **THEN** the documented admin JSON endpoints include the problem, token, settings, crawler, and embedding API routes under `/admin/api/*`
- **AND** admin HTML routes under `/admin/*` are excluded

### Requirement: Dedicated MCP reference page at `/docs/mcp`
The system SHALL expose a public HTML reference page at `/docs/mcp` that documents MCP transport usage and all exposed MCP tools.

#### Scenario: User opens the MCP reference page
- **WHEN** a client sends `GET /docs/mcp`
- **THEN** the system returns HTTP 200 with an HTML page documenting `GET /mcp`, `POST /mcp`, and these exact tools:
  - `resolve_problem`
  - `get_problem`
  - `get_daily_challenge`
  - `find_similar_problems`
  - `get_platform_status`

#### Scenario: MCP transport is documented as two distinct surfaces
- **WHEN** a user reviews the MCP transport section
- **THEN** the page documents `POST /mcp` as the Streamable HTTP entrypoint for initialize/tool calls
- **AND** documents `GET /mcp` as the SSE/session-resume endpoint

#### Scenario: Each MCP tool reference contains actionable detail
- **WHEN** a user reviews an MCP tool reference on `/docs/mcp`
- **THEN** the reference includes the exact tool name, required inputs, corresponding REST capability, output style, and a concise usage note

#### Scenario: MCP page includes connection guidance
- **WHEN** a user reviews `/docs/mcp`
- **THEN** the page includes at least one connection/configuration example and one MCP request example

### Requirement: Localized guide pages support the existing locale set
The homepage and MCP reference page SHALL support the existing locale set used by the project frontend (`en`, `zh-TW`, and `zh-CN`) for all visible non-technical guide copy. The Scalar-owned `/docs` page remains outside the shared frontend localization mechanism.

#### Scenario: Default locale on first visit
- **WHEN** a user opens `/` or `/docs/mcp` with no saved language preference
- **THEN** the page renders using the default frontend locale behavior already established by the project

#### Scenario: User switches docs-page language
- **WHEN** a user changes the language using the shared frontend localization mechanism
- **THEN** all translatable copy on `/` and `/docs/mcp` updates using the selected locale among `en`, `zh-TW`, and `zh-CN`
