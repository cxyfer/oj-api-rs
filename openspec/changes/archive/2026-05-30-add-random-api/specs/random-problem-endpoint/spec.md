## ADDED Requirements

### Requirement: Random problem retrieval

The system SHALL provide an endpoint `GET /api/v1/random` that returns a random set of problems as full `ProblemDetailResponse` objects, honoring optional filter parameters.

#### Scenario: Basic random query with no filters

- **WHEN** client sends `GET /api/v1/random` with no query parameters
- **THEN** system returns `200` with `{"results": [ProblemDetailResponse]}` containing 1 random problem from any source

#### Scenario: Random query with count

- **WHEN** client sends `GET /api/v1/random?count=5`
- **THEN** system returns `200` with `{"results": [...]}` containing up to 5 random problems

### Requirement: Source filtering

The system SHALL accept an optional `source` query parameter to restrict results to a single platform. Valid values SHALL be the same as the existing `VALID_SOURCES` list (`leetcode`, `atcoder`, `codeforces`, `luogu`, `spoj`).

#### Scenario: Valid source filter

- **WHEN** client sends `GET /api/v1/random?source=leetcode`
- **THEN** system returns random problems only from LeetCode

#### Scenario: Invalid source filter

- **WHEN** client sends `GET /api/v1/random?source=invalid`
- **THEN** system returns `400` with `{"type":"about:blank","title":"Bad Request","status":400,"detail":"invalid source: invalid"}`

### Requirement: Cross-platform difficulty mapping

The system SHALL accept an optional `difficulty` query parameter. When the value is one of `easy`, `medium`, or `hard`, the system SHALL apply per-platform difficulty mapping:

- **LeetCode**: Exact match on `difficulty` column (case-insensitive)
- **Luogu / SPOJ**: Exact match on `difficulty` column for native values
- **Codeforces / AtCoder**: Mapped to `rating` column ranges (1200/1800 thresholds) since these platforms typically have NULL difficulty

When the difficulty value is NOT `easy`/`medium`/`hard`, the system SHALL perform case-insensitive exact matching on the `difficulty` column, consistent with the existing `list_problems` behavior.

#### Scenario: Standard difficulty filter across all platforms

- **WHEN** client sends `GET /api/v1/random?difficulty=easy`
- **THEN** system returns random easy-rated problems using per-platform difficulty or rating mapping

#### Scenario: Native difficulty filter for specific platform

- **WHEN** client sends `GET /api/v1/random?source=luogu&difficulty=普及−`
- **THEN** system returns random Luogu problems where `difficulty` column equals `普及−` (case-insensitive)

### Requirement: Tag filtering

The system SHALL accept optional `tags` and `tag_mode` query parameters. `tags` is a comma-separated list of tag names. `tag_mode` SHALL be either `any` (default) or `all`, determining whether a problem must match any or all specified tags.

#### Scenario: Any-tag filter

- **WHEN** client sends `GET /api/v1/random?tags=dp,graph`
- **THEN** system returns random problems that have at least one of the tags "dp" or "graph"

#### Scenario: All-tag filter

- **WHEN** client sends `GET /api/v1/random?tags=dp,graph&tag_mode=all`
- **THEN** system returns random problems that have both "dp" and "graph" tags

### Requirement: Rating range filtering

The system SHALL accept optional `rating_min` and `rating_max` query parameters (floating-point values) to filter problems by their rating score. If both are provided, `rating_min` MUST be less than or equal to `rating_max`.

#### Scenario: Valid rating range

- **WHEN** client sends `GET /api/v1/random?rating_min=1500&rating_max=2000`
- **THEN** system returns random problems with rating between 1500 and 2000 inclusive

#### Scenario: Invalid rating range

- **WHEN** client sends `GET /api/v1/random?rating_min=2000&rating_max=1500`
- **THEN** system returns `400` with detail "rating_min must be <= rating_max"

### Requirement: Count parameter

The system SHALL accept an optional `count` query parameter (positive integer) specifying the maximum number of problems to return. Default SHALL be `1`. Maximum SHALL be `20`. If fewer matching problems exist than the requested count, the system SHALL return all matching problems.

#### Scenario: Count within limits

- **WHEN** client sends `GET /api/v1/random?count=10`
- **THEN** system returns up to 10 random problems

#### Scenario: Count exceeding maximum

- **WHEN** client sends `GET /api/v1/random?count=50`
- **THEN** system returns `400` with detail "count must be between 1 and 20"

#### Scenario: Fewer matches than requested count

- **WHEN** client sends `GET /api/v1/random?count=20` but only 3 problems match the filters
- **THEN** system returns `200` with 3 problems in the results array

### Requirement: Response format

The system SHALL return results in a JSON object with a `results` key containing an array of `ProblemDetailResponse` objects. Each object SHALL include all fields from `ProblemDetailResponse` (id, source, slug, title, title_cn, difficulty, ac_rate, rating, contest, problem_index, tags, link, category, paid_only, content, content_cn, similar_questions).

#### Scenario: Successful response structure

- **WHEN** system finds matching problems
- **THEN** response is `200` with `{"results": [ProblemDetailResponse, ...]}` and content type `application/json`

### Requirement: Authentication

The system SHALL require Bearer token authentication for this endpoint, consistent with all other `/api/v1/*` routes. Unauthenticated requests SHALL receive `401`.

#### Scenario: Unauthenticated request

- **WHEN** client sends `GET /api/v1/random` without Authorization header
- **THEN** system returns `401` with `{"type":"about:blank","title":"Unauthorized","status":401}`

### Requirement: Parameter validation precedence

The system SHALL validate query parameters in this order: `source` validity, `count` range, `rating_min` <= `rating_max`, then `tag_mode` validity. The first encountered error SHALL be returned as `400`.

#### Scenario: Multiple invalid parameters

- **WHEN** client sends `GET /api/v1/random?source=invalid&count=0`
- **THEN** system returns `400` with detail about the first validation failure (source)
