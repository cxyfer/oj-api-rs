## MODIFIED Requirements

### Requirement: aiohttp session factory
`BaseCrawler` SHALL provide `_create_aiohttp_session(**kwargs) -> aiohttp.ClientSession` that:
- Sets `trust_env=False` always
- When resolved proxy is `socks5://`: creates session with `aiohttp_socks.ProxyConnector.from_url(proxy_url)` as connector
- When resolved proxy is `socks5h://`: creates session with a SOCKS5 `aiohttp_socks.ProxyConnector` that enables remote DNS resolution
- When resolved proxy is HTTP/HTTPS: returns plain session (proxy injected per-request)
- When no proxy: returns plain session
- Merges caller-provided kwargs (headers, cookies, etc.)

#### Scenario: SOCKS5 proxy uses ProxyConnector
- **WHEN** resolved proxy for the crawler is `"socks5://127.0.0.1:1080"`
- **THEN** the returned session uses `ProxyConnector` as its connector

#### Scenario: SOCKS5H proxy uses remote DNS ProxyConnector
- **WHEN** resolved proxy for the crawler is `"socks5h://127.0.0.1:1080"`
- **THEN** the returned session uses a SOCKS5 `ProxyConnector` with remote DNS enabled

#### Scenario: SOCKS5H proxy preserves credentials
- **WHEN** resolved proxy for the crawler is `"socks5h://user:pass@127.0.0.1:1080"`
- **THEN** the returned session uses a SOCKS5 `ProxyConnector` configured with username `"user"`, password `"pass"`, and remote DNS enabled

#### Scenario: HTTP proxy returns plain session
- **WHEN** resolved proxy is `"http://127.0.0.1:8080"`
- **THEN** the returned session has no special connector; proxy is applied per-request

#### Scenario: No proxy returns plain session
- **WHEN** no proxy is configured
- **THEN** the returned session has default connector and `trust_env=False`

#### Scenario: Environment variables do not leak
- **WHEN** `HTTP_PROXY=http://env:1234` is set in environment and no proxy in config
- **THEN** the session does NOT route through `http://env:1234`

### Requirement: Request-level proxy helper for aiohttp
`BaseCrawler` SHALL provide `_get_aiohttp_request_proxy(scheme: str) -> Optional[str]` that returns the resolved proxy URL for non-SOCKS5 proxies (to be passed as `proxy=` on individual requests). For `socks5://` and `socks5h://`, it SHALL return `None` (handled at connector level).

#### Scenario: HTTP proxy returned for request-level use
- **WHEN** resolved proxy is `"http://127.0.0.1:8080"` and scheme is `"https"`
- **THEN** returns `"http://127.0.0.1:8080"`

#### Scenario: SOCKS5 returns None for request-level
- **WHEN** resolved proxy is `"socks5://127.0.0.1:1080"`
- **THEN** returns `None` (proxy handled by ProxyConnector at session level)

#### Scenario: SOCKS5H returns None for request-level
- **WHEN** resolved proxy is `"socks5h://127.0.0.1:1080"`
- **THEN** returns `None` (proxy handled by ProxyConnector at session level)
