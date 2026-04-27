## Context

The server already uses Axum for routing, Askama for HTML templates, and `/static` for frontend assets. Public traffic currently lands on API-style endpoints such as `/api/v1/*`, `/health`, and `/mcp`, while rendered HTML pages are limited to the authenticated `/admin/*` area.

The first version of this change added a public landing page at `/`, but the result still concentrated too much detailed reference material into one page while remaining incomplete at the route level. The user now wants a cleaner split:

- `/` should remain the polished public entry point.
- Full HTTP API reference should move to `/docs/api`.
- Full MCP reference should move to `/docs/mcp`.

The new direction should keep the docs experience visually strong while making the documentation contract explicit and mechanically implementable.

## Resolved Constraints

- The implementation SHALL stay within the existing Rust + Axum + Askama + static CSS/i18n stack.
- The documentation source of truth SHALL be a Rust-owned registry rendered by Askama, not route prose duplicated only in HTML.
- The homepage visual direction SHALL be Bento Editorial rather than the previous split-screen hero composition.
- The homepage SHALL remain curated rather than exhaustive.
- The homepage SHALL keep exactly three featured endpoint cards: `GET /api/v1/problems/{source}/{id}`, `GET /api/v1/daily`, and `GET /api/v1/similar`.
- The homepage SHALL include one canonical copyable example request.
- The homepage SHALL include a dual-state auth matrix that documents behavior for token-auth-disabled and token-auth-enabled deployments.
- Detailed HTTP API reference SHALL live at `/docs/api`.
- Detailed MCP reference SHALL live at `/docs/mcp`.
- The docs pages SHALL NOT use tabs or separate frontend bundles; they remain server-rendered pages.
- The MCP detail depth SHALL include tool name, required inputs, corresponding REST capability, and output style.
- The detail pages SHALL use cards with collapsed details by default, support keyboard-accessible expansion, and expose stable fragment IDs for deep-linking.

## Goals / Non-Goals

**Goals:**
- Keep `/` as the discoverable public entry point for the service.
- Make the homepage visually polished but information-light enough to scan quickly.
- Move exhaustive route/tool reference material out of the homepage and into dedicated pages.
- Ensure every public HTTP route and every exposed MCP tool is documented explicitly.
- Keep docs content synchronized with actual route behavior through a Rust-owned metadata model.
- Preserve existing auth behavior and avoid any coupling to admin-only UI.
- Keep all visible guide copy available in `en`, `zh-TW`, and `zh-CN`.

**Non-Goals:**
- Introduce a separate docs framework or client-side SPA.
- Replace the README as the long-form project document.
- Document admin-only routes on the public docs pages.
- Build a fully interactive API explorer or runtime schema introspection system.

## Decisions

### 1. Keep `/` as a curated homepage, not the full reference page
The root route will continue to render the public docs homepage, but the page will focus on orientation rather than exhaustive endpoint coverage.

**Why:**
- The user still wants `/` to be the public entry point.
- A curated homepage is easier to scan and visually stronger than an all-in-one long reference page.
- It creates a clear split between “what this service does” and “how every route works.”

**Alternatives considered:**
- Keep all detailed route reference on `/` → rejected because the page becomes cluttered and harder to scan.
- Move all docs under `/docs/*` and stop rendering `/` → rejected because it weakens discoverability.

### 2. Add two dedicated detail pages: `/docs/api` and `/docs/mcp`
The documentation will be split into two dedicated public pages:
- `/docs/api` for complete non-MCP HTTP API reference
- `/docs/mcp` for MCP transport and tool reference

**Why:**
- The user explicitly asked to separate detailed API and MCP request guidance into individual pages.
- API and MCP serve different integration modes and deserve distinct explanations.
- This reduces homepage density while making the detailed content easier to maintain.

**Alternatives considered:**
- Same-page tabs → rejected because the user chose independent routes.
- `/api-reference` + `/mcp-reference` → rejected in favor of `/docs/api` + `/docs/mcp`.

### 3. Use a Rust-owned documentation registry rendered by Askama
Route and tool metadata should be stored in Rust structs and passed into templates, rather than being expressed only as handwritten HTML prose.

**Why:**
- Public route behavior, auth expectations, and supported source keys should remain close to backend truth.
- This reduces drift between route implementation, homepage/docs templates, and README copy.
- It fits the current architecture better than route reflection or a new schema-generation system.

**Alternatives considered:**
- Hardcode all endpoint details directly in HTML/i18n JSON → rejected because drift risk is too high.
- Add a full OpenAPI or external docs generation layer → rejected because it expands scope too much.

### 4. Use a Bento Editorial homepage with constrained first-screen content
The homepage should use a Bento-style layout rather than the earlier split hero. On desktop first screen, it should show:
- product/value introduction
- dual-state auth matrix
- three featured endpoint cards
- one canonical request example spanning a wider row than the docs CTAs
- clear CTAs to `/docs/api` and `/docs/mcp` on the following row

**Why:**
- The user prefers a curated homepage that still preserves some product-style visual quality.
- Bento grouping supports “few features + few examples + clear next steps” better than the previous denser two-column docs composition.
- It makes the first screen informative without requiring the detailed route catalog to remain on the homepage.

**Alternatives considered:**
- Preserve the old split hero and only trim content → rejected because the user explicitly asked for page separation.
- Make the homepage minimal with no featured endpoints → rejected because the user wants some functional highlights retained.

### 5. Define an exact homepage content contract
The homepage must remain constrained. It should contain:
- a service summary/value proposition
- a dual-state auth matrix with rows for `/`, `/health`, `/api/v1/*`, `/status`, and `/mcp`
- exactly three featured endpoint cards:
  - `GET /api/v1/problems/{source}/{id}`
  - `GET /api/v1/daily`
  - `GET /api/v1/similar`
