## MODIFIED Requirements

### Requirement: Native Streamable HTTP MCP endpoint
The system SHALL expose a native Streamable HTTP MCP endpoint at `/mcp` on the same axum server and listener as the REST API.

#### Scenario: MCP initialize over `/mcp`
- **WHEN** a client sends a valid MCP `initialize` request to `POST /mcp`
- **THEN** the system returns HTTP 200 and a valid MCP initialize response from the in-process RMCP server

#### Scenario: Shared server lifecycle
- **WHEN** `oj-api-rs` starts listening on `server.listen_addr`
- **THEN** `/mcp` is served by the same process and listener as `/health`, `/status`, and `/api/v1/*`

### Requirement: MCP authentication follows public API token toggle
The `/mcp` endpoint SHALL use the same Bearer-token gate as `/api/v1/*` and `/status`.

#### Scenario: Token auth enabled
- **WHEN** token auth is enabled and a client calls `/mcp` without a valid Bearer token
- **THEN** the system returns HTTP 401 before MCP handshake completes

#### Scenario: Token auth disabled
- **WHEN** token auth is disabled and a client calls `/mcp` without a Bearer token
- **THEN** the system allows the MCP handshake and processes requests normally

### Requirement: MCP tool inventory matches oj-mcp-rs
The MCP server SHALL expose exactly these tools:
- `resolve_problem`
- `get_problem`
- `get_daily_challenge`
- `find_similar_problems`
- `get_platform_status`

#### Scenario: tools/list inventory
- **WHEN** an initialized MCP client requests `tools/list`
- **THEN** the returned tool names contain exactly the 5 tools above

### Requirement: Problem lookup tools return Markdown
`resolve_problem`, `get_problem`, and `get_daily_challenge` SHALL return text content in Markdown format.

#### Scenario: get_problem response shape
- **WHEN** `get_problem` succeeds
- **THEN** the tool result starts with `# {title}` and includes Source, ID, Difficulty, Tags, Link, and AC Rate metadata followed by the formatted statement body

#### Scenario: resolve_problem response shape
- **WHEN** `resolve_problem` succeeds
- **THEN** it returns the same Markdown problem format as `get_problem`

#### Scenario: daily fetch in progress
- **WHEN** `get_daily_challenge` hits a backend 202/fetching state
- **THEN** the MCP tool returns a non-error text result instructing the client to retry later

### Requirement: Similar and status tools return Markdown tables
`find_similar_problems` and `get_platform_status` SHALL return text content formatted as Markdown tables.

#### Scenario: find_similar_problems response
- **WHEN** `find_similar_problems` succeeds
- **THEN** the tool result includes a `# Similar Problems` heading and a Markdown table with source, ID, title, difficulty, similarity, and link

#### Scenario: get_platform_status response
- **WHEN** `get_platform_status` succeeds
- **THEN** the tool result includes a `# OJ Platform Status` heading and a Markdown table of per-platform totals, missing content, and not-embedded counts

### Requirement: MCP tool behavior reuses in-process API logic
The MCP implementation SHALL reuse the in-process `oj-api-rs` handlers / data paths rather than issuing loopback HTTP requests to the same service.

#### Scenario: shared resolve logic
- **WHEN** `resolve_problem` is called through MCP
- **THEN** it resolves LeetCode slugs, source detection, and DB lookups using the same underlying logic as the REST resolve endpoint

#### Scenario: shared similar-search logic
- **WHEN** `find_similar_problems` is called through MCP
- **THEN** text queries still use the existing embedding subprocess path and ID queries still use the existing KNN search path

### Requirement: MCP output is size-bounded
Text responses returned by MCP tools SHALL be truncated at a bounded maximum length rather than emitting arbitrarily large payloads.

#### Scenario: oversized problem statement
- **WHEN** a formatted MCP response exceeds the configured truncation threshold
- **THEN** the returned text is cut at a character boundary and ends with a truncation marker

### Requirement: MCP host allow-list is configurable
The MCP transport SHALL honor `[mcp].allowed_hosts` from config when validating incoming Host headers.

#### Scenario: explicit allowed hosts
- **WHEN** `mcp.allowed_hosts = ["oj.example.com", "localhost:7856"]`
- **THEN** `/mcp` accepts requests for those Host headers and rejects others

#### Scenario: empty allowed host list
- **WHEN** `mcp.allowed_hosts = []`
- **THEN** Host validation is disabled for `/mcp`
