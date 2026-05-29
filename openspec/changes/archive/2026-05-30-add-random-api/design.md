## Context

The existing API supports listing/filtering problems by specific source (`GET /api/v1/problems/{source}`), fetching single problems by ID, and searching via vector similarity. There is no endpoint for retrieving a random sampling of problems across all platforms.

Existing patterns to follow:
- All DB queries go through `tokio::task::spawn_blocking` with `ro_pool`
- Handlers validate inputs then delegate to `src/db/` functions returning `Option<T>` or vectors
- Response construction uses `build_problem_detail_response` for full-detail output
- Utoipa annotations document all public endpoints
- `VALID_SOURCES` const is shared across handlers

The `problems` table has mixed difficulty representations per platform:
- LeetCode: "Easy", "Medium", "Hard" (standardized)
- Luogu/SPOJ: "暂无评定", "入门", "普及−", "普及/提高−", "普及+/提高", "提高+/省选−", "省选/NOI−", "NOI/NOI+/CTSC"
- Codeforces/AtCoder: difficulty is typically NULL; rating is the difficulty indicator

## Goals / Non-Goals

**Goals:**
- Single endpoint `GET /api/v1/random` returning random problems with full detail
- Support optional filtering: `source`, `difficulty`, `tags`, `tag_mode`, `rating_min`, `rating_max`
- Cross-platform difficulty mapping (easy/medium/hard → per-platform conditions)
- Same auth and error conventions as existing `/api/v1/*` routes

**Non-Goals:**
- Deterministic seeding / reproducible randomness
- Excluding previously-seen problems from results
- Returning summaries only (no `detail` toggle — always full detail)
- Pagination (this is random sampling, not listing)

## Decisions

### D1: New DB module rather than extending `list_problems`

Independent `src/db/random.rs` with its own query builder. The `list_problems` function carries pagination, search, and sort logic that random doesn't need. Conversely, random needs cross-platform difficulty mapping and column-level `source` filtering that `list_problems` doesn't have. Keeping them separate follows KISS — each function stays ~100 lines without parameter explosion.

**Alternatives considered:**
- Extending `list_problems` with optional source/difficulty-mapping parameters → rejected: adds branching complexity to an already ~150-line function
- Extracting shared `WHERE` builder → rejected: only ~50 lines of shared filter logic; abstraction cost exceeds duplication

### D2: Difficulty mapping via `DifficultyMode` enum

```rust
enum DifficultyMode {
    Standard(Vec<&'static str>),   // exact match on difficulty column
    RatingRange(Option<f64>, Option<f64>),  // filter by rating column
}
```

When `difficulty` param is one of `easy`/`medium`/`hard`, build a per-source `OR` chain:
- LeetCode → `DifficultyMode::Standard` with mapped values
- Luogu/SPOJ → `DifficultyMode::Standard` with mapped values
- Codeforces/AtCoder → `DifficultyMode::RatingRange` (NULL difficulty → rating is the difficulty signal)
- Sources not specified in the filter → excluded from the chain for that value

When `difficulty` param is anything else, pass through to `LOWER(difficulty) = LOWER(?)` for case-insensitive native matching.

**Alternatives considered:**
- A single SQL `CASE WHEN` block → rejected: too large and hard to debug
- Multiple `SELECT` with `UNION ALL` → rejected: SQLite handles `OR` chains fine, keeps code simpler

### D3: 200 with empty array when no matches

Consistent with `list_problems` behavior (empty `data` array). 404 semantically means "this resource type doesn't exist" — but a valid filter query that happens to match nothing is a successful query, not a missing resource.

### D4: `ORDER BY RANDOM()` with `LIMIT ?count`

SQLite's built-in randomization. For tables under 100k rows this is fast enough. If profiling later shows issues, options include `rowid IN (ABS(RANDOM()) % max_rowid)` for a single random pick, but that short-circuits when `count > 1`.

## Risks / Trade-offs

- **[Performance] `ORDER BY RANDOM()` does full table scan** → Mitigation: acceptable for current data scale (~50k problems total); over-fetch + post-filter not needed since the query returns the random results directly
- **[Variability] No deterministic seed** → Non-goal. Acceptable for the use case.
- **[count semantics] `count` means "up to N"** → number of matching rows may be less than requested. Respond with whatever is available. Document this clearly.
- **[N+1 queries] `build_problem_detail_response` resolves similar_questions per-record** → Same pattern as `batch_problems` with `detail=true`. Not a regression.
