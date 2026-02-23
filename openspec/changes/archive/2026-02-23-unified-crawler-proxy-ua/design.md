## Context

Three Python crawlers (LeetCode/aiohttp, Codeforces/curl_cffi, AtCoder/aiohttp) manage HTTP sessions, headers, and retries independently. None support proxy configuration. User-Agent strings are hardcoded per-crawler. The `config.toml` `[crawler]` section currently only has `timeout_secs`. Rust's serde config uses `#[serde(default)]` without `deny_unknown_fields`, so new TOML fields are safely ignored.

## Goals / Non-Goals

**Goals:**
- Unified proxy (HTTP/HTTPS/SOCKS5) and User-Agent configuration via `config.toml`
- Global defaults with per-crawler overrides (`[crawler.<name>]`)
- Centralized session creation to eliminate scattered HTTP setup
- Zero Rust code changes

**Non-Goals:**
- Proxy rotation or pool management
- Retry/rate-limit unification (each crawler retains its own strategy)
- Crawler CLI interface changes
- Authentication proxy (username/password in URL is supported, but no separate auth config fields)

## Decisions

### D1: Plain base class, not ABC

**Choice:** `BaseCrawler` is a regular Python class (not `ABC`), with no `crawl()` abstract method.

**Rationale:** The three crawlers have completely different public APIs (`fetch_daily_challenge`, `sync_problemset`, `fetch_from_kenkoooo`). Forcing a unified `crawl()` signature would produce empty implementations or unnatural wrappers. The base class's value is config/header/proxy helpers, not interface enforcement.

**Alternatives considered:**
- Full ABC with `crawl()` → rejected (API mismatch, empty impls)
- Mixin → rejected (weaker contract, scattered logic)

### D2: Proxy injection strategy per HTTP library

**aiohttp (LeetCode, AtCoder):**
- HTTP/HTTPS proxy: `session.get(proxy=url)` at request level, via centralized helper
- SOCKS5 proxy: `aiohttp_socks.ProxyConnector.from_url(url)` at session level (connector-based). When ProxyConnector is used, no request-level `proxy=` param.
- All sessions created via `BaseCrawler._create_aiohttp_session()` helper

**curl_cffi (Codeforces):**
- Proxies set at session init: `AsyncSession(proxies={"http": url, "https": url}, trust_env=False)`
- Keys are `"http"`, `"https"` (no `://` suffix). SOCKS5 URLs passed as values (curl_cffi handles natively).
- Session created via `BaseCrawler._create_curl_session()` helper

**Rationale:** aiohttp-socks requires connector-level integration for SOCKS5 (ProxyConnector replaces TCPConnector). curl_cffi handles all proxy types natively via the proxies dict. Centralizing session creation prevents missed injection points (LeetCode has 6+ session creation sites currently).

### D3: Config merge — field-level with binary empty semantics

**Choice:** Field-level merge. Missing fields inherit from global `[crawler]`. Empty strings and missing fields are both treated as None (binary, no tri-state).

**Merge algorithm:**
```
for each field in CrawlerHttpConfig:
    value = normalize(per_crawler.get(field))  # "" → None
    if value is None:
        value = normalize(global_crawler.get(field))
    merged.field = value
```

**Rationale:** Tri-state (missing vs explicit-empty vs value) adds complexity without clear user benefit. Users who want to disable a parent proxy for a specific crawler can set a different proxy or simply not configure the parent.

### D4: Proxy resolution order (5-level)

```
resolve_proxy(scheme: str) → Optional[str]
  scheme must be "http" or "https"

Resolution chain (first non-None wins):
  1. merged.{scheme}_proxy     (e.g., https_proxy)
  2. merged.socks5_proxy       (fallback for any scheme)
  3. merged.proxy              (shorthand for all schemes)
  4. None                      (no proxy)
```

Note: After field-level merge, "merged" already reflects per-crawler > global inheritance. So the effective 4-layer precedence from proposal (per-crawler specific > per-crawler proxy > global specific > global proxy) is handled by the merge step, and resolve_proxy only operates on the merged result.

