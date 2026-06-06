## MODIFIED Requirements

### Requirement: Daily challenge retrieval
The system SHALL return daily challenges via `GET /api/v1/daily?domain={com|cn}&date={YYYY-MM-DD}` or `GET /api/v1/daily?source={daily-source}&date={YYYY-MM-DD}`. The `domain` parameter SHALL remain LeetCode-only, parsed as a `LeetCodeDomain` enum (`Com`, `Cn`), and mapped to canonical daily sources (`leetcode.com`, `leetcode.cn`). The `source` parameter SHALL accept LeetCode daily sources (`leetcode.com`, `leetcode.cn`) and known additional daily challenge sources (`sheep`, `0x3f`). The response SHALL expose `date`, canonical `source`, and a `problems` array assembled from stored problem references. Each response problem SHALL include problem detail fields and `similar_questions` as a hydrated array of `ProblemSummary` objects resolved from the stored slug list.

#### Scenario: Today's daily (default)
- **WHEN** client sends `GET /api/v1/daily` without parameters
- **THEN** system returns today's (UTC) daily challenge for `leetcode.com` with source defaulting to `leetcode.com`

#### Scenario: Today's CN daily
- **WHEN** client sends `GET /api/v1/daily?domain=cn` without `date` parameter
- **THEN** system returns today's (UTC+8) daily challenge for `leetcode.cn`

#### Scenario: Specific date
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-01-15`
- **THEN** system returns the daily challenge for that specific date from `leetcode.com`

#### Scenario: CN domain
- **WHEN** client sends `GET /api/v1/daily?domain=cn`
- **THEN** system returns the daily challenge from `leetcode.cn`

#### Scenario: Source alias leetcode.cn
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.cn`
- **THEN** system returns the same response as `?domain=cn`

#### Scenario: Source alias leetcode.com
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.com`
- **THEN** system returns the same response as `?domain=com`

#### Scenario: Domain takes precedence over source when equal
- **WHEN** client sends `GET /api/v1/daily?domain=cn&source=leetcode.cn`
- **THEN** system returns the daily challenge from `leetcode.cn` without conflict

#### Scenario: Daily response includes hydrated similar questions
- **WHEN** a resolved daily problem has similar question slugs that exist in the problem table for that problem source
- **THEN** the response problem returns `similar_questions` as hydrated summary objects in the same order as the stored slug list

#### Scenario: Daily response shape
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.com&date=2026-01-01` and a daily record exists
- **THEN** system returns HTTP 200 with a JSON object containing `date`, `source`, and `problems`, and does not return top-level problem fields such as `id`, `slug`, or `title`

#### Scenario: Multiple daily problems preserve order
- **WHEN** the stored daily record has `problems = ["leetcode:1234", "leetcode:1", "atcoder:abc321_a"]`
- **THEN** the response `problems` array returns resolved problems in that same reference order

#### Scenario: Sheep daily source
- **WHEN** client sends `GET /api/v1/daily?source=sheep&date=2026-06-02` and a daily record exists
- **THEN** system returns HTTP 200 with `source = "sheep"` and resolved Codeforces problems

#### Scenario: 0x3f daily source
- **WHEN** client sends `GET /api/v1/daily?source=0x3f&date=2026-06-02` and a daily record exists
- **THEN** system returns HTTP 200 with `source = "0x3f"` and resolved Codeforces problems

### Requirement: Daily challenge domain validation
The system SHALL only accept `com` or `cn` as valid domain values, validated via the `LeetCodeDomain` enum. The `source` parameter SHALL only accept supported daily source values. If both `domain` and `source` are provided, the system SHALL allow matching LeetCode pairs and SHALL return HTTP 400 for conflicts or for pairing a LeetCode-only `domain` with a non-LeetCode `source`.

#### Scenario: Invalid domain
- **WHEN** client sends `GET /api/v1/daily?domain=jp`
- **THEN** system returns HTTP 400 with error detail indicating invalid domain

#### Scenario: Invalid source
- **WHEN** client sends `GET /api/v1/daily?source=leetcode.jp`
- **THEN** system returns HTTP 400 with error detail indicating invalid source value

