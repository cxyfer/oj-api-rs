## ADDED Requirements

### Requirement: BaseCrawler plain base class
`BaseCrawler` SHALL be a plain Python class (not ABC) defined in `scripts/utils/base_crawler.py`. It SHALL accept a `crawler_name: str` parameter and read `CrawlerHttpConfig` from `ConfigManager` on initialization.

#### Scenario: Direct instantiation
- **WHEN** `BaseCrawler("leetcode")` is instantiated directly
- **THEN** no `TypeError` is raised (it is not an abstract class)

#### Scenario: Config loaded on init
- **WHEN** a subclass of `BaseCrawler` is created with `crawler_name="atcoder"`
- **THEN** `self.http_config` contains the merged `CrawlerHttpConfig` for `"atcoder"`

### Requirement: Header helper with fallback UA
`BaseCrawler` SHALL provide `_headers(referer: Optional[str] = None) -> dict` that injects `User-Agent` from config. When no `user_agent` is configured, it SHALL use `"Mozilla/5.0 (compatible; OJ-API-Bot/1.0)"` as fallback. The method SHALL accept an optional `referer` parameter to include a `Referer` header.

#### Scenario: Configured UA used
- **WHEN** config has `user_agent = "CustomBot/2.0"`
- **THEN** `_headers()` returns a dict containing `{"User-Agent": "CustomBot/2.0"}`

#### Scenario: Fallback UA when unconfigured
- **WHEN** no `user_agent` is set in config (global or per-crawler)
- **THEN** `_headers()` returns `{"User-Agent": "Mozilla/5.0 (compatible; OJ-API-Bot/1.0)"}`

#### Scenario: Referer included when provided
- **WHEN** `_headers(referer="https://example.com")` is called
- **THEN** the returned dict includes `{"Referer": "https://example.com"}`

### Requirement: aiohttp session factory
`BaseCrawler` SHALL provide `_create_aiohttp_session(**kwargs) -> aiohttp.ClientSession` that:
- Sets `trust_env=False` always
- When resolved proxy is SOCKS5: creates session with `aiohttp_socks.ProxyConnector.from_url(proxy_url)` as connector
- When resolved proxy is HTTP/HTTPS: returns plain session (proxy injected per-request)
- When no proxy: returns plain session
- Merges caller-provided kwargs (headers, cookies, etc.)

#### Scenario: SOCKS5 proxy uses ProxyConnector
- **WHEN** resolved proxy for the crawler is `"socks5://127.0.0.1:1080"`
- **THEN** the returned session uses `ProxyConnector` as its connector

#### Scenario: HTTP proxy returns plain session
- **WHEN** resolved proxy is `"http://127.0.0.1:8080"`
- **THEN** the returned session has no special connector; proxy is applied per-request

#### Scenario: No proxy returns plain session
- **WHEN** no proxy is configured
- **THEN** the returned session has default connector and `trust_env=False`

#### Scenario: Environment variables do not leak
- **WHEN** `HTTP_PROXY=http://env:1234` is set in environment and no proxy in config
- **THEN** the session does NOT route through `http://env:1234`

### Requirement: curl_cffi session factory
`BaseCrawler` SHALL provide `_create_curl_session(**kwargs) -> curl_cffi.requests.AsyncSession` that:
- Sets `trust_env=False` always
- Passes `proxies={"http": url, "https": url}` when proxy is configured (keys without `://`)
- Preserves caller-provided kwargs (`impersonate`, etc.)

#### Scenario: Proxy configured for curl_cffi
- **WHEN** resolved proxy is `"http://127.0.0.1:8080"`
- **THEN** session is created with `proxies={"http": "http://127.0.0.1:8080", "https": "http://127.0.0.1:8080"}`

#### Scenario: SOCKS5 proxy for curl_cffi
- **WHEN** resolved proxy is `"socks5://127.0.0.1:1080"`
- **THEN** session is created with `proxies={"http": "socks5://127.0.0.1:1080", "https": "socks5://127.0.0.1:1080"}`

#### Scenario: No proxy for curl_cffi
- **WHEN** no proxy is configured
- **THEN** session is created with `trust_env=False` and no `proxies` parameter

### Requirement: Request-level proxy helper for aiohttp
`BaseCrawler` SHALL provide `_get_aiohttp_request_proxy(scheme: str) -> Optional[str]` that returns the resolved proxy URL for non-SOCKS5 proxies (to be passed as `proxy=` on individual requests). For SOCKS5, it SHALL return `None` (handled at connector level).

#### Scenario: HTTP proxy returned for request-level use
- **WHEN** resolved proxy is `"http://127.0.0.1:8080"` and scheme is `"https"`
- **THEN** returns `"http://127.0.0.1:8080"`

#### Scenario: SOCKS5 returns None for request-level
- **WHEN** resolved proxy is `"socks5://127.0.0.1:1080"`
- **THEN** returns `None` (proxy handled by ProxyConnector at session level)
