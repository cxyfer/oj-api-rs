## ADDED Requirements

### Requirement: config.toml.example documents Tencent Docs token resolution
`config.toml.example` SHALL document direct local Tencent Docs token configuration and its environment-variable fallback without including any token value. The example SHALL show `token = ""` and `token_env = "TENCENT_DOCS_TOKEN"` under `[daily_sources.tencent_docs]`.

#### Scenario: Tencent Docs token settings present
- **WHEN** `config.toml.example` is inspected
- **THEN** it contains `[daily_sources.tencent_docs]` with `token = ""` and `token_env = "TENCENT_DOCS_TOKEN"`

#### Scenario: No Tencent Docs token value in example
- **WHEN** `config.toml.example` is inspected
- **THEN** it does not contain an actual Tencent Docs token value

### Requirement: config.toml.example documents fixed daily source schedule
`config.toml.example` SHALL document that additional daily sources refresh at fixed UTC+8 08:00, 10:00, and 12:00 and that these times are not configured through TOML in this change.

#### Scenario: Fixed schedule documented
- **WHEN** `config.toml.example` is inspected
- **THEN** comments document the UTC+8 08:00, 10:00, and 12:00 additional daily source refresh schedule
