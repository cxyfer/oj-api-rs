# luogu-crawler Specification

## Purpose
TBD - created by archiving change luogu-crawler. Update Purpose after archive.
## Requirements
### Requirement: Problem list sync via HTML parsing
The crawler SHALL fetch problem lists from `https://www.luogu.com.cn/problem/list?page=N` and extract embedded JSON from the `<script type="application/json">` tag with `lentille-context` attribute. The JSON path `data.problems.result[]` SHALL provide the problem array and `data.problems.count` SHALL provide the total count. Each page contains 50 problems. The crawler SHALL iterate all pages until `page > ceil(count / 50)`, dynamically updating total pages from each response.

#### Scenario: Successful full sync
- **WHEN** `--sync` is invoked and Luogu has 15400 problems (308 pages)
- **THEN** the crawler SHALL fetch all 308 pages sequentially, extract problems from each, and write them to the DB with `source="luogu"`

#### Scenario: Empty result array on valid page
- **WHEN** a page returns valid HTML with `lentille-context` but `data.problems.result` is an empty array
- **THEN** the crawler SHALL log a warning and stop pagination (no more problems beyond this page)

#### Scenario: Missing lentille-context script tag
- **WHEN** the HTML response does not contain a `lentille-context` script tag
- **THEN** the crawler SHALL treat this as a potential Cloudflare block, trigger retry with backoff, and NOT write progress for this page

### Requirement: Tags mapping with cache and fallback
The crawler SHALL fetch the tag list from `https://www.luogu.com.cn/_lfe/tags` once per sync run, parse the response as `{"tags": [...], "types": [...], "version": ...}`, and build a `{tag_id: tag_name}` mapping from all tags regardless of type. This mapping SHALL be cached in `luogu_tags.json`. Each problem's numeric `tags` array SHALL be converted to a string array using this mapping.

#### Scenario: Successful tag resolution
- **WHEN** a problem has `tags: [185, 42]` and the cached mapping contains `{185: "动态规划", 42: "模拟"}`
- **THEN** the mapped tags SHALL be `["动态规划", "模拟"]`

#### Scenario: Unknown tag ID not in mapping
- **WHEN** a problem has `tags: [185, 99999]` and tag ID 99999 is not in the cached mapping
- **THEN** the mapped tags SHALL be `["动态规划", "99999"]` (raw ID preserved as string)

#### Scenario: Tags API failure with existing cache
- **WHEN** the `/_lfe/tags` API request fails but `luogu_tags.json` contains a cached tags mapping
- **THEN** the crawler SHALL use the cached mapping and log a warning

#### Scenario: Tags API failure without cache
- **WHEN** the `/_lfe/tags` API request fails and no cached tags mapping exists in `luogu_tags.json`
- **THEN** the crawler SHALL proceed with sync, mapping all tag IDs to their string representation

### Requirement: Difficulty conversion
The crawler SHALL convert the numeric `difficulty` field (0-7) to Chinese text using a fixed mapping: 0→暂无评定, 1→入门, 2→普及−, 3→普及/提高−, 4→普及+/提高, 5→提高+/省选−, 6→省选/NOI−, 7→NOI/NOI+/CTSC. Any value outside 0-7 (including null, negative, or >7) SHALL map to `None`.

#### Scenario: Known difficulty value
- **WHEN** a problem has `difficulty: 5`
- **THEN** the DB `difficulty` field SHALL be `"提高+/省选−"`

#### Scenario: Unknown difficulty value
- **WHEN** a problem has `difficulty: 8` or `difficulty: -1`
- **THEN** the DB `difficulty` field SHALL be `NULL`

#### Scenario: Null difficulty
- **WHEN** a problem has `difficulty: null`
- **THEN** the DB `difficulty` field SHALL be `NULL`

### Requirement: DB field mapping
The crawler SHALL write to the `problems` table with `source = "luogu"`. Field mapping: `pid` → `id` and `slug`, `title` → `title` and `title_cn`, converted difficulty → `difficulty`, `100 * totalAccepted/totalSubmit` → `ac_rate` (percentage 0–100), converted tags → `tags` (as `json.dumps()` string), `https://www.luogu.com.cn/problem/{pid}` → `link`, `"Algorithms"` → `category`, `0` → `paid_only`. Fields `rating`, `contest`, `problem_index`, `content` SHALL be `NULL`.

#### Scenario: Standard problem mapping
- **WHEN** a Luogu problem has `pid="P1000"`, `title="A+B Problem"`, `difficulty=1`, `totalAccepted=100`, `totalSubmit=200`, `tags=[185]`
- **THEN** the DB record SHALL have `id="P1000"`, `source="luogu"`, `slug="P1000"`, `title="A+B Problem"`, `title_cn="A+B Problem"`, `difficulty="入门"`, `ac_rate=50`, `tags='["动态规划"]'`, `link="https://www.luogu.com.cn/problem/P1000"`, `category="Algorithms"`, `paid_only=0`

#### Scenario: Zero submissions
- **WHEN** a problem has `totalSubmit=0`
- **THEN** `ac_rate` SHALL be `NULL`

