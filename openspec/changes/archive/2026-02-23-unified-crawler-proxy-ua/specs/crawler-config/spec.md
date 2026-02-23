## ADDED Requirements

### Requirement: Crawler HTTP config fields in config.toml
The `[crawler]` section in `config.toml` SHALL support the following optional fields: `user_agent` (string), `proxy` (string), `http_proxy` (string), `https_proxy` (string), `socks5_proxy` (string). All fields default to unset when omitted.

#### Scenario: Global proxy shorthand
- **WHEN** `[crawler]` sets `proxy = "http://127.0.0.1:7890"` and no scheme-specific proxy is set
- **THEN** `resolve_proxy("http")` and `resolve_proxy("https")` both return `"http://127.0.0.1:7890"`

#### Scenario: Scheme-specific proxy overrides shorthand
- **WHEN** `[crawler]` sets `proxy = "http://fallback:8080"` and `https_proxy = "http://specific:9090"`
- **THEN** `resolve_proxy("https")` returns `"http://specific:9090"` and `resolve_proxy("http")` returns `"http://fallback:8080"`

#### Scenario: SOCKS5 fallback
- **WHEN** `[crawler]` sets only `socks5_proxy = "socks5://127.0.0.1:1080"`
- **THEN** `resolve_proxy("http")` and `resolve_proxy("https")` both return `"socks5://127.0.0.1:1080"`

#### Scenario: Scheme-specific wins over socks5
- **WHEN** `[crawler]` sets `https_proxy = "http://specific:9090"` and `socks5_proxy = "socks5://127.0.0.1:1080"`
- **THEN** `resolve_proxy("https")` returns `"http://specific:9090"`

#### Scenario: No proxy configured
- **WHEN** `[crawler]` has no proxy fields set
- **THEN** `resolve_proxy("http")` and `resolve_proxy("https")` both return `None`

### Requirement: Per-crawler config overrides
The system SHALL support per-crawler override sections `[crawler.<name>]` (where `<name>` is `leetcode`, `codeforces`, or `atcoder`). Each field in the override section, if present and non-empty, SHALL replace the corresponding global `[crawler]` field. Missing fields SHALL inherit from global.

#### Scenario: Partial override inherits missing fields
- **WHEN** `[crawler]` sets `user_agent = "GlobalBot/1.0"` and `https_proxy = "http://global:8080"`, and `[crawler.leetcode]` sets only `https_proxy = "http://lc:9090"`
- **THEN** `get_crawler_config("leetcode")` returns `user_agent = "GlobalBot/1.0"` and `https_proxy = "http://lc:9090"`

#### Scenario: Unknown crawler name returns global defaults
- **WHEN** no `[crawler.spoj]` section exists
- **THEN** `get_crawler_config("spoj")` returns the global `[crawler]` values

#### Scenario: Empty string treated as unset
- **WHEN** `[crawler]` sets `https_proxy = "http://global:8080"` and `[crawler.leetcode]` sets `https_proxy = ""`
- **THEN** `get_crawler_config("leetcode").https_proxy` is `None` and inherits global value `"http://global:8080"` through merge

### Requirement: CrawlerHttpConfig dataclass
`ConfigManager` SHALL provide a `get_crawler_config(crawler_name: str) -> CrawlerHttpConfig` method. `CrawlerHttpConfig` SHALL be a frozen dataclass with fields: `user_agent`, `proxy`, `http_proxy`, `https_proxy`, `socks5_proxy` (all `Optional[str]`, default `None`). It SHALL provide `resolve_proxy(scheme: str) -> Optional[str]` accepting `"http"` or `"https"`.

#### Scenario: Idempotent config retrieval
- **WHEN** `get_crawler_config("leetcode")` is called twice with no config changes
- **THEN** both calls return structurally identical `CrawlerHttpConfig` instances

#### Scenario: Invalid scheme rejected
- **WHEN** `resolve_proxy("ftp")` is called
- **THEN** the method SHALL raise `ValueError`

#### Scenario: resolve_proxy resolution chain
- **WHEN** merged config has `https_proxy = "http://a:1"`, `socks5_proxy = "socks5://b:2"`, `proxy = "http://c:3"`
- **THEN** `resolve_proxy("https")` returns `"http://a:1"` (scheme-specific wins)

### Requirement: Proxy URL validation at config load
The system SHALL validate all proxy URL fields at config load time. Valid schemes are `http`, `https`, `socks5`, `socks5h`. URLs MUST have a non-empty host. Invalid URLs SHALL cause immediate failure with a descriptive error message.

#### Scenario: Malformed proxy URL
- **WHEN** `[crawler]` sets `proxy = "not-a-url"`
- **THEN** `ConfigManager` initialization SHALL raise an error indicating the invalid proxy URL

#### Scenario: Unsupported proxy scheme
- **WHEN** `[crawler]` sets `proxy = "ftp://127.0.0.1:21"`
- **THEN** `ConfigManager` initialization SHALL raise an error indicating unsupported scheme

#### Scenario: Valid SOCKS5 URL accepted
- **WHEN** `[crawler]` sets `socks5_proxy = "socks5://user:pass@127.0.0.1:1080"`
- **THEN** `ConfigManager` initialization SHALL succeed

### Requirement: Rust config compatibility
New TOML fields and sub-tables (`[crawler.<name>]`) SHALL be silently ignored by Rust's `CrawlerConfig` serde deserialization. Zero Rust code changes are required.

#### Scenario: Rust build unaffected
- **WHEN** `config.toml` contains new `[crawler]` fields and `[crawler.leetcode]` sub-table
- **THEN** `cargo build --release` SHALL succeed without any Rust code modifications
