## Context

`daily_challenge` currently stores a full problem snapshot keyed by `(date, domain)`, while the canonical problem data already lives in `problems(source, id)`. The Rust server initializes the table in `src/db/mod.rs`, reads it in `src/db/daily.rs`, and assembles `/api/v1/daily` in `src/api/daily.rs`. The Python crawler initializes and writes the same table in `scripts/utils/database.py`, then fills the row from LeetCode data in `scripts/leetcode.py`.

The target model treats a daily challenge as a source/date record with an ordered list of problem references. A reference is a string in `{problem_source}:{problem_id}` format stored inside a JSON array, for example `["leetcode:1234", "leetcode:1", "atcoder:abc321_a"]`.

## Goals / Non-Goals

**Goals:**

- Store daily challenge membership without duplicating full problem fields.
- Support multiple problems per daily source while preserving order.
- Preserve existing `domain=com|cn` and `source=leetcode.com|leetcode.cn` query compatibility.
- Return a breaking but clean `{ date, source, problems }` response from `/api/v1/daily`.
- Migrate existing legacy `daily_challenge` rows without dropping cached data.
- Keep Rust server startup and standalone Python crawler startup compatible with both old and new databases.
- Localize LeetCode CN daily responses by projecting `title_cn` / `content_cn` into `title` / `content` when available.

**Non-Goals:**

- Introduce a general migration framework or `schema_migrations` table.
- Add foreign keys or a separate join table for daily problem references.
- Support arbitrary non-LeetCode daily source fetching in the crawler beyond representing refs in storage and response.
- Preserve the old single-problem `/api/v1/daily` response shape.

## Decisions

### Use one JSON reference column instead of a join table

The new `daily_challenge` schema will be:

```sql
CREATE TABLE IF NOT EXISTS daily_challenge (
    date TEXT NOT NULL,
    source TEXT NOT NULL,
    problems TEXT NOT NULL,
    PRIMARY KEY (date, source)
);
```

`problems` is a JSON string array of refs. Each ref is parsed with `split_once(':')`, so only the first colon separates source and id.

Alternative considered: `daily_challenge_problems` join table with `position`, `problem_source`, and `problem_id`. It offers stronger relational querying, but the current access pattern is single date/source lookup and ordered response assembly. JSON is simpler and matches existing project patterns for JSON string arrays such as `tags` and `similar_questions`.

### Upgrade old schemas at initialization

Rust and Python initialization will inspect `PRAGMA table_info(daily_challenge)`. If the table already has `date`, `source`, and `problems`, initialization is a no-op. If it has the legacy `domain` / `id` snapshot shape, initialization will rebuild it in a transaction:

1. Rename `daily_challenge` to a temporary legacy table.
2. Create the new compact table.
3. Copy legacy rows into the new table.
4. Drop the temporary legacy table after successful copy.

Legacy mapping:

- `domain = 'com'` → `source = 'leetcode.com'`
- `domain = 'cn'` → `source = 'leetcode.cn'`
- otherwise preserve the domain string as source
- `id` → `"leetcode:{id}"`

If a legacy row lacks `id`, migration should attempt to resolve `slug` from `problems(source='leetcode', slug=legacy.slug)` before skipping it. Rows that cannot form a valid reference are skipped with logging.

Alternative considered: keep dual read paths for old and new schemas. That would spread compatibility logic into API code indefinitely. Startup migration keeps runtime reads simple.

### Canonical daily source replaces response domain

Request parsing keeps `domain` as a LeetCode alias, but all internal daily reads use canonical daily source:

- omitted source/domain → `leetcode.com`
- `domain=com` or `source=leetcode.com` → `leetcode.com`
- `domain=cn` or `source=leetcode.cn` → `leetcode.cn`

The response exposes `source`, not `domain`.

### Resolve problem refs from `problems`

Rust daily reads will parse the JSON refs and resolve each `problem_source` / `problem_id` from `problems`. The response preserves reference order and skips malformed or missing refs with diagnostic logging. If no valid problem can be resolved from an existing daily row, the record is treated as unusable so the API can follow the existing no-data behavior.

### Localize at API projection time

Problem rows keep both default and Chinese fields. `/api/v1/daily` projects fields according to the daily source:

- `leetcode.cn`: `title = title_cn fallback title`, `content = content_cn fallback content`, LeetCode link host rewritten to `https://leetcode.cn` when the problem source is `leetcode`.
- `leetcode.com`: `title = title`, `content = content`, LeetCode link host rewritten to `https://leetcode.com` when the problem source is `leetcode`.
- other daily sources: use default `title`, `content`, and stored link.

The daily response does not expose `title_cn` or `content_cn`; clients needing raw multilingual fields can use problem detail endpoints.

## Risks / Trade-offs

- [Breaking API response] → Document the response change in OpenAPI and update tests to enforce the new shape.
- [No DB-level referential integrity for JSON refs] → Validate refs at read/write boundaries and log missing refs.
- [Concurrent Rust/Python initialization] → Use transactional rebuild with SQLite locking where possible; keep migration idempotent.
- [Legacy rows with missing ids] → Resolve by slug when possible; otherwise skip with logging rather than corrupting the new table.
- [CN projection may hide English fields] → Keep full multilingual data available through existing problem endpoints.
- [Malformed JSON in `problems`] → Treat the daily row as unusable and allow fallback behavior instead of panicking.

## Migration Plan

1. Add idempotent schema inspection and migration helpers in Rust DB initialization.
2. Add equivalent idempotent schema inspection and migration helpers in Python `DailyChallengeDatabaseManager._init_db`.
3. Change Python daily writes to persist `date`, canonical daily `source`, and JSON refs only.
4. Change Rust daily reads and API response assembly to consume compact refs and resolve problem details.
5. Update tests for legacy migration, new schema reads, API response shape, CN localization, and crawler persistence.
6. Update OpenAPI schemas through Rust model changes.

Rollback strategy: because the migration rebuilds the table, rollback to old code would not understand the new compact table. If rollback is required, restore the database from backup or run a reverse migration that expands each first `leetcode:{id}` ref into the legacy snapshot shape using `problems` data.