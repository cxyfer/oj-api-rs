# Design: daily-crawler-fallback

## Architecture Decisions

### D1: Args Whitelist — Grammar-Based Validation

**Decision**: Parse `args` into a `CrawlerAction` enum, reject anything that doesn't match.

```rust
enum CrawlerAction {
    None,                          // [] — backward-compatible
    Daily,                         // ["--daily"]
    Date(String),                  // ["--date", "YYYY-MM-DD"]
    Init,                          // ["--init"]
    Monthly(u16, u8),              // ["--monthly", "YEAR", "MONTH"]
}
```

Validation rules:
- `--date` value must match `^\d{4}-\d{2}-\d{2}$` and be a valid calendar date
- `--monthly` YEAR in `[2000, 2100]`, MONTH in `[1, 12]`
- Any other combination → HTTP 400

**Rationale**: String whitelist comparison is fragile. Parsing into typed enum guarantees no injection and enables reuse in R1 fallback.

### D2: CrawlerJob Model Extension

```rust
#[derive(Debug, Clone, Serialize)]
pub struct CrawlerJob {
    pub job_id: String,
    pub source: String,
    pub args: Vec<String>,          // NEW: CLI args passed
    pub trigger: CrawlerTrigger,    // NEW: Admin | DailyFallback
    pub started_at: String,
    pub finished_at: Option<String>, // NEW
    pub status: CrawlerStatus,
}

#[derive(Debug, Clone, Serialize)]
pub enum CrawlerTrigger {
    Admin,
    DailyFallback,
}
```

### D3: AppState Changes

```rust
pub struct AppState {
    // existing
    pub ro_pool: Pool,
    pub rw_pool: Pool,
    pub config: Config,
    pub token_auth_enabled: AtomicBool,

    // admin crawler — single running job + history
    pub crawler_lock: tokio::sync::Mutex<Option<CrawlerJob>>,
    pub crawler_history: tokio::sync::Mutex<VecDeque<CrawlerJob>>,  // NEW, cap=50

    // daily fallback — keyed by (domain, date), independent from admin
    pub daily_fallback: tokio::sync::Mutex<HashMap<String, DailyFallbackEntry>>,  // NEW
}

struct DailyFallbackEntry {
    status: CrawlerStatus,
    started_at: Instant,
    cooldown_until: Option<Instant>,  // 30s cooldown after failure
}
```

Key: `"com:2024-06-15"` (domain + date string)

### D4: Daily Fallback Flow

```
GET /api/v1/daily?domain=com&date=...
  ├── domain != "com" → existing logic (404 if no data)
  ├── DB has data → 200 + JSON
  └── DB no data
       ├── check daily_fallback[key]
       │    ├── Running → 202 {"status":"fetching","retry_after":30}
       │    ├── cooldown active → 202 {"status":"fetching","retry_after":remaining}
       │    └── None or expired
       │         ├── spawn python3 → success → 202
       │         └── spawn fails → 500
       └── spawn task updates daily_fallback[key] on completion
```

### D5: Crawler Status API Extension

`GET /admin/api/crawlers/status` response changes:

```json
{
  "running": true,
  "current_job": { ... },
  "history": [ ... ]  // last 50 jobs, newest first
}
```

### D6: Scripts Directory Structure

```
scripts/
├── leetcode.py          ← cp from references
├── atcoder.py           ← cp from references
├── codeforces.py        ← cp from references
├── embedding_cli.py     ← cp from references
├── requirements.txt     ← filtered (no discord, apscheduler)
├── utils/
│   ├── __init__.py
│   ├── config.py
│   ├── database.py
│   ├── html_converter.py
│   └── logger.py
└── embeddings/
    ├── __init__.py
    ├── generator.py
    ├── rewriter.py
    ├── searcher.py
    └── storage.py
```

### D7: Crawlers Admin Page Layout

Three sections:
1. **Control Panel** — source selector (3 buttons), action selector (radio: daily/date/init/monthly), date input (shown only when action=date), year/month inputs (shown only when action=monthly), trigger button
2. **Current Status** — card showing running job details, auto-polls every 3s when running
3. **History Table** — columns: Source, Args, Trigger, Started, Finished, Status, Duration

Page renders with initial data from backend (is_running + history). JS handles polling and trigger.

### D8: Rust Path Constants

All Python script invocations use a single constant:

```rust
const SCRIPTS_DIR: &str = "scripts";
```

Updated in: `src/admin/handlers.rs`, `src/api/similar.rs`, `src/api/daily.rs`
