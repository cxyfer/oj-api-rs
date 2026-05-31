## 1. Database Query

- [x] 1.1 Add `list_difficulties(pool, source)` in `src/db/problems.rs` to return distinct trimmed non-empty difficulty values for one source.
- [x] 1.2 Implement LeetCode and Luogu known-order sorting with lexical fallback for unknown difficulty values.
- [x] 1.3 Add DB unit tests for empty omission, canonical value preservation, LeetCode order, Luogu order, and fallback order.

## 2. Public API

- [x] 2.1 Add `list_difficulties` handler in `src/api/problems.rs` using existing source validation, `spawn_blocking`, JSON response, and RFC 7807 errors.
- [x] 2.2 Register `GET /api/v1/difficulties/{source}` in `src/api/mod.rs` next to the tags endpoint.
- [x] 2.3 Add utoipa path annotation and include the handler in `src/api/openapi.rs` paths/tags as needed.

## 3. Documentation Surface

- [x] 3.1 Add a homepage/docs API card for `/api/v1/difficulties/{source}` mirroring the tags card style.
- [x] 3.2 Update related homepage/docs tests and i18n keys if route cards require them.

## 4. Verification

- [x] 4.1 Run `cargo fmt`.
- [x] 4.2 Run focused Rust tests for `db::problems` and API/homepage docs coverage.
- [x] 4.3 Run `cargo test` or the repository's appropriate full test command before marking implementation complete.