#### Scenario: Conflicting domain and source
- **WHEN** client sends `GET /api/v1/daily?domain=com&source=leetcode.cn`
- **THEN** system returns HTTP 400 with error detail indicating domain and source conflict

#### Scenario: Domain conflicts with additional daily source
- **WHEN** client sends `GET /api/v1/daily?domain=com&source=sheep`
- **THEN** system returns HTTP 400 with error detail indicating domain and source conflict

### Requirement: Daily challenge not found
The system SHALL return HTTP 404 only when no usable daily challenge record exists in the DB AND no fallback behavior applies. A daily row with malformed JSON, malformed problem refs, or no resolvable problems SHALL be treated as unusable. When fallback behavior applies, the system SHALL return HTTP 202 instead. LeetCode sources SHALL keep spawning the LeetCode fallback crawler. Additional daily sources SHALL return the HTTP 202 fetching response without spawning a crawler from the API handler.

#### Scenario: No data, fallback triggered (com)
- **WHEN** client sends `GET /api/v1/daily?domain=com&date=2024-06-15` and no DB record exists and no fallback is running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` and spawns background crawler

#### Scenario: No data, fallback triggered (cn)
- **WHEN** client sends `GET /api/v1/daily?domain=cn` and no DB record exists for today (UTC+8) and no fallback is running
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}` and spawns background crawler with `--domain cn`

#### Scenario: Malformed stored refs treated as unusable
- **WHEN** the DB has a daily row whose `problems` column is invalid JSON or contains no valid problem references
- **THEN** the system treats the row as unusable and follows the existing no-data fallback behavior

#### Scenario: Missing referenced problems treated as unusable when none resolve
- **WHEN** the DB has a daily row but none of its problem references resolve from the `problems` table
- **THEN** the system treats the row as unusable and follows the existing no-data fallback behavior

#### Scenario: Missing additional daily source does not spawn API fallback crawler
- **WHEN** client sends `GET /api/v1/daily?source=sheep&date=2026-06-02` and no usable DB record exists
- **THEN** system returns HTTP 202 with `{"status": "fetching", "retry_after": 30}`
- **AND** the API handler does not spawn `leetcode.py` or `codeforces.py`

### Requirement: Daily challenge problem localization projection
The system SHALL assemble daily response problems by resolving stored problem refs from the `problems` table and projecting display fields according to the daily challenge source. For `source = "leetcode.cn"`, the response problem `title` SHALL use `title_cn` when present and fallback to `title`, and `content` SHALL use `content_cn` when present and fallback to `content`. For all other daily sources, including `leetcode.com`, `sheep`, and `0x3f`, the response SHALL use default `title` and `content` fields. LeetCode host rewriting SHALL apply only when the resolved problem source is `leetcode` and the daily source is `leetcode.com` or `leetcode.cn`.

#### Scenario: CN daily uses Chinese title and content
- **WHEN** `GET /api/v1/daily?source=leetcode.cn&date=2026-01-01` resolves `leetcode:1` whose problem row has both English and Chinese fields
- **THEN** the returned problem uses `title_cn` as `title` and `content_cn` as `content`

#### Scenario: CN daily falls back when Chinese fields are absent
- **WHEN** `GET /api/v1/daily?source=leetcode.cn&date=2026-01-01` resolves a problem whose `title_cn` or `content_cn` is empty
- **THEN** the returned problem falls back to the corresponding default `title` or `content`

#### Scenario: COM daily uses default fields
- **WHEN** `GET /api/v1/daily?source=leetcode.com&date=2026-01-01` resolves `leetcode:1`
- **THEN** the returned problem uses default `title` and `content`

#### Scenario: LeetCode link host follows daily source
- **WHEN** the daily source is `leetcode.cn` and a resolved problem source is `leetcode`
- **THEN** the returned problem link uses the `https://leetcode.cn` host

#### Scenario: additional daily source preserves Codeforces link
- **WHEN** the daily source is `sheep` and a resolved problem source is `codeforces`
- **THEN** the returned problem link remains the stored Codeforces link