**socks5_proxy positioning:** Acts as a fallback — scheme-specific (`http_proxy`/`https_proxy`) always wins over `socks5_proxy`. This matches the mental model where SOCKS5 is a "tunnel everything" option that specific overrides can bypass.

### D5: Environment variable isolation

**Choice:** Explicitly disable environment proxy leaking in both libraries.

- aiohttp: `ClientSession(trust_env=False)` — always, even though current default is False
- curl_cffi: `AsyncSession(trust_env=False)` + always pass explicit `proxies` dict

**Rationale:** Prevents silent behavior changes when `HTTP_PROXY`/`HTTPS_PROXY` env vars are set on the host. Config.toml is the single source of truth.

### D6: Codeforces User-Agent handling

**Choice:** Codeforces crawler always ignores `user_agent` config. The `_headers()` method is overridden to never inject User-Agent, preserving `impersonate="chrome124"` TLS fingerprint consistency.

**Rationale:** curl_cffi's `impersonate` generates a complete header set including User-Agent that matches the TLS fingerprint. Injecting a custom UA would create a fingerprint mismatch detectable by Cloudflare.

### D7: Fallback User-Agent

**Choice:** When no `user_agent` is configured (globally or per-crawler), LeetCode and AtCoder use `"Mozilla/5.0 (compatible; OJ-API-Bot/1.0)"` as the default.

**Rationale:** Removes hardcoded per-crawler UA strings while maintaining a sensible default. The previous values (`"Mozilla/5.0"` for LeetCode, `"LeetCodeDailyDiscordBot/1.0"` for AtCoder) were arbitrary and inconsistent.

### D8: SOCKS5 full support via aiohttp-socks

**Choice:** Add `aiohttp-socks` as a runtime dependency in `scripts/pyproject.toml`.

**Rationale:** User chose full SOCKS5 support. aiohttp requires aiohttp-socks for SOCKS5 (ProxyConnector). curl_cffi supports SOCKS5 natively. Making it a hard dependency avoids runtime surprises.

### D9: Proxy URL validation at config load

**Choice:** Validate all proxy URLs at `ConfigManager` initialization (fail-fast). Check: valid scheme (`http`, `https`, `socks5`, `socks5h`), non-empty host.

**Rationale:** Catching malformed URLs early (config load) is better than runtime failures during crawling, which may happen minutes into a long crawl job.

## Risks / Trade-offs

**[Risk] LeetCode has 6+ scattered session creation points** → Mitigation: All session creation funneled through `_create_aiohttp_session()`. Code review must verify zero bare `ClientSession()` calls remain.

**[Risk] aiohttp SOCKS5 requires connector-level proxy (session-scoped), but HTTP proxy uses request-level** → Mitigation: `_create_aiohttp_session()` detects proxy type and returns appropriate session (ProxyConnector for SOCKS5, plain session for HTTP proxy with request-level injection).

**[Risk] curl_cffi proxies dict set at session init cannot be changed per-request** → Mitigation: Codeforces already creates one session per crawl invocation. No rotation needed.

**[Risk] Binary empty semantics prevents per-crawler proxy disabling** → Mitigation: Acceptable trade-off for simplicity. Users can work around by setting a non-functional proxy or restructuring config.

**[Risk] Large refactor touching all three crawlers simultaneously** → Mitigation: Implementation order: ConfigManager → BaseCrawler → AtCoder (smallest) → Codeforces (proxy only) → LeetCode (most complex). Each step independently verifiable.

## Migration Plan

1. Add `aiohttp-socks` to `scripts/pyproject.toml` and lock
2. Implement `CrawlerHttpConfig` + `get_crawler_config()` in `config.py`
3. Create `scripts/utils/base_crawler.py` with session helpers
4. Migrate AtCoder → verify CLI unchanged
5. Migrate Codeforces → verify CLI unchanged, verify impersonate preserved
6. Migrate LeetCode → verify all 6+ session points covered
7. Update `config.toml.example` with commented examples
8. Verify `cargo build --release` still passes (zero Rust changes)

Rollback: Revert the Python changes. No database or config format migration needed — new TOML fields are purely additive and ignored when not present.

## Open Questions

None. All ambiguities resolved during planning phase.
