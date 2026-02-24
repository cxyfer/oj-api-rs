## Context

The OJ API project uses Python subprocess calls (`uv run python3 embedding_cli.py`) for all LLM-related operations (embedding generation, problem statement rewriting). Currently, all LLM calls are hardcoded to the Gemini API via the `google-genai` SDK, the admin panel has no UI for embedding management, and the batch embedding pipeline has reliability issues causing incomplete vectorization.

The Rust server manages crawler jobs via `CrawlerJob` model with in-memory state (`crawler_lock: Mutex<Option<CrawlerJob>>`, `crawler_history: Mutex<VecDeque<CrawlerJob>>`). Admin UI uses Askama templates, vanilla JS with AJAX polling, and dark-theme CSS with i18n support (en, zh-TW, zh-CN).

Python side uses a producer-consumer architecture (`rewrite_queue` → `embed_queue`) with `EmbeddingRewriter` and `EmbeddingGenerator` classes directly instantiating `google.genai.Client`.

## Goals / Non-Goals

**Goals:**

- Provide admin UI for embedding statistics viewing and job triggering
- Make the embedding pipeline reliable with full problem accounting
- Abstract LLM provider selection to support Gemini and OpenAI-compatible APIs
- Maintain backward compatibility with existing `[gemini]` config section

**Non-Goals:**

- Rust-native LLM calls (Python subprocess model is retained)
- Real-time streaming of Python stdout to browser (file-based progress instead)
- Support for providers beyond Gemini and OpenAI-compatible (extensible but not built)
- Migration tooling for existing embeddings when switching providers
- Admin UI for provider configuration (config.toml only)

## Decisions

### D1: Separate embedding job infrastructure from crawler

Embedding jobs use independent `embedding_lock: Mutex<Option<EmbeddingJob>>` and `embedding_history: Mutex<VecDeque<EmbeddingJob>>` in `AppState`. Crawler and embedding jobs MAY run in parallel.

**Alternatives considered:**
- Shared `crawler_lock`: Rejected because crawler and embedding jobs have different parameters, lifecycles, and status semantics. Sharing would conflate UI state and prevent parallel execution.
- Global heavy-job mutex: Rejected as unnecessary; SQLite WAL mode handles concurrent reads/writes adequately.

### D2: Rust direct DB queries for embedding stats

The `GET /admin/api/embeddings/stats` endpoint queries SQLite directly (via `spawn_blocking`) rather than spawning a Python subprocess.

Queries join `problems`, `problem_embeddings`, and `vec_embeddings` tables to compute per-source counts: total problems, problems with content, already embedded, pending.

**Alternatives considered:**
- Python `--stats` subprocess: Rejected due to subprocess startup latency (~1-2s) on every page load and additional failure surface.

### D3: Real-time progress via shared file

Python writes progress to `scripts/logs/{job_id}.progress.json` atomically (write to temp file then rename). Rust `GET /admin/api/embeddings/status` reads this file when a job is running.

Progress file schema:
```json
{
  "phase": "rewriting|embedding|completed|failed",
  "rewrite_progress": { "done": 45, "total": 100, "skipped": 2 },
  "embed_progress": { "done": 20, "total": 43 },
  "started_at": "2026-02-24T05:00:00Z"
}
```

The `processed_count` (done + skipped) MUST be monotonically non-decreasing.

**Alternatives considered:**
- stdout streaming with pipe read: Rejected due to complexity of async pipe reading in Rust and buffering issues.
- SQLite progress table: Rejected to avoid write contention on the main database during heavy embedding operations.

### D4: LLM provider abstraction with lazy imports

New `scripts/embeddings/providers/` package with:
- `base.py`: Abstract `LLMProvider` class (`embed`, `embed_batch`, `rewrite`)
- `gemini.py`: `GeminiProvider` wrapping `google-genai` SDK
- `openai_compat.py`: `OpenAICompatProvider` wrapping `openai` SDK
- `factory.py`: `create_provider(config)` factory function

Provider-agnostic exceptions: `TransientProviderError` (retryable), `PermanentProviderError` (not retryable). Each adapter maps SDK-specific errors to these types.

