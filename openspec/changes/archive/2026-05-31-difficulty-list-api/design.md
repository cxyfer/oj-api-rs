## Context

The public API already exposes `GET /api/v1/tags/{source}` to discover tag filter values and `GET /api/v1/problems/{source}` supports a case-insensitive `difficulty` filter. Difficulty values are platform-specific: LeetCode uses Easy/Medium/Hard, Luogu uses Chinese tier labels, and other sources may store numeric or judge-specific labels. Clients currently need out-of-band knowledge to build correct difficulty filters.

## Goals / Non-Goals

**Goals:**
- Add a public discovery endpoint for distinct difficulty values per supported source.
- Keep the API shape close to `/api/v1/tags/{source}` for predictable client integration.
- Return only stored, non-empty difficulty values and order them deterministically.
- Use existing auth, source validation, error response, OpenAPI, and `spawn_blocking` DB patterns.

**Non-Goals:**
- Normalize or migrate stored difficulty values.
- Add new difficulty taxonomies for sources that do not currently store difficulty data.
- Change existing `difficulty` filtering semantics on problem list endpoints.
- Add an admin-only difficulty endpoint unless needed by the public endpoint implementation.

## Decisions

- Use `GET /api/v1/difficulties/{source}` returning `Vec<String>`.
  - Rationale: mirrors `/api/v1/tags/{source}` while keeping difficulty metadata a separate resource.
  - Alternative considered: `GET /api/v1/tags/{source}?kind=difficulty`, rejected because difficulty is already a first-class filter field, not a tag variant.

- Preserve canonical stored difficulty strings in responses.
  - Rationale: clients can pass returned values directly to the existing `difficulty` filter without lossy case or locale conversion.
  - Alternative considered: lowercase all values like `list_tags`; rejected because Luogu labels and judge-specific strings are canonical display/filter values.

- Sort with known platform order first, then deterministic lexical fallback.
  - Rationale: LeetCode and Luogu have meaningful difficulty progressions; unknown values still remain stable without introducing platform-specific guesses.
  - Alternative considered: pure alphabetical sorting; rejected because it would put user-facing difficulty progressions in unintuitive order.

- Implement DB access as `list_difficulties(pool, source) -> Option<Vec<String>>` in `src/db/problems.rs`.
  - Rationale: this matches `list_tags`, keeps SQL synchronous inside the DB layer, and lets the handler continue using `tokio::task::spawn_blocking`.
  - Alternative considered: compute difficulties from `list_problems`; rejected because it would be paginated, slower, and less direct.

## Risks / Trade-offs

- Stored data may contain inconsistent casing or duplicate semantic labels → Query distinct trimmed values and use deterministic ordering; do not silently merge values beyond exact stored strings.
- Some sources may have no difficulty data → Return HTTP 200 with an empty array, matching discovery endpoint expectations.
- Platform ordering can grow over time → Keep known-order mapping small and local to the DB/API layer; fallback lexical order covers new values safely.
- Documentation may drift from OpenAPI → Update handler annotations and homepage API card together with route registration.
