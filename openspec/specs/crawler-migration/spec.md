## ADDED Requirements

### Requirement: LeetCode crawler inherits BaseCrawler
`LeetCodeClient` SHALL inherit from `BaseCrawler` with `crawler_name="leetcode"`. All `aiohttp.ClientSession()` creation points SHALL be replaced with `self._create_aiohttp_session()`. All hardcoded `User-Agent` headers SHALL be replaced with `self._headers()`. All request calls SHALL pass `proxy=self._get_aiohttp_request_proxy(scheme)` when applicable.

#### Scenario: UA from config replaces hardcoded value
- **WHEN** `[crawler]` sets `user_agent = "CustomBot/1.0"`
- **THEN** LeetCode requests use `"CustomBot/1.0"` instead of hardcoded `"Mozilla/5.0"`

#### Scenario: Proxy applied to all LeetCode requests
- **WHEN** `[crawler.leetcode]` sets `https_proxy = "http://lc-proxy:8080"`
- **THEN** all LeetCode HTTP requests route through `"http://lc-proxy:8080"`

#### Scenario: No config regression
- **WHEN** `[crawler]` has no proxy or user_agent fields
- **THEN** LeetCode crawler uses fallback UA `"Mozilla/5.0 (compatible; OJ-API-Bot/1.0)"` and no proxy

#### Scenario: CLI invocation unchanged
- **WHEN** Rust invokes `uv run python3 leetcode.py --daily`
- **THEN** the command succeeds with identical CLI interface as before

#### Scenario: No bare ClientSession remaining
- **WHEN** LeetCode crawler source is inspected
- **THEN** zero direct `aiohttp.ClientSession()` calls exist outside of `_create_aiohttp_session()`

### Requirement: Codeforces crawler inherits BaseCrawler
`CodeforcesClient` SHALL inherit from `BaseCrawler` with `crawler_name="codeforces"`. Session creation SHALL use `self._create_curl_session(impersonate="chrome124")`. The `_headers()` method SHALL be overridden to never inject `User-Agent`, preserving `impersonate` fingerprint consistency. Only proxy config is consumed.

#### Scenario: User-Agent always ignored
- **WHEN** `[crawler]` sets `user_agent = "CustomBot/1.0"` and `[crawler.codeforces]` sets `user_agent = "AnotherBot/2.0"`
- **THEN** Codeforces requests do NOT include a custom `User-Agent` header (impersonate handles it)

#### Scenario: Proxy applied to Codeforces
- **WHEN** `[crawler]` sets `proxy = "http://127.0.0.1:7890"`
- **THEN** Codeforces `AsyncSession` is created with `proxies={"http": "http://127.0.0.1:7890", "https": "http://127.0.0.1:7890"}`

#### Scenario: No config regression
- **WHEN** `[crawler]` has no proxy fields
- **THEN** Codeforces crawler behaves identically to current (impersonate="chrome124", no proxy)

#### Scenario: CLI invocation unchanged
- **WHEN** Rust invokes `uv run python3 codeforces.py --sync`
- **THEN** the command succeeds with identical CLI interface as before

### Requirement: AtCoder crawler inherits BaseCrawler
`AtCoderClient` SHALL inherit from `BaseCrawler` with `crawler_name="atcoder"`. All `aiohttp.ClientSession()` creation SHALL use `self._create_aiohttp_session()`. Hardcoded `User-Agent` (`"LeetCodeDailyDiscordBot/1.0"`) SHALL be replaced with `self._headers()`.

#### Scenario: UA from config replaces hardcoded value
- **WHEN** `[crawler.atcoder]` sets `user_agent = "AtCoderBot/1.0"`
- **THEN** AtCoder requests use `"AtCoderBot/1.0"` instead of hardcoded `"LeetCodeDailyDiscordBot/1.0"`

#### Scenario: Proxy applied to AtCoder
- **WHEN** `[crawler]` sets `socks5_proxy = "socks5://127.0.0.1:1080"`
- **THEN** AtCoder sessions use `ProxyConnector` with the SOCKS5 URL

#### Scenario: No config regression
- **WHEN** `[crawler]` has no proxy or user_agent fields
- **THEN** AtCoder crawler uses fallback UA `"Mozilla/5.0 (compatible; OJ-API-Bot/1.0)"` and no proxy

#### Scenario: CLI invocation unchanged
- **WHEN** Rust invokes `uv run python3 atcoder.py --sync`
- **THEN** the command succeeds with identical CLI interface as before

### Requirement: Module-level UA constants removed
All module-level `USER_AGENT` or hardcoded User-Agent string constants in `leetcode.py`, `codeforces.py`, and `atcoder.py` SHALL be removed. UA injection is handled exclusively by `BaseCrawler._headers()`.

#### Scenario: No hardcoded UA strings
- **WHEN** crawler source files are searched for hardcoded User-Agent strings
- **THEN** no `"Mozilla/5.0"` or `"LeetCodeDailyDiscordBot/1.0"` string literals remain in request header construction
