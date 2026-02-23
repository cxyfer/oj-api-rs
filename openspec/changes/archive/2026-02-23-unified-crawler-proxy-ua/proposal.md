# Proposal: Unified Crawler Proxy & User-Agent Configuration

## Context

All three Python crawlers (LeetCode, Codeforces, AtCoder) lack proxy support and hardcode User-Agent strings independently. There is no shared base class — each crawler manages HTTP sessions, headers, retries, and rate limiting in isolation. The `config.toml` already has a `[crawler]` section (with only `timeout_secs`), and Rust's serde config parsing uses `#[serde(default)]` (no `deny_unknown_fields`), so adding new fields is safe.

### Current State

| Crawler | HTTP Library | User-Agent | Proxy |
|---------|-------------|------------|-------|
| LeetCode | aiohttp | Hardcoded `"Mozilla/5.0"` | None |
| AtCoder | aiohttp | Hardcoded `"LeetCodeDailyDiscordBot/1.0"` | None |
| Codeforces | curl_cffi | `impersonate="chrome124"` (auto) | None |

### User Decisions (from clarification)

- Codeforces: inject proxy only, skip user_agent (impersonate handles it)
- Architecture: full ABC base class with enforced interface
- Config granularity: global defaults + per-crawler overrides + multi-protocol proxy support

## Requirements

### R1: config.toml — Crawler HTTP Settings

Extend `[crawler]` section with proxy and user_agent fields. Support per-crawler overrides via `[crawler.<name>]` sub-sections.

```toml
[crawler]
timeout_secs = 300
user_agent = "Mozilla/5.0 (compatible; OJ-API-Bot/1.0)"
# proxy = "http://127.0.0.1:7890"         # single proxy shorthand
http_proxy = ""                            # HTTP-specific proxy
https_proxy = ""                           # HTTPS-specific proxy
socks5_proxy = ""                          # SOCKS5 proxy

# Per-crawler overrides (all fields optional, inherit from [crawler])
# [crawler.leetcode]
# user_agent = "..."
# https_proxy = "..."

# [crawler.codeforces]
# https_proxy = "..."
# NOTE: user_agent is ignored for codeforces (impersonate handles it)

# [crawler.atcoder]
# user_agent = "..."
# https_proxy = "..."
```

**Scenario**: When `[crawler.leetcode]` defines `https_proxy` but not `user_agent`, LeetCode crawler uses its own `https_proxy` and falls back to `[crawler].user_agent`.

**Constraint**: `proxy` field is a shorthand — if set, it applies to all protocols unless overridden by specific `http_proxy`/`https_proxy`/`socks5_proxy`. Resolution order: per-crawler specific > per-crawler `proxy` > global specific > global `proxy`.

### R2: ConfigManager — Crawler Config Accessor

Add a `get_crawler_config(crawler_name: str) -> CrawlerHttpConfig` method to `ConfigManager` that merges global `[crawler]` defaults with per-crawler `[crawler.<name>]` overrides.

```python
@dataclass
class CrawlerHttpConfig:
    user_agent: Optional[str] = None
    proxy: Optional[str] = None        # shorthand
    http_proxy: Optional[str] = None
    https_proxy: Optional[str] = None
    socks5_proxy: Optional[str] = None

    def resolve_proxy(self, scheme: str = "https") -> Optional[str]:
        """Resolve effective proxy for a given scheme."""
        ...
```

**Scenario**: `get_crawler_config("leetcode")` merges `[crawler]` + `[crawler.leetcode]`, returning a `CrawlerHttpConfig` with resolved values.

**Constraint**: Must not break existing `ConfigManager` API. The `get_crawler_config` method is additive.

### R3: Abstract Base Class — BaseCrawler

Create `scripts/utils/base_crawler.py` with `BaseCrawler(ABC)`:

- Reads `CrawlerHttpConfig` from config on init
- Provides `_headers(referer=None) -> dict` (injects user_agent)
- Provides `_get_proxy_kwargs() -> dict` (returns library-appropriate proxy args)
- Declares abstract method `async def crawl(self, **kwargs) -> Any`
- Subclasses override `_headers()` / `_get_proxy_kwargs()` when needed (e.g., Codeforces skips UA injection)

**Constraint**: BaseCrawler must NOT dictate which HTTP library to use. Each subclass retains its own session management (aiohttp / curl_cffi). The base class only provides config access and header/proxy helpers.

**Constraint**: Codeforces subclass must override `_headers()` to NOT inject user_agent (preserving impersonate behavior). It only uses proxy config.

### R4: Migrate Existing Crawlers

Refactor `LeetCodeClient`, `CodeforcesClient`, `AtCoderClient` to inherit from `BaseCrawler`:

- Replace hardcoded User-Agent strings with `self._headers()` calls
- Inject proxy into session creation:
  - aiohttp: `aiohttp.ClientSession(...)` with `proxy=` param on `.get()`/`.post()`
  - curl_cffi: `AsyncSession(impersonate=..., proxies=...)` with proxies dict
- Remove module-level `USER_AGENT` constants

**Constraint**: All existing CLI arguments, entry points (`main()`), and Rust invocation patterns (`uv run python3 <script>.py`) must remain unchanged.

**Constraint**: When no proxy/user_agent is configured, behavior must be identical to current (no regression).

### R5: Rust Side — No Changes Required

Rust's `CrawlerConfig` struct only reads `timeout_secs`. New TOML fields (`user_agent`, `proxy`, `http_proxy`, etc.) and sub-tables (`[crawler.leetcode]`) are silently ignored by serde due to `#[serde(default)]` without `deny_unknown_fields`.

**Constraint**: Zero Rust code changes. Verified by existing serde behavior.

### R6: config.toml.example Update

Add commented-out examples for all new fields in `config.toml.example` under `[crawler]`.

## Success Criteria

1. `cargo build --release` succeeds without changes to Rust code
2. Each crawler runs with empty `[crawler]` section (no proxy/UA configured) — behavior identical to current
3. Setting `[crawler] proxy = "http://127.0.0.1:7890"` routes all crawler HTTP traffic through that proxy
4. Setting `[crawler.leetcode] https_proxy = "..."` overrides only LeetCode's HTTPS proxy
5. Setting `[crawler] user_agent = "..."` changes UA for LeetCode and AtCoder but NOT Codeforces
6. `BaseCrawler` ABC prevents instantiation without implementing `crawl()`
7. All three crawlers pass existing CLI invocation patterns from Rust (`uv run python3 <script>.py --daily` etc.)

## Dependencies & Risks

- **aiohttp proxy**: aiohttp supports `proxy=` param on request methods natively. No new deps needed.
- **curl_cffi proxy**: `AsyncSession(proxies={"https": "..."})` supported natively. No new deps.
- **SOCKS5**: Both aiohttp (via `aiohttp-socks`) and curl_cffi support SOCKS5. If SOCKS5 is configured, `aiohttp-socks` must be added to `pyproject.toml` dependencies.
- **Risk**: Large refactor touching all three crawlers simultaneously. Mitigate by implementing BaseCrawler first, then migrating one crawler at a time with verification.
