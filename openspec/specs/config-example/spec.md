## ADDED Requirements

### Requirement: config.toml.example documents crawler HTTP settings
`config.toml.example` SHALL include commented-out examples for all new `[crawler]` fields: `user_agent`, `proxy`, `http_proxy`, `https_proxy`, `socks5_proxy`. It SHALL also include commented-out examples for per-crawler override sections `[crawler.leetcode]`, `[crawler.codeforces]`, `[crawler.atcoder]` with a note that Codeforces ignores `user_agent`.

#### Scenario: Example file contains all new fields
- **WHEN** `config.toml.example` is inspected
- **THEN** it contains commented examples for `user_agent`, `proxy`, `http_proxy`, `https_proxy`, `socks5_proxy` under `[crawler]`

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
