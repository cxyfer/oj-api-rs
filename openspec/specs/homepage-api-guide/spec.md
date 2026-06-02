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
The homepage SHALL present a curated set of information only: a service summary, a dual-state auth matrix, exactly three featured endpoint cards, one ready-to-copy example request, and visible navigation to the detailed docs pages.

#### Scenario: Homepage shows the selected featured endpoints
- **WHEN** a user opens `/`
- **THEN** the page highlights exactly these three endpoints as feature cards:
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

### Requirement: Homepage uses the selected Bento Editorial layout
The homepage SHALL use the selected Bento Editorial direction rather than the earlier split-screen reference layout.

#### Scenario: User sees the first screen on desktop
- **WHEN** a user opens `/` on a desktop-sized viewport
- **THEN** the initial viewport presents the value proposition, auth matrix, three featured endpoint cards, and one example as a coherent first-screen composition without requiring detailed route catalogs to remain on the homepage
- **AND** the canonical request card occupies a wider row than the docs CTA cards, with the `GET /docs` and `GET /docs/mcp` CTA cards placed on the following row

#### Scenario: Homepage stays curated on smaller screens
- **WHEN** the homepage is rendered on tablet or mobile widths
- **THEN** the same curated content remains available in stacked or simplified layouts without moving detailed reference content back onto `/`

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

#### Scenario: Each MCP tool card contains actionable detail
- **WHEN** a user reviews an MCP tool card on `/docs/mcp`
- **THEN** the card includes the exact tool name, required inputs, corresponding REST capability, output style, and a concise usage note

#### Scenario: MCP page includes connection guidance
- **WHEN** a user reviews `/docs/mcp`
- **THEN** the page includes at least one connection/configuration example and one MCP request example

### Requirement: Public docs pages support the existing locale set
The homepage, API docs page, and MCP reference page SHALL support the existing locale set used by the project frontend (`en`, `zh-TW`, and `zh-CN`) for all visible non-technical guide copy.

#### Scenario: Default locale on first visit
- **WHEN** a user opens `/`, `/docs`, or `/docs/mcp` with no saved language preference
- **THEN** the page renders using the default frontend locale behavior already established by the project

#### Scenario: User switches docs-page language
- **WHEN** a user changes the language using the shared frontend localization mechanism
- **THEN** all translatable copy on `/`, `/docs`, and `/docs/mcp` updates using the selected locale among `en`, `zh-TW`, and `zh-CN`
