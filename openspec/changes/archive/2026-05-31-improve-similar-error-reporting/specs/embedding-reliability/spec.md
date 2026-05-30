## ADDED Requirements

### Requirement: Embed-text command reports stage-aware failures
The `embedding_cli.py --embed-text` mode SHALL report failures using a structured JSON error envelope on stdout before exiting non-zero when failure occurs during configuration, query rewrite, or embedding generation. The error envelope SHALL include `error.stage`, `error.kind`, and `error.message`. The public `error.message` SHALL be sanitized and SHALL NOT include provider exception text, API keys, model names, base URLs, prompts, or stack traces.

#### Scenario: Rewrite failure reports rewrite stage
- **WHEN** query rewrite raises an exception in `embedding_cli.py --embed-text`
- **THEN** stdout contains a JSON error envelope with `error.stage = "rewrite"`
- **AND** the process exits non-zero
- **AND** stderr or logs retain diagnostic exception details for operators

#### Scenario: Embedding failure reports embedding stage
- **WHEN** embedding generation raises an exception in `embedding_cli.py --embed-text`
- **THEN** stdout contains a JSON error envelope with `error.stage = "embedding"`
- **AND** the process exits non-zero
- **AND** stderr or logs retain diagnostic exception details for operators

#### Scenario: Configuration failure reports config stage
- **WHEN** provider or model configuration fails before rewrite or embedding can run
- **THEN** stdout contains a JSON error envelope with `error.stage = "config"`
- **AND** the process exits non-zero
- **AND** stderr or logs retain diagnostic exception details for operators

#### Scenario: Successful embed-text output unchanged
- **WHEN** rewrite and embedding generation both succeed
- **THEN** stdout contains the existing success JSON object with `embedding` and `rewritten`
- **AND** the process exits with code 0
