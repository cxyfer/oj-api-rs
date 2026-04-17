## MODIFIED Requirements

### Requirement: Bearer token gate also applies to HTTP MCP
The native `/mcp` HTTP MCP endpoint SHALL use the same `TokenAuthEnabled` gate and Bearer-token validation logic as `/api/v1/*` and `/status`.

#### Scenario: HTTP MCP without token while auth enabled
- **WHEN** token auth is enabled and client sends MCP initialize to `POST /mcp` without `Authorization` header
- **THEN** system returns HTTP 401 with RFC 7807 error body

#### Scenario: HTTP MCP with valid token while auth enabled
- **WHEN** token auth is enabled and client sends MCP initialize to `POST /mcp` with a valid active Bearer token
- **THEN** the request is accepted and MCP handshake proceeds

#### Scenario: HTTP MCP without token while auth disabled
- **WHEN** token auth is disabled and client sends MCP initialize to `POST /mcp` without Authorization header
- **THEN** the request is accepted and MCP handshake proceeds
