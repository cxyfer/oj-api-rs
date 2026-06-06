## Context

`daily_challenge` already stores compact rows as `(date, source, problems)`, where `problems` is an ordered JSON array of `{problem_source}:{problem_id}` refs. The Rust API currently constrains daily source selection to LeetCode domains and always uses the LeetCode crawler for fallback. The Codeforces crawler already owns Codeforces problem metadata and can be extended with daily-source ingestion without a schema migration.

Sheep's `Daily_CF_Problems` repository exposes predictable raw Markdown paths under `daily_problems/{YYYY}/{MM}/{MMDD}/problems.md`. 0x3f's Tencent Docs spreadsheet is not a stable anonymous machine API, so automated support should require a stable downloaded/exported local tabular file rather than scraping Tencent Docs UI or private endpoints.

## Goals / Non-Goals

**Goals:**
- Expose `sheep` and `0x3f` through `GET /api/v1/daily?source=...` when rows exist.
- Add crawler CLI flags for the new daily challenge sources.
- Parse Sheep Markdown daily tables and write Codeforces problem rows plus compact daily refs.
- Parse 0x3f stable local CSV/TSV-style exports and write compact daily refs.
- Preserve all LeetCode daily domain, localization, link rewriting, fallback, and wait behavior.

**Non-Goals:**
- No anonymous Tencent Docs UI scraping, private `dop-api` scraping, cookie-based access, or Tencent Open API OAuth flow in this change.
- No database schema migration.
- No automatic fallback for the new sources from the Rust daily endpoint until source-specific crawler fallback can be made explicit and safe.
- No guarantee that Gym problems can be enriched beyond metadata present in the external daily sources.

## Decisions

1. **Use existing compact daily storage**
   - Store daily source rows with `source = "sheep"` or `"0x3f"` and refs like `"codeforces:1930A"`.
   - Rationale: the schema and Rust resolver already support arbitrary problem sources and ordered refs.
   - Alternative considered: create dedicated daily-source tables. Rejected because it adds migration cost without a requirement.

2. **Generalize API source selection without generalizing LeetCode domains**
   - Keep `domain={com|cn}` LeetCode-only.
   - Allow `source` to select either LeetCode or known additional daily challenge sources.
   - For non-LeetCode sources, compute default today with UTC and skip LeetCode localization/link rewriting.
   - Alternative considered: introduce a broad `DailySource` enum across models. Rejected for now because source-specific behavior is limited to `daily.rs`.

3. **Only LeetCode sources trigger Rust fallback crawler**
   - If an additional daily challenge source row is missing, return the existing `202 fetching` shape without spawning a crawler from the API path.
   - Rationale: Sheep/0x3f crawler inputs differ, 0x3f may require a local file, and API-triggered crawlers must not rely on unavailable parameters.
   - Alternative considered: spawn `codeforces.py --daily-source sheep`. Deferred to avoid silently treating 0x3f and Sheep differently in runtime fallback.

4. **Extend `codeforces.py` rather than adding a new script**
   - Add `--daily-source {sheep,0x3f}`, `--date`, and `--daily-file` to the Codeforces crawler.
   - Rationale: it reuses `ProblemsDatabaseManager`, Codeforces logging/progress conventions, and the existing source whitelist.
   - Alternative considered: new `codeforces_daily.py`. Rejected to keep admin/source registration surface minimal.

5. **Parse external source metadata into minimal problem rows**
   - Problem IDs follow existing regular Codeforces format `{contestId}{index}` and Gym format `GYM{gymId}{index}`.
   - `slug` equals `id`; `title` uses Markdown link text when available; `rating` comes from Sheep difficulty or 0x3f rating-like columns; `content` may use hints/notes.
   - Rationale: daily responses must resolve refs even before full Codeforces problemset enrichment exists.

## Risks / Trade-offs

- **Sheep Markdown is human-maintained** → Keep parser tolerant of table spacing and multiple links per row; treat 404/missing files as no-data instead of corrupting rows.
- **0x3f export columns are not verified from anonymous access** → Use alias-based header detection and fail clearly when date or problem URLs cannot be found.
- **API returns 202 for missing additional daily challenge source rows without spawning** → Clients see the same non-terminal shape as existing fallback; operationally, daily source ingestion must be run by crawler CLI.
- **Gym metadata may be sparse** → Store link/title/rating/hint from curated source so responses remain useful.
