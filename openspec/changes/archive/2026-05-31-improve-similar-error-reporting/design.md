## Context

`POST /api/v1/similar` is the only public API path in scope for this change. It synchronously invokes `uv run python3 embedding_cli.py --embed-text <query>`, and that Python mode currently performs both query rewrite and embedding generation before returning JSON to Rust. Rust treats any non-zero subprocess exit as `embedding service failed`, so rewrite misconfiguration, rewrite timeout, provider errors, and embedding failures are indistinguishable to API clients and require stderr inspection.

The broader application already uses `ProblemDetail` as the RFC 7807 HTTP error envelope. This design keeps that envelope and adds a small stage-aware contract only between `embedding_cli.py --embed-text` and `POST /api/v1/similar`.

## Goals / Non-Goals

**Goals:**
- Distinguish rewrite-stage failures from embedding-stage failures for `POST /api/v1/similar`.
- Keep client-facing error details sanitized and stable.
- Preserve full provider details in logs for operator diagnosis.
- Avoid changing successful similar-search responses.
- Keep the implementation local enough to reuse later without forcing a global API error taxonomy now.

**Non-Goals:**
- Do not change `GET /api/v1/similar/{source}/{id}`.
- Do not change admin crawler, admin embedding batch, or daily fallback subprocess error semantics.
- Do not expose Python stderr, provider messages, API keys, model names, or base URLs in public responses.
- Do not solve SDK timeout propagation in this change; that remains a separate reliability improvement.

## Decisions

### 1. Use a structured JSON error envelope from `--embed-text`

`embedding_cli.py --embed-text` will catch failures around the rewrite and embedding phases separately and emit a machine-readable JSON error envelope before exiting non-zero. The envelope should be minimal and sanitized, for example:

```json
{
  "error": {
    "stage": "rewrite",
    "kind": "provider_error",
    "message": "query rewrite service failed"
  }
}
```

Allowed stages for this change are `config`, `rewrite`, `embedding`, and `output`. Rust will treat unknown stages as `embedding` pipeline failures rather than exposing raw text.

**Rationale:** stdout JSON is already the success contract between Python and Rust. Extending it with an explicit error shape avoids brittle stderr parsing while preserving the existing subprocess boundary.

**Alternative considered:** parse stderr in Rust. Rejected because provider tracebacks are not stable and may contain sensitive configuration details.

### 2. Map stage-aware errors to sanitized `ProblemDetail` details

Rust will parse stdout on non-zero subprocess exits. If it contains a recognized error envelope, `POST /api/v1/similar` will return a stage-specific 502 detail:

- `config` → `embedding service configuration failed`
- `rewrite` → `query rewrite service failed`
- `embedding` → `embedding service failed`
- `output` → `invalid embedding response`

If stdout is missing, invalid, or not an error envelope, Rust will fall back to the current generic `embedding service failed` behavior and log stderr.

**Rationale:** clients get actionable high-level stage information without receiving provider internals. Existing clients that only branch on HTTP status continue to work.

**Alternative considered:** add new fields to `ProblemDetail`. Rejected for this change because it would alter the shared error schema and broaden scope beyond `POST /api/v1/similar`.

### 3. Keep stderr as log-only diagnostic data

Python may still write full exception details to stderr or logs. Rust will log stderr along with exit status and, when available, the structured stage/kind. The public response will use only the sanitized message derived from the stage.

**Rationale:** this preserves operational visibility while avoiding accidental leakage of API keys, provider URLs, prompts, or stack traces.

### 4. Scope the reusable pattern to this endpoint only

The stage-aware envelope will be implemented near the `--embed-text` code path rather than as a global subprocess framework. Names and helper functions may be written so future crawler or admin jobs can adopt them, but no other endpoint behavior changes in this change.

**Rationale:** `POST /api/v1/similar` is the immediate user-facing pain point. Admin jobs use asynchronous log/progress channels and need a separate design if standardized later.

## Risks / Trade-offs

- **Risk: Python crashes before writing the error envelope** → Rust keeps the generic fallback and logs stderr.
- **Risk: stdout contains mixed log text and JSON** → keep `--embed-text` structured output as the only stdout payload; diagnostic logs should go to stderr/logger.
- **Risk: exposing too much in error messages** → use fixed public messages derived from stage, not provider exception strings.
- **Risk: future stages need more detail** → keep stage/kind fields extensible, but do not add client-visible schema beyond RFC 7807 detail in this change.
- **Risk: clients depend on exact old detail string** → HTTP status remains 502; this is an intentional improvement to a diagnostic message, not a success-schema change.

## Migration Plan

1. Add stage-aware error emission to `embedding_cli.py --embed-text`.
2. Add Rust-side parsing for success and error envelopes in `POST /api/v1/similar`.
3. Add tests for rewrite failure, embedding failure, invalid error envelope fallback, and success response compatibility.
4. Update OpenAPI description/tests only if they assert exact 502 detail.
5. Rollback by reverting to the previous generic non-zero subprocess handling; no data migration is required.

## Open Questions

None. Scope is explicitly limited to `POST /api/v1/similar`.
