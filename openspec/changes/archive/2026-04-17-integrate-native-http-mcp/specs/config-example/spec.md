## MODIFIED Requirements

### Requirement: config.toml.example documents MCP host allow-list
`config.toml.example` SHALL document the `[mcp]` section with `allowed_hosts = []` and explain that it controls Host allow-list validation for `/mcp`.

#### Scenario: MCP section present
- **WHEN** `config.toml.example` is inspected
- **THEN** it contains an `[mcp]` section with `allowed_hosts = []`

#### Scenario: allow-list semantics documented
- **WHEN** `config.toml.example` is inspected
- **THEN** comments explain that the setting applies to `/mcp` Host validation and that an empty list disables the allow-list
