## 1. Core Implementation

- [x] 1.1 Add batch request/response types to `src/api/problems.rs` (BatchItem, BatchQuery, BatchNotFoundItem, BatchResponse)
- [x] 1.2 Add `record_to_summary` helper to convert ProblemRecord → ProblemSummary without similar_questions hydration
- [x] 1.3 Implement `batch_problems` handler with validation (empty, max size, source) and spawn_blocking DB loop
- [x] 1.4 Register `POST /api/v1/problems/batch` route in `src/api/mod.rs` with `post` import

## 2. Documentation

- [x] 2.1 Add batch endpoint `HttpRouteCard` to `src/home.rs` (Problems group, 9→10 cards)
- [x] 2.2 Update `home.html` route count text (nine → ten)
- [x] 2.3 Add `problem_batch` i18n keys to en.json, zh-TW.json, zh-CN.json
- [x] 2.4 Update `api_body` i18n key in all three locale files (nine → ten)
- [x] 2.5 Add batch endpoint to README.md Problems section

## 3. Verification

- [x] 3.1 Update test assertions in `src/home.rs` for card count (9→10)
- [x] 3.2 Pass `cargo clippy` and `cargo fmt --check`
- [x] 3.3 Pass all 13 home module tests
