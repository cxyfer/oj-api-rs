## ADDED Requirements

### Requirement: Daily source problem detail enrichment
After parsing Sheep or 0x3f daily problems, the dedicated daily-source crawler SHALL determine detail-enrichment candidates from the database state that existed before the daily-source snapshot transaction. A problem SHALL be selected when its `(source, id)` row does not exist, when the existing row has both an empty or whitespace-only `title` and an empty or whitespace-only `content`, or when both existing fields still equal the current curated daily snapshot after trimming whitespace. The crawler SHALL commit the existing atomic problem-snapshot and daily-reference transaction before invoking source-specific detail retrieval. Enrichment SHALL run sequentially in parsed order and SHALL remain best-effort: a failed problem retrieval SHALL NOT roll back the committed daily row or prevent later candidates from being attempted.

#### Scenario: Missing problem is enriched after daily storage
- **WHEN** a Sheep or 0x3f parser extracts a supported problem whose `(source, id)` row does not exist
- **THEN** the crawler first commits the problem snapshot and ordered daily references
- **AND** then invokes the existing source-specific detail retrieval for that problem

#### Scenario: Blank title and content trigger enrichment
- **WHEN** a parsed problem already exists with an empty or whitespace-only `title`
- **AND** its `content` is also empty or whitespace-only
- **THEN** the crawler selects that problem for detail enrichment regardless of whether tags are populated

#### Scenario: One populated detail field skips enrichment
- **WHEN** a parsed problem already exists and either `title` or `content` is non-empty after trimming whitespace
- **AND** its stored title and content do not both equal the current curated daily snapshot
- **THEN** the crawler does not invoke detail enrichment for that problem

#### Scenario: Unchanged daily snapshot retries enrichment
- **WHEN** a previous enrichment attempt failed and the stored title and content still equal the current curated daily snapshot after trimming whitespace
- **THEN** a later ingestion selects the problem for detail enrichment again
- **AND** successful source details that differ from the snapshot prevent unnecessary retries on later ingestions

#### Scenario: Failed daily storage prevents enrichment
- **WHEN** the atomic problem-snapshot and daily-reference transaction fails
- **THEN** the crawler does not invoke any source-specific detail retrieval for that ingestion

#### Scenario: Existing single-problem crawlers are reused
- **WHEN** an enrichment candidate has source `codeforces`, `atcoder`, or `luogu`
- **THEN** the crawler invokes that source's existing single-problem retrieval
- **AND** an AtCoder candidate preserves its parsed contest path when the task ID alone is ambiguous

#### Scenario: Source details replace curated snapshot details
- **WHEN** an enrichment candidate was absent before daily snapshot storage, had both blank title and content, or still matched the current curated snapshot
- **AND** its source crawler returns non-empty title or content fields
- **THEN** the returned source title and content replace the corresponding curated snapshot fields supported by that crawler
- **AND** curated rating, difficulty, tags, and other metadata omitted by the source response remain stored

#### Scenario: AtCoder enrichment uses the official task title
- **WHEN** an AtCoder enrichment candidate can be resolved in its contest task list
- **THEN** the stored title and content come from the AtCoder task listing and problem statement
- **AND** they replace non-empty placeholder or summary values written by the daily snapshot

#### Scenario: Existing LeetCode detail path is reused
- **WHEN** an enrichment candidate has source `leetcode`
- **THEN** the crawler invokes the existing LeetCode detail lookup after the snapshot exists
- **AND** selects the LeetCode domain from the parsed problem URL without triggering broad problemset initialization

#### Scenario: Local LeetCode numeric ID becomes the daily key
- **WHEN** a parsed LeetCode URL has slug `two-sum`
- **AND** the local problem table contains `id=1, slug=two-sum`
- **THEN** candidate selection and snapshot storage use problem ID `1`
- **AND** the ordered daily reference is `leetcode:1`
- **AND** enrichment updates `leetcode:1` without creating a new `leetcode:two-sum` row

#### Scenario: LeetCode slug remains the fallback key
- **WHEN** a parsed LeetCode URL has slug `two-sum`
- **AND** no matching local numeric problem ID exists
- **THEN** candidate selection, snapshot storage, and enrichment use problem ID `two-sum`
- **AND** daily storage completes without first fetching remote problemset metadata

#### Scenario: Gym enrichment preserves the daily reference key
- **WHEN** a Codeforces Gym candidate has stored ID `GYM106539D`
- **THEN** enrichment fetches the corresponding Gym problem and updates `codeforces:GYM106539D`
- **AND** does not create a second `codeforces:106539D` problem row
- **AND** does not broaden the ordinary one-argument Codeforces crawler input grammar

#### Scenario: Codeforces placeholder title is replaced
- **WHEN** a Codeforces or Gym enrichment response contains an official title in the problem statement header
- **THEN** the official title without its redundant `<problem_index>.` prefix replaces the stored problem-ID placeholder
- **AND** the problem index remains available in the separate `problem_index` field
- **AND** the statement body remains stored as content without duplicating the header metadata

#### Scenario: Existing Gym placeholder title retries enrichment
- **WHEN** a stored Gym problem already has source content but its title still equals its `GYM...` problem ID
- **THEN** a later daily ingestion selects it for enrichment again
- **AND** stores the official title while preserving the existing Gym key and curated metadata

#### Scenario: Codeforces tags come from contest metadata
- **WHEN** Codeforces single-problem enrichment finds matching contest API metadata for the same problem index
- **AND** that metadata contains non-empty tags
- **THEN** the crawler stores the source tags instead of stored curated tags
- **AND** preserves curated rating and other metadata omitted by the source response

#### Scenario: Missing Codeforces metadata preserves stored tags
- **WHEN** a Codeforces enrichment candidate already has non-empty stored tags
- **AND** matching contest metadata is unavailable or contains an empty tag list
- **THEN** the fetched title and content may still replace snapshot details
- **AND** the stored tags remain unchanged as the fallback

#### Scenario: Existing Codeforces row with empty tags retries enrichment
- **WHEN** a stored Codeforces problem has source title and content but its tags are empty
- **THEN** a later daily ingestion selects it for enrichment again
- **AND** a Codeforces row with non-empty tags and source details is not selected solely for tag enrichment

#### Scenario: Public Gym page contains a sign-in navigation link
- **WHEN** a public Codeforces Gym response contains both a problem statement and a normal `/enter` navigation link
- **THEN** the crawler parses and stores the problem statement
- **AND** does not classify the page as authentication-protected solely because of that link

#### Scenario: LeetCode whitespace values are replaced by details
- **WHEN** a LeetCode enrichment candidate has whitespace-only `title` and `content`, even if its tags are non-empty
- **THEN** LeetCode detail retrieval is performed
- **AND** non-blank returned title and content fields replace the whitespace-only stored values

#### Scenario: Enrichment failure is isolated
- **WHEN** source-specific retrieval fails or raises an error for one candidate
- **THEN** the already committed daily row and problem snapshots remain stored
- **AND** the crawler continues attempting later enrichment candidates in parsed order
