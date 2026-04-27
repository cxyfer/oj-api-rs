- [x] 1.1 Add public rendered routes for `/`, `/docs/api`, and `/docs/mcp` in the main Axum router without changing auth behavior for existing API, admin, or MCP endpoints.
- [x] 1.2 Add or refactor public page handlers so the three docs pages render through Askama outside the admin router.
- [x] 1.3 Introduce a Rust-owned documentation metadata model for homepage cards, HTTP route cards, MCP transport cards, and MCP tool cards.

- [x] 2.1 Redesign the homepage `/` as a Bento Editorial landing page instead of the previous split-screen-heavy reference layout.
- [x] 2.2 Implement the homepage content contract: service summary, dual-state auth matrix, exactly three featured endpoint cards, one canonical example, and visible links to `/docs/api` and `/docs/mcp`.
- [x] 2.3 Ensure the homepage featured endpoint cards are exactly `GET /api/v1/problems/{source}/{id}`, `GET /api/v1/daily`, and `GET /api/v1/similar`.
- [x] 2.4 Keep the homepage curated by removing exhaustive route/tool reference details from `/`.

- [x] 3.1 Create the `/docs/api` reference page layout and group route cards into Problems, Discovery, and Service sections.
- [x] 3.2 Document all nine public non-MCP HTTP routes on `/docs/api` as distinct cards: problem detail, problem list, tags, resolve, daily, similar by id, similar by query, status, and health.
- [x] 3.3 Ensure every `/docs/api` route card includes method, exact path, purpose, auth rule, meaningful inputs, success response shape, and one copyable example.
- [x] 3.4 Ensure all `{source}` references on `/docs/api` explicitly list the supported public source keys `leetcode`, `codeforces`, `atcoder`, `luogu`, and `spoj`.
- [x] 3.5 Implement collapsed-by-default route detail panels on `/docs/api` with independent expansion, keyboard accessibility, and stable fragment IDs.

- [x] 4.1 Create the `/docs/mcp` reference page layout with separate transport and tool sections.
- [x] 4.2 Document `POST /mcp` and `GET /mcp` as distinct MCP transport cards with their different responsibilities.
- [x] 4.3 Add five MCP tool cards for `resolve_problem`, `get_problem`, `get_daily_challenge`, `find_similar_problems`, and `get_platform_status`.
- [x] 4.4 Ensure every MCP tool card includes exact tool name, required inputs, corresponding REST capability, output style, and a concise usage note.
- [x] 4.5 Add at least one connection/configuration example and one MCP request example to `/docs/mcp`.

- [x] 5.1 Add or update Askama templates and dedicated public stylesheets/components needed for the three-page docs flow while keeping the pages visually separate from the admin shell.
- [x] 5.2 Extend locale keys in `en`, `zh-TW`, and `zh-CN` so all visible non-technical copy on `/`, `/docs/api`, and `/docs/mcp` is translated through the existing i18n mechanism.
- [x] 5.3 Keep technical tokens such as methods, paths, source keys, query parameter names, and MCP tool names stable across locales.

- [x] 6.1 Verify `GET /`, `GET /docs/api`, and `GET /docs/mcp` return public docs pages without admin authentication or admin-only navigation.
- [x] 6.2 Verify the homepage auth matrix truthfully documents both token-auth-disabled and token-auth-enabled behavior for `/`, `/health`, `/api/v1/*`, `/status`, and `/mcp`.
- [x] 6.3 Verify `/docs/api` covers exactly the nine public non-MCP HTTP routes and no admin routes.
- [x] 6.4 Verify `/docs/mcp` covers `GET /mcp`, `POST /mcp`, and exactly the five exposed MCP tools.
- [x] 6.5 Verify all copyable examples remain aligned with current public route behavior and supported source keys.
- [x] 6.6 Verify language switching works across `en`, `zh-TW`, and `zh-CN` on all three public docs pages.
- [x] 6.7 Verify detail panels on `/docs/api` and `/docs/mcp` are collapsed by default, expandable independently, and usable via keyboard navigation.
