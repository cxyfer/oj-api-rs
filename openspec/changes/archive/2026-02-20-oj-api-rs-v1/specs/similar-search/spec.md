## ADDED Requirements

### Requirement: Similar search by problem
The system SHALL find similar problems via `GET /api/v1/similar/{source}/{id}?limit={n}&threshold={f}`. It SHALL retrieve the seed problem's embedding from `vec_embeddings`, perform KNN search with `k = min(limit * over_fetch_factor, 200)`, compute `similarity = 1.0 - distance`, filter by threshold and optional source, exclude the seed problem, and truncate to `limit`.

#### Scenario: Successful similar search
- **WHEN** client sends `GET /api/v1/similar/leetcode/1?limit=5&threshold=0.7`
- **THEN** system returns up to 5 problems with similarity >= 0.7, sorted by similarity descending, each including `source`, `id`, `title`, `difficulty`, `link`, and `similarity` score

#### Scenario: Seed problem excluded
- **WHEN** KNN results include the seed problem itself
- **THEN** system excludes it from the response

#### Scenario: No embedding for seed
- **WHEN** client queries `/api/v1/similar/leetcode/999999` and no embedding exists
- **THEN** system returns HTTP 404 with error detail indicating no embedding found

#### Scenario: Default parameters
- **WHEN** client sends `GET /api/v1/similar/leetcode/1` without limit or threshold
- **THEN** system uses `limit=10` and `threshold=0.0`

#### Scenario: Limit exceeds maximum
- **WHEN** client sends `GET /api/v1/similar/leetcode/1?limit=100`
- **THEN** system clamps `limit` to 50

### Requirement: Similar search by text
The system SHALL find similar problems via `GET /api/v1/similar?query={text}&limit={n}&threshold={f}&source={filter}`. It SHALL invoke the Python subprocess `embedding_cli.py --embed-text` to generate an embedding, then perform KNN search.

#### Scenario: Successful text search
- **WHEN** client sends `GET /api/v1/similar?query=binary+search+on+sorted+array&limit=5`
- **THEN** system returns up to 5 similar problems with similarity scores

#### Scenario: Query too short
- **WHEN** client sends `GET /api/v1/similar?query=ab` (less than 3 characters)
- **THEN** system returns HTTP 400 with error detail indicating minimum 3 characters

#### Scenario: Python subprocess timeout
- **WHEN** the Python subprocess does not respond within 30 seconds
- **THEN** system kills the subprocess and returns HTTP 504

#### Scenario: Python subprocess failure
- **WHEN** the Python subprocess exits with non-zero code or outputs invalid JSON
- **THEN** system returns HTTP 502 with a generic error message (no stderr exposure)

### Requirement: Similar search source filtering
The system SHALL support multi-value source filtering via comma-separated `source` parameter.

#### Scenario: Single source filter
- **WHEN** client sends `GET /api/v1/similar/leetcode/1?source=codeforces`
- **THEN** all returned problems have `source = "codeforces"`

#### Scenario: Multiple source filter
- **WHEN** client sends `GET /api/v1/similar/leetcode/1?source=leetcode,codeforces`
- **THEN** all returned problems have source either "leetcode" or "codeforces"

### Requirement: Similar search result invariants
All similar search results SHALL satisfy: similarity in [0.0, 1.0], sorted descending by similarity, count <= limit, all similarities >= threshold.

#### Scenario: Similarity bounds
- **WHEN** any KNN search returns results
- **THEN** every result has `0.0 <= similarity <= 1.0`

#### Scenario: Result ordering
- **WHEN** results contain more than one item
- **THEN** `results[i].similarity >= results[i+1].similarity` for all i

### Requirement: Over-fetch factor configuration
The system SHALL use `over_fetch_factor` (default 4) for KNN `k` calculation: `k = min(limit * over_fetch_factor, 200)`. The factor SHALL be configurable via environment variable.

#### Scenario: Over-fetch capping
- **WHEN** `limit=50` and `over_fetch_factor=4` (k would be 200)
- **THEN** system uses `k=200` (at the cap)

#### Scenario: Large limit capped
- **WHEN** `limit=50` and `over_fetch_factor=5` (k would be 250)
- **THEN** system uses `k=200` (capped)
