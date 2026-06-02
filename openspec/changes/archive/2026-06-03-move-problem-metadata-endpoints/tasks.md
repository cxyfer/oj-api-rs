## 1. Route Relocation

- [x] 1.1 Replace public route registrations for `GET /api/v1/tags/{source}` and `GET /api/v1/difficulties/{source}` with `GET /api/v1/problems/tags/{source}` and `GET /api/v1/problems/difficulties/{source}`.
- [x] 1.2 Keep existing `list_tags` and `list_difficulties` handler behavior unchanged, including bearer auth, source validation, response format, and database calls.

## 2. OpenAPI Updates

- [x] 2.1 Update `#[utoipa::path]` annotations for tag and difficulty discovery handlers to use the new nested paths.
- [x] 2.2 Update OpenAPI capability documentation or route inventory references so generated docs no longer advertise the old top-level paths.

## 3. Tests and Verification

- [x] 3.1 Add or update routing tests that confirm the nested difficulty endpoint returns the same data as the existing handler behavior.
- [x] 3.2 Add or update routing tests that confirm the nested tag endpoint returns the same data as the existing handler behavior.
- [x] 3.3 Verify old top-level metadata paths are not registered if route-level tests already cover public router behavior.
- [x] 3.4 Run `cargo fmt`.
- [x] 3.5 Run `cargo test`.