#### Scenario: Missing pid
- **WHEN** a problem record lacks a `pid` field
- **THEN** the crawler SHALL skip this problem and log a warning

### Requirement: Batch DB insert with idempotency
The crawler SHALL use `ProblemsDatabaseManager.update_problems()` (INSERT OR IGNORE) per page. The `tags` field MUST be pre-serialized via `json.dumps()` before passing to `update_problems()`. Repeated sync runs over the same data SHALL NOT create duplicate rows or modify existing rows.

#### Scenario: Idempotent re-sync
- **WHEN** `--sync` is run twice with identical upstream data
- **THEN** the second run SHALL insert 0 new rows and the DB state SHALL be identical

#### Scenario: Tags serialization
- **WHEN** tags are mapped to `["动态规划", "模拟"]`
- **THEN** the value passed to `update_problems()` for the `tags` field SHALL be the string `'["动态规划", "模拟"]'`

### Requirement: Rate limiting with minimum floor
The crawler SHALL enforce a minimum interval of 1.0 seconds between consecutive HTTP requests to Luogu domains. The `__init__` method SHALL clamp the rate_limit parameter: `self.rate_limit = max(rate_limit, 1.0)`. A `_throttle()` method SHALL use `time.monotonic()` to track and enforce this interval.

#### Scenario: Rate limit below minimum
- **WHEN** `--rate-limit 0.5` is passed
- **THEN** the effective rate limit SHALL be 1.0 seconds

#### Scenario: Consecutive requests
- **WHEN** two pages are fetched sequentially
- **THEN** the elapsed time between the two HTTP requests SHALL be >= 1.0 seconds

### Requirement: Cloudflare challenge detection
The crawler SHALL implement `_is_rate_limited(html)` that returns `True` when the response matches Cloudflare challenge signatures: title containing "just a moment..." or "attention required! | cloudflare", or body containing markers "too many requests", "captcha", "cloudflare". Additionally, if HTTP status is 200 but the `lentille-context` script tag is absent, this SHALL be treated as a suspected block. Detection SHALL trigger exponential backoff (base=2.0, max=60.0) and SHALL NOT write progress.

#### Scenario: Cloudflare challenge page
- **WHEN** the response HTML contains `<title>Just a moment...</title>`
- **THEN** `_is_rate_limited()` SHALL return `True`

#### Scenario: Normal page with lentille-context
- **WHEN** the response HTML contains a valid `lentille-context` script tag
- **THEN** `_is_rate_limited()` SHALL return `False`

#### Scenario: HTTP 200 without lentille-context
- **WHEN** the response status is 200 but HTML lacks `lentille-context` script tag
- **THEN** the crawler SHALL treat this as suspected block and trigger retry with backoff

### Requirement: Progress tracking with resume
The crawler SHALL maintain `data/luogu_progress.json` with fields: `completed_pages` (array of page number strings), `last_completed_page` (int), `total_count_snapshot` (int), `tags_map` (object), `last_updated` (ISO8601 UTC). Progress SHALL be written atomically (tmp file → fsync → rename). A page SHALL be added to `completed_pages` only after its DB commit succeeds. `--sync` SHALL skip pages already in `completed_pages`.

#### Scenario: Resume after interruption
- **WHEN** sync was interrupted at page 150 and `completed_pages` contains pages 1-149
- **THEN** re-running `--sync` SHALL start from page 150 (first page not in completed_pages)

#### Scenario: Atomic write on crash
- **WHEN** the process crashes during progress file write
- **THEN** the progress file SHALL contain either the previous valid state or the new valid state, never a partial/corrupt JSON

#### Scenario: completed_pages monotonicity
- **WHEN** multiple sync runs are performed
- **THEN** `completed_pages` SHALL only grow (set inclusion: S_next ⊇ S_prev)

### Requirement: CLI interface
The crawler SHALL provide the following CLI arguments via argparse: `--sync` (sync all problem pages), `--fill-missing-content` (fetch problem content for all problems with NULL content), `--missing-content-stats` (show count of problems missing content), `--status` (show progress), `--overwrite` (overwrite existing problems instead of skipping), `--rate-limit <float>` (request interval, default 1.0, min 1.0), `--batch-size <int>` (DB write batch size for content sync, default 10), `--data-dir <str>` (data directory), `--db-path <str>` (database path). If no action flag is provided, the crawler SHALL print help and exit.

#### Scenario: Sync invocation
- **WHEN** `python3 luogu.py --sync` is executed
- **THEN** the crawler SHALL fetch tags, then iterate all pages writing problems to DB

#### Scenario: Fill missing content invocation
- **WHEN** `python3 luogu.py --fill-missing-content` is executed
- **THEN** the crawler SHALL query DB for problems with NULL content, fetch each problem's detail page, compose markdown, and update the content field

#### Scenario: Missing content stats
- **WHEN** `python3 luogu.py --missing-content-stats` is executed
- **THEN** the crawler SHALL display the count of luogu problems with NULL content and total luogu problem count

#### Scenario: Status display
- **WHEN** `python3 luogu.py --status` is executed
- **THEN** the crawler SHALL display: completed pages count, last completed page, total count snapshot, last updated timestamp, and DB problem count for source="luogu"

