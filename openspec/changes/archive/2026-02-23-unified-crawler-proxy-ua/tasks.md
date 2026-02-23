## 1. Dependencies & Config Layer

- [x] 1.1 Add `aiohttp-socks` to `scripts/pyproject.toml` dependencies and run `uv lock`
- [x] 1.2 Implement `CrawlerHttpConfig` frozen dataclass in `scripts/utils/config.py` with fields: `user_agent`, `proxy`, `http_proxy`, `https_proxy`, `socks5_proxy` (all `Optional[str]`), and `resolve_proxy(scheme: str) -> Optional[str]` method. Resolution chain: scheme-specific → socks5_proxy → proxy → None. Reject invalid scheme with `ValueError`.
- [x] 1.3 Implement proxy URL validation helper in `scripts/utils/config.py`: validate scheme (`http`, `https`, `socks5`, `socks5h`), non-empty host. Called at config load time (fail-fast).
- [x] 1.4 Implement `get_crawler_config(crawler_name: str) -> CrawlerHttpConfig` in `ConfigManager`: field-level merge of `[crawler]` + `[crawler.<name>]`, normalize empty strings to `None`, validate proxy URLs, return frozen dataclass.
- [x] 1.5 Verify `cargo build --release` passes with new TOML fields present (zero Rust changes).

## 2. BaseCrawler

- [x] 2.1 Create `scripts/utils/base_crawler.py` with `BaseCrawler` plain base class. Constructor accepts `crawler_name: str`, reads `CrawlerHttpConfig` via `ConfigManager.get_crawler_config()`.
- [x] 2.2 Implement `_headers(referer: Optional[str] = None) -> dict`: inject `User-Agent` from config or fallback `"Mozilla/5.0 (compatible; OJ-API-Bot/1.0)"`. Include `Referer` if provided.
- [x] 2.3 Implement `_create_aiohttp_session(**kwargs) -> aiohttp.ClientSession`: `trust_env=False` always. SOCKS5 → `ProxyConnector.from_url()` as connector. HTTP/HTTPS → plain session (proxy per-request). Merge caller kwargs.
- [x] 2.4 Implement `_get_aiohttp_request_proxy(scheme: str) -> Optional[str]`: return resolved proxy for non-SOCKS5; return `None` for SOCKS5 (handled at connector level).
- [x] 2.5 Implement `_create_curl_session(**kwargs) -> AsyncSession`: `trust_env=False` always. Pass `proxies={"http": url, "https": url}` when configured. Preserve caller kwargs (`impersonate`, etc.).

## 3. Migrate AtCoder (smallest surface)

- [x] 3.1 Make `AtCoderClient` inherit `BaseCrawler` with `crawler_name="atcoder"`.
- [x] 3.2 Replace all `aiohttp.ClientSession()` calls with `self._create_aiohttp_session()`.
- [x] 3.3 Replace hardcoded `"LeetCodeDailyDiscordBot/1.0"` UA with `self._headers()`.
- [x] 3.4 Inject `proxy=self._get_aiohttp_request_proxy("https")` on all request calls.
- [x] 3.5 Verify `uv run python3 atcoder.py --sync` CLI invocation works unchanged.

## 4. Migrate Codeforces (proxy only)

- [x] 4.1 Make `CodeforcesClient` inherit `BaseCrawler` with `crawler_name="codeforces"`.
- [x] 4.2 Override `_headers()` to never inject `User-Agent` (preserve impersonate fingerprint).
- [x] 4.3 Replace `AsyncSession(impersonate="chrome124")` with `self._create_curl_session(impersonate="chrome124")`.
- [x] 4.4 Verify `uv run python3 codeforces.py --sync` CLI invocation works unchanged.

## 5. Migrate LeetCode (most complex)

- [x] 5.1 Make `LeetCodeClient` inherit `BaseCrawler` with `crawler_name="leetcode"`.
- [x] 5.2 Audit and replace all `aiohttp.ClientSession()` creation points (6+ sites) with `self._create_aiohttp_session()`.
- [x] 5.3 Replace all hardcoded `"Mozilla/5.0"` UA with `self._headers()`.
- [x] 5.4 Inject `proxy=self._get_aiohttp_request_proxy("https")` on all `.get()`/`.post()` calls.
- [x] 5.5 Remove module-level `USER_AGENT` constant if present.
- [x] 5.6 Verify zero bare `aiohttp.ClientSession()` calls remain in source.
- [x] 5.7 Verify `uv run python3 leetcode.py --daily` CLI invocation works unchanged.

## 6. Config Example & Cleanup

- [x] 6.1 Update `config.toml.example`: add commented examples for `user_agent`, `proxy`, `http_proxy`, `https_proxy`, `socks5_proxy` under `[crawler]`.
- [x] 6.2 Add commented `[crawler.leetcode]`, `[crawler.codeforces]`, `[crawler.atcoder]` override sections to `config.toml.example` with note that Codeforces ignores `user_agent`.
- [x] 6.3 Remove all hardcoded UA string literals from crawler files (grep verify: no `"Mozilla/5.0"` or `"LeetCodeDailyDiscordBot/1.0"` in header construction).
