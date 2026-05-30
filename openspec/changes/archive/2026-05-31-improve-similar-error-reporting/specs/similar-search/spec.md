## MODIFIED Requirements

### Requirement: Similar search by text
The system SHALL find similar problems via `POST /api/v1/similar` with JSON body fields `query`, `limit`, `threshold`, and `source`. It SHALL invoke the Python subprocess `embedding_cli.py --embed-text` to generate an embedding, then perform KNN search. When the subprocess reports a stage-aware failure, the system SHALL return an RFC 7807 error with sanitized detail for the failed stage and SHALL NOT expose subprocess stderr or provider internals in the response.

#### Scenario: Successful text search
- **WHEN** client sends `POST /api/v1/similar` with JSON body `{"query":"binary search on sorted array","limit":5}`
- **THEN** system returns up to 5 similar problems with similarity scores

#### Scenario: Query too short
- **WHEN** client sends `POST /api/v1/similar` with JSON body `{"query":"ab"}` (less than 3 characters)
- **THEN** system returns HTTP 400 with error detail indicating minimum 3 characters

#### Scenario: Python subprocess timeout
- **WHEN** the Python subprocess does not respond within the configured `[embedding].timeout_secs`
- **THEN** system kills the subprocess and returns HTTP 504

#### Scenario: Query rewrite failure
- **WHEN** the Python subprocess exits non-zero and reports stage `rewrite`
- **THEN** system returns HTTP 502 with detail `query rewrite service failed`
- **AND** the response does not include subprocess stderr, provider exception text, API keys, model names, base URLs, or stack traces

#### Scenario: Embedding generation failure
- **WHEN** the Python subprocess exits non-zero and reports stage `embedding`
- **THEN** system returns HTTP 502 with detail `embedding service failed`
- **AND** the response does not include subprocess stderr, provider exception text, API keys, model names, base URLs, or stack traces

#### Scenario: Configuration failure
- **WHEN** the Python subprocess exits non-zero and reports stage `config`
- **THEN** system returns HTTP 502 with detail `embedding service configuration failed`
- **AND** the response does not include subprocess stderr, provider exception text, API keys, model names, base URLs, or stack traces

#### Scenario: Invalid subprocess output
- **WHEN** the Python subprocess exits successfully but outputs invalid JSON
- **THEN** system returns HTTP 502 with detail `invalid embedding response`

#### Scenario: Unstructured subprocess failure fallback
- **WHEN** the Python subprocess exits non-zero without a recognized stage-aware error envelope
- **THEN** system returns HTTP 502 with detail `embedding service failed`
- **AND** system logs stderr for operator diagnosis