Dependencies are lazily imported at adapter instantiation time. Missing unused provider SDK does not cause startup failure.

`EmbeddingGenerator` and `EmbeddingRewriter` become thin wrappers delegating to the provider instance.

**Alternatives considered:**
- Single adapter with if/else branching: Rejected for maintainability; adding a 3rd provider would require editing multiple methods.
- Dependency injection without factory: Rejected; factory centralizes config-to-provider resolution.

### D5: Config structure with backward-compatible fallback

New `[llm]` top-level section in `config.toml`:
```toml
[llm]
provider = "gemini"  # or "openai"
api_key = "..."
base_url = "..."

[llm.models.embedding]
name = "gemini-embedding-001"
dim = 768
task_type = "SEMANTIC_SIMILARITY"
batch_size = 32

[llm.models.rewrite]
name = "gemini-2.0-flash"
temperature = 0.3
timeout = 60
max_retries = 2
workers = 8
```

Fallback chain: `[llm]` present → use `[llm]`; `[llm]` absent AND `[gemini]` present → use `[gemini]` with `provider = "gemini"` implied; both absent → error.

API key resolution per-capability: `[llm.models.<cap>].api_key` → `[llm].api_key` → environment variable (`OPENAI_API_KEY` or `GEMINI_API_KEY` based on provider).

Embedding and rewrite MAY use different providers if configured separately (mixed provider support).

**Alternatives considered:**
- Breaking change removing `[gemini]`: Rejected to avoid disrupting existing deployments.

### D6: Batch failure retry with bisection fallback

When `embed_batch()` fails:
1. Retry full batch up to `max_retries` times with exponential backoff.
2. If still failing, bisect batch into halves and retry each half recursively.
3. At batch_size=1, if still failing, record as permanent failure for that problem_id.

Maximum API calls per batch: bounded by `max_retries * log2(batch_size)` per item.

### D7: Structured summary output

Python outputs a JSON summary line with fixed prefix `EMBEDDING_SUMMARY:` on stdout upon completion:
```json
{
  "total_pending": 100,
  "succeeded": 90,
  "skipped": { "empty_content": 3, "rewrite_timeout": 2, "rewrite_error": 1 },
  "failed": { "embed_permanent": 3, "embed_transient": 1 },
  "duration_secs": 245.3
}
```

Invariant: `succeeded + sum(skipped.values()) + sum(failed.values()) == total_pending`.

Exit code: 0 if `sum(failed.values()) == 0`, else 1.

### D8: Embedding trigger API parameter validation

`POST /admin/api/embeddings/trigger` accepts typed request body:
```json
{
  "source": "leetcode",
  "rebuild": false,
  "dry_run": false,
  "batch_size": 32,
  "filter": null
}
```

Rust validates `source` against known sources (or "all"), `batch_size` within [1, 256], and `filter` as optional non-empty string. No arbitrary flag passthrough.

## Risks / Trade-offs

**[Risk] SQLite contention during rebuild** → Embedding rebuild drops and recreates `vec_embeddings` table, which blocks concurrent similarity searches. Mitigation: Admin UI shows warning when rebuild is selected; similarity API returns 503 during rebuild.

**[Risk] Progress file atomicity on crash** → If Python crashes mid-write, progress file may be stale. Mitigation: Rust reads file with fallback to "unknown" state; progress file uses atomic write (temp + rename).

**[Risk] Dimension mismatch on provider switch** → Switching from Gemini (768-dim) to OpenAI (1536-dim) without rebuild causes search failures. Mitigation: Provider validates dimension at startup; `--build` checks dimension consistency and fails fast with clear error requiring `--rebuild`.

**[Risk] In-memory job state lost on restart** → Same limitation as existing crawler infrastructure. Mitigation: Log files persist; progress file survives restart. Acceptable for admin tool.

**[Risk] Mixed provider API cost unpredictability** → Different providers have different pricing models. Mitigation: `--dry-run` estimates API call counts before execution.

**[Trade-off] Subprocess model retained** → Simpler architecture but slower job startup and limited real-time communication. Acceptable given the batch nature of embedding jobs and the shared-file progress mechanism.