#### Scenario: No action flag
- **WHEN** `python3 luogu.py` is executed without any flags
- **THEN** the crawler SHALL print argparse help text and exit

### Requirement: Problem content fetching from detail page
The crawler SHALL fetch individual problem pages from `https://www.luogu.com.cn/problem/{pid}` and extract the `lentille-context` JSON. From `data.problem.content`, it SHALL read the fields `background`, `description`, `formatI`, `formatO`, `hint` (all Markdown strings). From `data.problem.samples`, it SHALL read the sample test cases as a 2D array `[["input", "output"], ...]`.

#### Scenario: Successful content fetch
- **WHEN** `fetch_problem_content("P1001")` is called and the page returns valid lentille-context
- **THEN** it SHALL return a Markdown string containing the composed sections

#### Scenario: Missing lentille-context on detail page
- **WHEN** the detail page HTML does not contain a `lentille-context` script tag
- **THEN** the crawler SHALL treat this as a potential Cloudflare block, trigger retry with backoff, and return None after max retries

#### Scenario: Problem with no content object
- **WHEN** `data.problem.content` is null or missing in the lentille-context JSON
- **THEN** the crawler SHALL log a warning, skip this problem, and NOT update the DB content field

### Requirement: Content markdown composition
The crawler SHALL compose a single Markdown string from the content fields using Chinese section headers matching Luogu's official UI. Sections SHALL be ordered: `## 题目背景` (background, omitted if empty), `## 题目描述` (description), `## 输入格式` (formatI), `## 输出格式` (formatO), `## 样例` (samples formatted as numbered input/output code blocks), `## 说明/提示` (hint, omitted if empty). Each section's body is the raw Markdown from Luogu (no HTML conversion needed). Sections SHALL be separated by double newlines.

#### Scenario: Full content with all sections
- **WHEN** a problem has background="bg text", description="desc text", formatI="input fmt", formatO="output fmt", samples=[["1 2", "3"]], hint="hint text"
- **THEN** the composed markdown SHALL be:
  ```
  ## 题目背景

  bg text

  ## 题目描述

  desc text

  ## 输入格式

  input fmt

  ## 输出格式

  output fmt

  ## 样例

  ### 样例输入 #1

  ```
  1 2
  ```

  ### 样例输出 #1

  ```
  3
  ```

  ## 说明/提示

  hint text
  ```

#### Scenario: No background and no hint
- **WHEN** a problem has background="" and hint=""
- **THEN** the composed markdown SHALL omit the `## 题目背景` and `## 说明/提示` sections entirely

#### Scenario: Multiple samples
- **WHEN** a problem has samples=[["1", "2"], ["3", "4"]]
- **THEN** the samples section SHALL contain `### 样例输入 #1`, `### 样例输出 #1`, `### 样例输入 #2`, `### 样例输出 #2`

#### Scenario: Empty samples array
- **WHEN** a problem has samples=[]
- **THEN** the `## 样例` section SHALL be omitted

### Requirement: Content sync with DB-driven resume
The `--sync-content` command SHALL query the DB via `get_problem_ids_missing_content(source="luogu")` which filters `source='luogu' AND (content IS NULL OR content = '') AND category='Algorithms' AND paid_only=0`, then fetch each problem's detail page sequentially. Resume is purely DB-driven: no `content_completed_pids` in progress JSON. Each subsequent run re-queries the DB, and problems with non-NULL non-empty content are automatically excluded.

#### Scenario: Initial sync-content run
- **WHEN** `--sync-content` is run for the first time with 15000 problems missing content
- **THEN** the crawler SHALL fetch all 15000 problem detail pages sequentially, updating content in DB after each batch of 10

#### Scenario: Resume after interruption
- **WHEN** sync-content was interrupted after completing 5000 problems (their content is now non-NULL in DB)
- **THEN** re-running `--sync-content` SHALL re-query DB, find only the remaining ~10000 problems with NULL content, and continue from there

#### Scenario: All content already fetched
- **WHEN** `--sync-content` is run but all luogu problems already have non-NULL content
- **THEN** the crawler SHALL log "No problems with missing content" and exit

#### Scenario: All content fields empty (special problem type)
- **WHEN** `_compose_content_markdown` returns `""` (all sections empty)
- **THEN** the crawler SHALL NOT update the DB content field (keep NULL), allowing retry on next run

### Requirement: Content DB update via batch_update_content
The crawler SHALL use `ProblemsDatabaseManager.batch_update_content()` to write the composed Markdown to the `content` column with `batch_size=10`. Updates SHALL use `UPDATE problems SET content = ? WHERE source = ? AND id = ?`. Only non-empty composed markdown SHALL be included in the batch.

#### Scenario: Successful content update
- **WHEN** content is fetched for P1001 and composed to non-empty markdown
- **THEN** the DB record for `source='luogu', id='P1001'` SHALL have its `content` field updated to the composed markdown string

#### Scenario: Content update does not overwrite other fields
- **WHEN** `batch_update_content()` is called
- **THEN** only the `content` column SHALL be modified; all other columns (title, tags, difficulty, etc.) SHALL remain unchanged

