## ADDED Requirements

### Requirement: Health check endpoint
The system SHALL expose `GET /health` (no authentication required) that validates DB connectivity, sqlite-vec extension loading, and embedding vector dimension consistency.

#### Scenario: All healthy
- **WHEN** DB connection succeeds AND `vec_version()` returns a value AND `vec_length()` on the embeddings table returns 768
- **THEN** system returns HTTP 200 with `{"status": "ok", "db": true, "sqlite_vec": true, "vec_dimension": 768, "version": "..."}`

#### Scenario: DB connection failure
- **WHEN** DB connection cannot be established
- **THEN** system returns HTTP 503 with `{"status": "unhealthy", "db": false, ...}`

#### Scenario: sqlite-vec not loaded
- **WHEN** `vec_version()` call fails
- **THEN** system returns HTTP 503 with `{"status": "unhealthy", "sqlite_vec": false, ...}`

#### Scenario: Dimension mismatch
- **WHEN** `vec_length()` returns a value other than 768
- **THEN** system returns HTTP 503 with `{"status": "unhealthy", "vec_dimension": <actual>, ...}`

### Requirement: Startup self-check (fail-fast)
The system SHALL verify DB connectivity, sqlite-vec loading, and vector dimension at startup. If any check fails, the system SHALL log an error and exit immediately with non-zero code.

#### Scenario: Startup with invalid DB path
- **WHEN** `DATABASE_PATH` points to a non-existent file
- **THEN** system logs error and exits with code 1

#### Scenario: Startup with sqlite-vec registration failure
- **WHEN** `sqlite3_auto_extension` fails to register sqlite-vec
- **THEN** system logs error and exits with code 1

### Requirement: Graceful shutdown
The system SHALL handle SIGTERM/SIGINT by stopping acceptance of new connections, waiting up to 10 seconds for in-flight requests to complete, then terminating. Running Python subprocesses (crawlers, embeddings) SHALL be killed on shutdown.

#### Scenario: Clean shutdown
- **WHEN** SIGTERM is received with 3 in-flight requests
- **THEN** system completes those 3 requests (within 10s) then exits

#### Scenario: Forced shutdown on timeout
- **WHEN** SIGTERM is received and in-flight requests do not complete within 10s
- **THEN** system forcefully terminates after 10s
