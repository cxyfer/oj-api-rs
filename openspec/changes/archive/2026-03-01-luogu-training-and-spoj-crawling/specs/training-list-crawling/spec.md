# Spec: Training List Crawling

## Requirements

### R1: CLI Interface
- `luogu.py --training-list <value>` accepts either:
  - Training list URL: `https://www.luogu.com.cn/training/378042#problems` or `https://www.luogu.com.cn/training/378042`
  - Training list ID: `378042`
- Python parses URL/ID; Rust passes raw string unchanged (ValueType::Str)

### R2: Data Extraction
- Fetch training list page HTML
- Extract `lentille-context` JSON, path: `data.training.problems[]`
- Each element contains a `problem` sub-object with standard Luogu problem fields

### R3: Filter Rules
- Skip problems where `pid.startswith("AT")` or `pid.startswith("CF")` (case-sensitive on stored pid)
- Log skipped problems at INFO level

### R4: Data Mapping
- Reuse existing `_map_problem()` with `source='luogu'`
- Store metadata only (pid, title, difficulty, tags, ac_rate); do NOT auto-fetch content
- `slug = pid`, `link = https://www.luogu.com.cn/problem/{pid}`

### R5: Rust Arg Whitelist
- Add `--training-list` to `LUOGU_ARGS`: `{ arity: 1, value_type: Str, ui_exposed: true }`

### R6: Admin UI
- Add `--training-list` as text input in luogu tab of CRAWLER_CONFIG
- Add i18n key `crawlers.flags.training_list` in all 3 locale files

## Constraints
- C1: Value type is Str; no URL validation in Rust
- C2: Training list page has no pagination; single request fetches all problems
- C3: lentille-context structure: `data.training.problems[].problem`

## PBT Properties

### P1: Filter Completeness
```
INVARIANT: For any pid in training list, if pid.startsWith("AT") or pid.startsWith("CF"), it is excluded from DB write
FALSIFICATION: Generate pids with AT/CF/P/B/SP/T/U prefixes; verify AT/CF never persisted, others always persisted
CATEGORY: invariant_preservation
```

### P2: Rust Passthrough
```
INVARIANT: Rust validate_args accepts --training-list with any non-empty string, forwards byte-exact value
FALSIFICATION: Fuzz with URLs, numeric IDs, unicode, whitespace-only; verify non-empty accepted, empty rejected
CATEGORY: round_trip
```

### P3: Single Request
```
INVARIANT: Training list fetch makes exactly 1 HTTP request to the training page endpoint
FALSIFICATION: Instrument HTTP client; assert request count == 1 per invocation
CATEGORY: bounds
```

### P4: Idempotent DB Write
```
INVARIANT: Running --training-list twice with same ID produces identical DB state (INSERT OR IGNORE)
FALSIFICATION: Run twice, compare row counts and field values for affected source='luogu' rows
CATEGORY: idempotency
```
