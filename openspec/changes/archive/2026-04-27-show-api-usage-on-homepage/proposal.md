## Why

The project already exposes a capable public API, but the current root page mixes orientation content with incomplete route-level reference details. The user now wants the information architecture to be split more cleanly: keep `/` as a polished public landing page, but move complete HTTP API and MCP reference material into dedicated pages.

This change reduces homepage clutter, improves scanability, and makes the documentation contract more maintainable while still keeping the service easy to discover from the root path.

## What Changes

- Keep `GET /` as the public docs homepage, but narrow its role to orientation and product-facing guidance.
- Redesign `/` as a curated Bento-style landing page that highlights a small set of feature/value cards, a dual-state auth matrix, three featured endpoints, and one canonical request example.
- Add a dedicated HTTP API reference page at `GET /docs/api` that documents all public non-MCP HTTP endpoints.
- Add a dedicated MCP reference page at `GET /docs/mcp` that documents `GET /mcp`, `POST /mcp`, and all five MCP tools.
- Keep the implementation on the existing Rust + Axum + Askama + static asset stack, using a Rust-owned metadata registry as the source of truth for route and tool documentation.
- Extend the existing i18n flow so all non-technical copy on the new pages remains available in `en`, `zh-TW`, and `zh-CN`.
- Keep the README as supplementary documentation rather than the primary route-by-route reference.

## Capabilities

### New Capabilities
- `homepage-api-guide`: Provide a curated public homepage at `/` that introduces the service, explains auth expectations, highlights three representative endpoints, and points users to the detailed docs pages.
- `http-api-reference-page`: Provide a complete public HTTP API reference at `/docs/api` covering all public non-MCP routes.
- `mcp-reference-page`: Provide a complete MCP reference at `/docs/mcp` covering transport usage and all exposed MCP tools.

### Modified Capabilities
- None.

## Impact

- Public web routes in the Axum server, including new rendered routes for `/docs/api` and `/docs/mcp`.
- Askama templates for a curated homepage plus two detailed reference pages.
- A Rust-owned documentation metadata layer used to render endpoint/tool cards consistently.
- Static frontend assets and translations for the new multi-page docs flow.
- Verification work to keep docs coverage aligned with the actual public API and MCP surface.
