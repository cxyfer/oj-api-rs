## ADDED Requirements

### Requirement: Batch embedding failure triggers retry then bisection
The `_flush_embeddings` function SHALL retry a failed batch up to `max_retries` times with exponential backoff. If the full batch still fails, it SHALL bisect the batch into halves and retry each half recursively. At batch_size=1, a persistent failure SHALL be recorded as a permanent failure for that problem_id. The maximum API calls per original batch SHALL be bounded by `max_retries * ceil(log2(batch_size))` per item.

#### Scenario: Transient batch failure recovers on retry
- **WHEN** `embed_batch()` fails with a transient error on first attempt
- **AND** succeeds on second attempt
- **THEN** all items in the batch are embedded successfully
- **AND** total API calls = 2

#### Scenario: Permanent batch failure bisects to isolate bad item
- **WHEN** `embed_batch()` fails for a batch of 4 items due to one problematic item
- **THEN** system bisects to [2, 2], identifies the half containing the bad item
- **AND** further bisects to isolate the single failing item
- **AND** records exactly 1 failed problem_id

#### Scenario: All items in batch permanently fail
- **WHEN** `embed_batch()` fails for all items even at batch_size=1
- **THEN** all problem_ids are recorded as failed with `embed_permanent` reason
- **AND** no infinite retry loop occurs

### Requirement: Rewrite failures log problem ID and reason
Every rewrite failure (timeout, API error, empty result) SHALL log the specific problem_id and failure reason. Timeout errors SHALL include the timeout duration in the log message. Each failure type SHALL be categorized as: `rewrite_timeout`, `rewrite_error`, or `rewrite_empty`.

#### Scenario: Rewrite timeout includes problem ID
- **WHEN** rewrite for problem "leetcode-42" times out after 60 seconds
- **THEN** log contains problem_id "leetcode-42", reason "rewrite_timeout", and timeout value 60

#### Scenario: Rewrite API error includes problem ID
- **WHEN** rewrite for problem "codeforces-1A" throws an API exception
- **THEN** log contains problem_id "codeforces-1A" and reason "rewrite_error"

### Requirement: Empty content after HTML parsing is tracked distinctly
Problems where `html_to_text()` produces empty/whitespace-only output SHALL be categorized as `empty_content` (distinct from rewrite failures). The problem_id SHALL be logged and included in the final summary under the `skipped.empty_content` category.

#### Scenario: HTML with no extractable text
- **WHEN** problem content is `<div></div>` (valid HTML, no text)
- **THEN** problem is categorized as `skipped.empty_content` in summary
- **AND** problem_id is logged with reason "empty_content"

### Requirement: Structured summary output on completion
Upon completion (success or partial failure), the script SHALL output a JSON summary line to stdout with prefix `EMBEDDING_SUMMARY:`. The summary SHALL contain: `total_pending`, `succeeded`, `skipped` (object with reason-count pairs), `failed` (object with reason-count pairs), `duration_secs`. The invariant `succeeded + sum(skipped) + sum(failed) == total_pending` SHALL hold. Summary SHALL be output even if the pipeline encounters fatal errors (via `finally` block).

#### Scenario: Successful completion summary
- **WHEN** all 100 pending problems are embedded successfully
- **THEN** summary shows `{ "total_pending": 100, "succeeded": 100, "skipped": {}, "failed": {}, "duration_secs": ... }`
- **AND** exit code is 0

#### Scenario: Partial failure summary
- **WHEN** 90 of 100 pending problems succeed, 5 skip (empty content), 5 fail (embed permanent)
- **THEN** summary shows `succeeded=90, skipped={"empty_content":5}, failed={"embed_permanent":5}`
- **AND** `90 + 5 + 5 == 100`
- **AND** exit code is 1

#### Scenario: Summary output despite fatal error
- **WHEN** pipeline crashes due to unhandled exception after processing 50 items
- **THEN** summary is still output with partial counts and remaining items counted as failed

### Requirement: Exit code reflects failure presence
The script SHALL exit with code 0 if and only if `sum(failed.values()) == 0`. Any non-zero failed count SHALL result in exit code 1. Skipped items do NOT contribute to failure.

#### Scenario: All skipped but none failed
- **WHEN** all pending problems are skipped (empty content)
- **THEN** exit code is 0

#### Scenario: One item fails
- **WHEN** 99 succeed and 1 fails
- **THEN** exit code is 1

### Requirement: Progress file is written atomically and monotonically
The script SHALL write progress to `scripts/logs/{job_id}.progress.json` using atomic write (write to temp file, then rename). The `processed_count` (sum of done + skipped across phases) SHALL be monotonically non-decreasing. Progress file SHALL be valid JSON at every observable read.

#### Scenario: Progress file survives process crash
- **WHEN** Python process is killed during a progress write
- **THEN** the progress file contains the last successfully written state (not a partial write)

#### Scenario: Processed count never decreases
- **WHEN** progress file is read at intervals t1 < t2
- **THEN** `processed_count(t2) >= processed_count(t1)`

### Requirement: Idempotent rebuild skip
Running `--build` when all eligible problems already have embeddings (matching current model and dimension) SHALL result in zero LLM API calls and zero new database writes.

#### Scenario: No-op when fully embedded
- **WHEN** all problems with content already have embeddings for the current model
- **THEN** script logs "No pending embeddings to process" and exits with code 0
- **AND** no API calls are made

### Requirement: CLI argument backward compatibility
All existing CLI arguments (`--build`, `--rebuild`, `--query`, `--stats`, `--dry-run`, `--embed-text`, `--source`, `--top-k`, `--min-similarity`, `--batch-size`, `--filter`) SHALL continue to function with unchanged semantics.

#### Scenario: Existing --stats invocation unchanged
- **WHEN** user runs `embedding_cli.py --stats --source leetcode`
- **THEN** output format and content matches previous behavior
