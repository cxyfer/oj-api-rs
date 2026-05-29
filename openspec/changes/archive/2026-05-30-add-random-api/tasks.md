## 1. Database Layer

- [x] 1.1 Create `src/db/random.rs` with `DifficultyMode` enum and `random_problems()` function
- [x] 1.2 Implement `WHERE` clause builder with optional source filter, difficulty mapping, tag filter, tag_mode, and rating range
- [x] 1.3 Implement `ORDER BY RANDOM() LIMIT ?count` query via `spawn_blocking`
- [x] 1.4 Register `pub mod random;` in `src/db/mod.rs`

## 2. API Handler

- [x] 2.1 Create `src/api/random.rs` with `RandomQuery` struct (source, difficulty, tags, tag_mode, rating_min, rating_max, count)
- [x] 2.2 Implement `validate_random_query()` with source validation, count limits (1-20), rating range check, tag_mode validation
- [x] 2.3 Implement `random_problems` handler: parse query, validate, call `db::random::random_problems` via `spawn_blocking`, build `ProblemDetailResponse` array
- [x] 2.4 Add full utoipa documentation (`#[utoipa::path]`) with all params, response schemas, and security

## 3. Route Registration

- [x] 3.1 Register `GET /api/v1/random` route in `src/api/mod.rs`'s `public_router()`
- [x] 3.2 Add `pub mod random;` module declaration in `src/api/mod.rs`

## 4. Verification

- [x] 4.1 Run `cargo build --release` to confirm compilation
- [x] 4.2 Run `cargo test` to ensure no regressions
- [x] 4.3 Run `cargo clippy` to confirm code quality
- [x] 4.4 Manually test endpoint with curl: basic random, filtered, edge cases (invalid source, count=0, rating_min > rating_max, empty result set)