- one canonical copyable example request
- links to `/docs/api`, `/docs/mcp`, and optionally the README

**Why:**
- This gives the homepage enough technical credibility without turning it back into the full reference.
- The exact endpoint list eliminates implementation ambiguity.

### 6. Define an exact `/docs/api` content contract
`/docs/api` must document all public non-MCP HTTP routes as separate cards:
- `GET /api/v1/problems/{source}/{id}`
- `GET /api/v1/problems/{source}`
- `GET /api/v1/tags/{source}`
- `GET /api/v1/resolve/{*query}`
- `GET /api/v1/daily`
- `GET /api/v1/similar/{source}/{id}`
- `GET /api/v1/similar`
- `GET /status`
- `GET /health`

Each route card must include:
- method
- exact path
- purpose
- auth rule
- required/meaningful inputs
- success response shape
- one copyable example
- collapsible details for secondary information

The page should group these cards as:
- Problems
- Discovery
- Service

All route references that use `{source}` must explicitly list the supported public source keys:
`leetcode`, `codeforces`, `atcoder`, `luogu`, `spoj`.

**Why:**
- This satisfies the “complete API details” requirement without bloating the homepage.
- One-card-per-route avoids vague family-only summaries.

### 7. Define an exact `/docs/mcp` content contract
`/docs/mcp` must document MCP in two layers:

**Transport cards:**
- `POST /mcp` as the Streamable HTTP entrypoint for initialize/tool calls
- `GET /mcp` as the SSE/session-resume endpoint

**Tool cards:**
- `resolve_problem`
- `get_problem`
- `get_daily_challenge`
- `find_similar_problems`
- `get_platform_status`

Each tool card must include:
- exact tool name
- required inputs
- corresponding REST capability
- output style
- a concise usage note

The page should also include at least one connection/configuration example and one MCP request example.

**Why:**
- The user wants MCP documented separately and completely.
- MCP transport semantics differ enough from REST that they need a dedicated page.

### 8. Keep documentation copy translatable, but keep technical tokens stable
All user-facing guide copy should flow through the existing locale system. Technical tokens such as method names, paths, query keys, enum values, source keys, and MCP tool names should remain stable and untranslated.

**Why:**
- This preserves consistency across locales and prevents technical drift.
- It reduces translation churn for code-like strings.

## Risks / Trade-offs

- **[Rust docs registry drifts from route behavior]** → Keep route/tool metadata centralized and update it in the same change as backend route changes.
- **[Homepage becomes too bare after moving details out]** → Keep three featured endpoint cards plus one canonical example so `/` still feels functional.
- **[Detailed docs pages become long and unwieldy]** → Use grouped cards with collapsed details and deep-linkable fragment IDs.
- **[Auth behavior is misrepresented]** → Use a dual-state auth matrix and per-card auth labels instead of relying on one global sentence.
- **[Supported source list becomes stale]** → Derive or share the same public source constant used by problem-route docs.
- **[MCP transport is confused with ordinary REST]** → Give `GET /mcp` and `POST /mcp` separate transport cards and explain their distinct roles.
- **[Locale coverage becomes incomplete]** → Treat all new visible labels and explanatory copy as part of the i18n workflow.

## Property-Based Test Targets

1. **HTTP route coverage invariance**
   - **Invariant:** `/docs/api` documents exactly the nine public non-MCP HTTP routes and no admin routes.
   - **Falsification strategy:** Compare rendered route cards against the expected set; fail on missing or extra entries.

2. **MCP coverage invariance**
   - **Invariant:** `/docs/mcp` documents exactly `GET /mcp`, `POST /mcp`, and the five exposed MCP tools.
   - **Falsification strategy:** Compare rendered transport/tool entries against the backend metadata set.

3. **Auth truthfulness invariance**
   - **Invariant:** The homepage auth matrix always describes both token-auth-disabled and token-auth-enabled behavior correctly for `/`, `/health`, `/api/v1/*`, `/status`, and `/mcp`.
   - **Falsification strategy:** Validate matrix labels against the intended auth policy model, including conditional routes.

4. **Supported source consistency**
   - **Invariant:** All `{source}` documentation and examples use only `leetcode`, `codeforces`, `atcoder`, `luogu`, or `spoj`.
   - **Falsification strategy:** Scan docs metadata and rendered examples for unsupported source keys.

5. **Locale completeness**
   - **Invariant:** Every translatable visible docs string used by the new pages resolves in `en`, `zh-TW`, and `zh-CN`.
   - **Falsification strategy:** Enumerate docs-page i18n keys and fail if any key is missing in one locale.

6. **Navigation reachability**
   - **Invariant:** The homepage always provides working navigation paths to `/docs/api` and `/docs/mcp`.
   - **Falsification strategy:** Check rendered anchor targets/URLs and ensure they resolve to public pages.

7. **Collapse-state behavior**
   - **Invariant:** Detailed route/tool cards render collapsed by default and can be expanded independently without JS-only dependence.
   - **Falsification strategy:** Inspect rendered markup for semantic collapsible structure and independent IDs.

## Migration Plan

1. Replace the current single-page-heavy docs emphasis with a three-page public docs flow.
2. Add public handlers and templates for `/docs/api` and `/docs/mcp`.
3. Introduce a Rust-owned docs metadata model for homepage, HTTP API reference, and MCP reference content.
4. Update the homepage to the Bento Editorial layout with constrained first-screen content.
5. Add or update translations for all visible non-technical copy on the three public docs pages.
6. Verify docs coverage, auth messaging, i18n behavior, and public accessibility.
7. Rollback strategy: remove the new docs routes/templates/metadata and restore the previous single homepage behavior.
