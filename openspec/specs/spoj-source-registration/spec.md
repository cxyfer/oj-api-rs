# Spec: SPOJ Source Registration & Crawling

## Requirements

### R1: CrawlerSource::Spoj
- New enum variant `CrawlerSource::Spoj`
- `parse("spoj")` returns `Ok(Self::Spoj)`
- `script_name()` returns `"luogu.py"` (shared script)
- `arg_specs()` returns `SPOJ_ARGS`

### R2: SPOJ_ARGS Whitelist
| Flag | Arity | ValueType | ui_exposed |
|---|---|---|---|
| `--sync-spoj` | 0 | None | true |
| `--rate-limit` | 1 | Float | true |
| `--batch-size` | 1 | Int | true |
| `--data-dir` | 1 | Str | false |
| `--db-path` | 1 | Str | false |

### R3: detect.rs Routing
- `SP\d+` bare ID input: return `("spoj", "SP{n}")`
- Luogu URL `luogu.com.cn/problem/SP{n}`: return `("spoj", "SP{n}")`
- Extract SP\d+ pattern from `LUOGU_ID_RE` into a separate check that runs first

### R4: API VALID_SOURCES
- Add `"spoj"` to `VALID_SOURCES` in `src/api/problems.rs`

### R5: Python --sync-spoj
- Crawl `https://www.luogu.com.cn/problem/list?type=SP&page={n}`, 50/page
- DB storage: `source='spoj'`, `id='SP{n}'` (preserve Luogu numbering)
- Title format `"CODE - Name"` → `slug=CODE`, `title=Name`
- No `" - "` separator → `slug=id`
- `link = https://www.spoj.com/problems/{slug}/`
- Difficulty: reuse Luogu 0-7 mapping (DIFFICULTY_MAP)
- Progress: `spoj_progress.json` (separate from `luogu_progress.json`)

### R6: Python --source Parameter
- New `--source` argument for luogu.py (type=str, choices: luogu, spoj)
- `--fill-missing-content --source spoj` fetches content for `source='spoj'` rows
- Default (no --source): targets `source='luogu'` (backward compatible)
- Rust whitelist: add `--source` to `LUOGU_ARGS` with `ValueType::Str`

### R7: Admin UI
- Add `spoj` tab in CRAWLER_CONFIG with `--sync-spoj` checkbox
- Add `spoj` to source button groups in templates (crawlers.html, problems.html, embeddings.html)
- Add i18n keys: `sources.spoj`, `crawlers.flags.sync_spoj`
- Add `--source` as select input in luogu tab (options: luogu, spoj)
- Add i18n key: `crawlers.flags.source`
- Add SPOJ difficulty labels to i18n (reuse luogu_0 through luogu_7 pattern or add spoj-specific)

## Constraints
- C6: DB PK = `(source='spoj', id='SP1')`
- C7: slug from title `" - "` split; fallback slug=id
- C8: Pagination same as sync(): `data.problems.count`, 50/page
- C9: Progress uses `spoj_progress.json`
- C10: `--fill-missing-content --source spoj` from luogu tab
- C12: detect.rs SP\d+ → spoj (both URL and bare ID)
- C14: SPOJ difficulty reuses Luogu 0-7

## PBT Properties

### P1: detect.rs Routing Consistency
```
INVARIANT: detect_source("SP{n}") == detect_source("https://www.luogu.com.cn/problem/SP{n}") == ("spoj", "SP{N}")
FALSIFICATION: Generate SP\d+ strings, compare bare ID vs URL detection results; any mismatch falsifies
CATEGORY: invariant_preservation
```

### P2: Slug Derivation Determinism
```
INVARIANT: For title containing " - ", slug = title.split(" - ")[0]; otherwise slug = id. Same input always produces same output.
FALSIFICATION: Fuzz titles with 0/1/many " - " separators, leading/trailing spaces, unicode dashes
CATEGORY: round_trip
```

### P3: Source Isolation
```
INVARIANT: All rows inserted by --sync-spoj have source='spoj'; never 'luogu'
FALSIFICATION: After sync-spoj, query DB for any row where id matches SP\d+ AND source != 'spoj'
CATEGORY: invariant_preservation
```

### P4: Progress File Isolation
```
INVARIANT: spoj_progress.json mutations never modify luogu_progress.json content or mtime, and vice versa
FALSIFICATION: Run both syncs, monitor file checksums; any cross-contamination falsifies
CATEGORY: invariant_preservation
```

### P5: --source Targeting
```
INVARIANT: --fill-missing-content --source spoj only queries and updates rows where source='spoj'
FALSIFICATION: Seed DB with mixed sources missing content; run with --source spoj; verify only spoj rows updated
CATEGORY: invariant_preservation
```

### P6: Sync Idempotency
```
INVARIANT: Running --sync-spoj twice produces identical DB state
FALSIFICATION: Run twice, diff DB snapshots; any difference falsifies
CATEGORY: idempotency
```

### P7: Argument Validation Completeness
```
INVARIANT: SPOJ_ARGS whitelist accepts exactly {--sync-spoj, --rate-limit, --batch-size, --data-dir, --db-path}; rejects all others
FALSIFICATION: Generate all possible --flag combinations; validate_args must accept iff flag in whitelist
CATEGORY: bounds
```

### P8: LUOGU_ARGS Extended Validation
```
INVARIANT: LUOGU_ARGS accepts --training-list (Str) and --source (Str) in addition to existing flags; argument order does not affect validation result
FALSIFICATION: Permute all valid arg combinations; any order-dependent acceptance/rejection falsifies
CATEGORY: commutativity
```
