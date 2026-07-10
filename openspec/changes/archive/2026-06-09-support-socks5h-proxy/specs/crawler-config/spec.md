## MODIFIED Requirements

### Requirement: Proxy URL validation at config load
The system SHALL validate all proxy URL fields at config load time. Valid schemes are `http`, `https`, `socks5`, `socks5h`. URLs MUST have a non-empty host. SOCKS proxy URLs MUST include a valid port. Invalid URLs SHALL cause immediate failure with a descriptive error message.

#### Scenario: Malformed proxy URL
- **WHEN** `[crawler]` sets `proxy = "not-a-url"`
- **THEN** `ConfigManager` initialization SHALL raise an error indicating the invalid proxy URL

#### Scenario: Unsupported proxy scheme
- **WHEN** `[crawler]` sets `proxy = "ftp://127.0.0.1:21"`
- **THEN** `ConfigManager` initialization SHALL raise an error indicating unsupported scheme

#### Scenario: Valid SOCKS5 URL accepted
- **WHEN** `[crawler]` sets `socks5_proxy = "socks5://user:pass@127.0.0.1:1080"`
- **THEN** `ConfigManager` initialization SHALL succeed

#### Scenario: Valid SOCKS5H URL accepted
- **WHEN** `[crawler]` sets `socks5_proxy = "socks5h://user:pass@127.0.0.1:1080"`
- **THEN** `ConfigManager` initialization SHALL succeed

#### Scenario: SOCKS proxy without port rejected
- **WHEN** `[crawler]` sets `socks5_proxy = "socks5h://127.0.0.1"`
- **THEN** `ConfigManager` initialization SHALL raise an error indicating the missing proxy port
