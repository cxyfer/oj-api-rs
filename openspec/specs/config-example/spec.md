## Purpose

Define documentation expectations for generated and checked-in configuration examples.
## Requirements
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

### Requirement: aiohttp-socks added as dependency
`aiohttp-socks` SHALL be added to `scripts/pyproject.toml` as a runtime dependency for SOCKS5 proxy support.

#### Scenario: Dependency present in pyproject.toml
- **WHEN** `scripts/pyproject.toml` is inspected
- **THEN** `aiohttp-socks` appears in the dependencies list

### Requirement: config.toml.example documents embedding timeout defaults
`config.toml.example` SHALL document the `[embedding]` section with both `timeout_secs` and `batch_timeout_secs` so the example configuration reflects the default query and batch embedding timeouts.

#### Scenario: Embedding timeout settings are present
- **WHEN** `config.toml.example` is inspected
- **THEN** it contains `[embedding] timeout_secs = 300` and `batch_timeout_secs = 3600`

### Requirement: config.toml.example documents MCP host allow-list
`config.toml.example` SHALL document the `[mcp]` section with `allowed_hosts = []` and explain that it controls Host allow-list validation for `/mcp`.

#### Scenario: MCP section present
- **WHEN** `config.toml.example` is inspected
- **THEN** it contains an `[mcp]` section with `allowed_hosts = []`

#### Scenario: allow-list semantics documented
- **WHEN** `config.toml.example` is inspected
- **THEN** comments explain that the setting applies to `/mcp` Host validation and that an empty list disables the allow-list
