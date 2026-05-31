## Why

Clients can already discover available tags through `/api/v1/tags/{source}`, but they must guess platform-specific difficulty values before using the existing `difficulty` filter. A difficulty discovery endpoint makes filtering UIs and integrations consistent across supported online judges.

## What Changes

- Add a public API endpoint similar to `/api/v1/tags/{source}` that returns distinct difficulty values for a supported source.
- Return only non-empty difficulties present in stored problem data.
- Sort difficulties in platform-appropriate order where known, with deterministic fallback ordering for unknown values.
- Reuse existing source validation, Bearer token auth, RFC 7807 error style, and blocking DB access pattern.

## Capabilities

### New Capabilities
- `difficulty-list-api`: Public API for discovering per-source difficulty values.

### Modified Capabilities
- `problem-query`: Problem query API gains a discoverability endpoint for difficulty filter values.

## Impact

- Public API routes in `src/api/mod.rs` and handlers in `src/api/problems.rs`.
- Database query logic in `src/db/problems.rs`.
- OpenAPI generation in `src/api/openapi.rs` via handler annotations.
- API documentation/homepage cards if the existing tag endpoint is documented there.
- Inline Rust tests covering sorting, invalid source handling, and empty/malformed difficulty behavior.
