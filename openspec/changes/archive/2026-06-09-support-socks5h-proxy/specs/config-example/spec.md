## MODIFIED Requirements

### Requirement: config.toml.example documents crawler HTTP settings
`config.toml.example` SHALL include commented-out examples for all new `[crawler]` fields: `user_agent`, `proxy`, `http_proxy`, `https_proxy`, `socks5_proxy`. It SHALL document supported proxy schemes `http://`, `https://`, `socks5://`, and `socks5h://`, including that `socks5h://` resolves DNS through the proxy. It SHALL also include commented-out examples for per-crawler override sections `[crawler.leetcode]`, `[crawler.codeforces]`, `[crawler.atcoder]` with a note that Codeforces ignores `user_agent`.

#### Scenario: Example file contains all new fields
- **WHEN** `config.toml.example` is inspected
- **THEN** it contains commented examples for `user_agent`, `proxy`, `http_proxy`, `https_proxy`, `socks5_proxy` under `[crawler]`

#### Scenario: Proxy schemes documented
- **WHEN** `config.toml.example` is inspected
- **THEN** it documents `http://`, `https://`, `socks5://`, and `socks5h://` as supported crawler proxy schemes

#### Scenario: SOCKS5H DNS behavior documented
- **WHEN** `config.toml.example` is inspected
- **THEN** it explains that `socks5h://` resolves DNS through the proxy

#### Scenario: Per-crawler override examples present
- **WHEN** `config.toml.example` is inspected
- **THEN** it contains commented `[crawler.leetcode]`, `[crawler.codeforces]`, `[crawler.atcoder]` sections with field examples

#### Scenario: Codeforces UA note present
- **WHEN** `config.toml.example` is inspected
- **THEN** a comment under `[crawler.codeforces]` notes that `user_agent` is ignored (impersonate handles it)
