## Why

`POST /api/v1/similar` currently reports every non-zero `embedding_cli.py --embed-text` exit as `embedding service failed`, even when the failure happened during query rewrite, configuration, or response serialization. This makes production failures misleading and slows diagnosis while still requiring the public API to avoid exposing provider stderr or secrets.

## What Changes

- Classify the text-similar embedding pipeline failures by stage instead of collapsing all subprocess failures into one message.
- Preserve RFC 7807 error responses and keep provider stderr out of client-facing responses.
- Return stable, sanitized 502 details that distinguish rewrite failures from embedding failures and invalid subprocess output.
- Log enough structured context for operators to identify the failing stage and underlying provider error from server logs.
- No breaking API schema changes; success responses remain unchanged.

## Capabilities

### New Capabilities

### Modified Capabilities
- `similar-search`: Text-query similar search error handling will distinguish query rewrite failures, embedding generation failures, and invalid subprocess responses while preserving generic sanitized client responses.
- `embedding-reliability`: `embedding_cli.py --embed-text` will expose stage-aware failure information for its rewrite and embedding phases.

## Impact

- Affected API: `POST /api/v1/similar` error responses for Python subprocess failures.
- Affected code: `src/api/similar.rs`, `scripts/embedding_cli.py`, and related embedding provider error handling/logging as needed.
- Affected specs: `similar-search`, `embedding-reliability`.
- No database schema changes, no public success-response changes, and no provider credential exposure in responses.
