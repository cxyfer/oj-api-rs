## Why

The current `daily_challenge` table stores a full problem snapshot for one LeetCode daily challenge per date/domain, duplicating fields already maintained in `problems` and making multi-problem or non-LeetCode daily sources awkward to support. Refactoring daily storage into date/source metadata plus JSON problem references keeps the cache smaller, avoids duplicated problem data, and aligns the API response with future multi-source daily challenge use cases.

## What Changes

- **BREAKING**: Change `GET /api/v1/daily` response from a single top-level problem object to `{ date, source, problems }`.
- Replace the daily challenge DB snapshot schema with `date`, `source`, and `problems`, where `problems` is a JSON string array of `{problem_source}:{problem_id}` references such as `["leetcode:1234", "atcoder:abc321_a"]`.
- Add schema detection and data-preserving rebuild for legacy `daily_challenge(date, domain, id, ...)` tables, mapping `com` to `leetcode.com`, `cn` to `leetcode.cn`, and each legacy row to `problems = ["leetcode:{id}"]`.
- Update Rust daily reads to parse problem references, resolve them from `problems`, preserve reference order, and assemble the new response shape.
- Update Python crawler persistence to write only daily source/date/problem references while still ensuring problem details are available in `problems`.
- Preserve existing query aliases: `domain=com|cn` and `source=leetcode.com|leetcode.cn`, with canonical response field `source`.
- For `source=leetcode.cn`, localize response problem `title`, `content`, and LeetCode links using Chinese fields when available; for `leetcode.com`, use default English fields.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `daily-challenge`: Daily challenge storage, API response shape, schema compatibility migration, and localized problem projection are changing.

## Impact

- Rust DB initialization and daily query code: `src/db/mod.rs`, `src/db/daily.rs`.
- Rust API response models and handler assembly: `src/api/daily.rs`, `src/models.rs`, `src/api/openapi.rs`.
- Python daily challenge persistence and schema initialization: `scripts/utils/database.py`, `scripts/leetcode.py`.
- Tests for schema migration, JSON reference parsing, localized projection, fallback behavior, and OpenAPI schemas.
- API clients consuming `/api/v1/daily` must adapt to the new `{ date, source, problems }` response format.
