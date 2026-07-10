## ADDED Requirements

### Requirement: Tencent Docs token configuration
The Python crawler configuration SHALL support a direct local Tencent Docs token and an environment-variable fallback. A non-empty direct `[daily_sources.tencent_docs].token` value SHALL take precedence; otherwise the crawler SHALL resolve the variable named by `token_env`, which defaults to `TENCENT_DOCS_TOKEN`. Tracked examples and specifications SHALL contain only an empty token placeholder.

#### Scenario: Direct token takes precedence
- **WHEN** config contains a non-empty `[daily_sources.tencent_docs].token`
- **AND** the configured environment variable is also set
- **THEN** `ConfigManager` resolves the trimmed direct token

#### Scenario: Empty direct token falls back to environment
- **WHEN** config omits or leaves `[daily_sources.tencent_docs].token` empty
- **AND** the resolved token environment variable is set in the process environment
- **THEN** the 0x3f online crawler uses that trimmed environment value as the Tencent Docs MCP Authorization token

#### Scenario: Empty fallback name with direct token
- **WHEN** config contains a non-empty `[daily_sources.tencent_docs].token`
- **AND** `[daily_sources.tencent_docs].token_env` is empty
- **THEN** the crawler uses the direct token without evaluating the fallback

#### Scenario: No token source configured
- **WHEN** `[daily_sources.tencent_docs].token` and `token_env` are both empty
- **THEN** configuration validation fails with a descriptive error
