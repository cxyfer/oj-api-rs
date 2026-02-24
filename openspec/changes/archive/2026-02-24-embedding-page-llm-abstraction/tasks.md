## 1. LLM Provider Abstraction — Base Infrastructure

- [x] 1.1 Create `scripts/embeddings/providers/__init__.py` package with public exports (`LLMProvider`, `TransientProviderError`, `PermanentProviderError`, `create_provider`)
- [x] 1.2 Implement `scripts/embeddings/providers/base.py` with abstract `LLMProvider` class (abstract methods: `embed`, `embed_batch`, `rewrite`) and provider-agnostic error types (`TransientProviderError`, `PermanentProviderError`)
- [x] 1.3 Implement `scripts/embeddings/providers/factory.py` with `create_provider(config, capability)` factory that reads `[llm].provider` (or per-capability override) and returns the appropriate provider instance; raise `ValueError` for unknown providers

## 2. LLM Provider Abstraction — Config Migration

- [x] 2.1 Extend `scripts/utils/config.py` `ConfigManager` to support `[llm]` section with fallback chain: `[llm]` → `[gemini]` → error; emit deprecation warning when falling back to `[gemini]`
- [x] 2.2 Add API key resolution chain: `[llm.models.<cap>].api_key` → `[llm].api_key` → env var (`OPENAI_API_KEY` / `GEMINI_API_KEY` based on provider)
- [x] 2.3 Add per-capability provider override support in config (`[llm.models.embedding].provider` can differ from `[llm].provider`)
- [x] 2.4 Update `config.toml.example` with new `[llm]` section structure and comments explaining fallback behavior

## 3. LLM Provider Abstraction — Gemini Adapter

- [x] 3.1 Implement `scripts/embeddings/providers/gemini.py` `GeminiProvider` with lazy `google-genai` import at `__init__` time
- [x] 3.2 Move existing retry logic (tenacity for 429/503) from `generator.py`/`rewriter.py` into `GeminiProvider`, mapping `errors.APIError` to `TransientProviderError`/`PermanentProviderError`
- [x] 3.3 Add dimension validation in `embed`/`embed_batch`: raise `PermanentProviderError` if returned vector dim != config dim

## 4. LLM Provider Abstraction — OpenAI-Compatible Adapter

- [x] 4.1 Implement `scripts/embeddings/providers/openai_compat.py` `OpenAICompatProvider` with lazy `openai` import at `__init__` time
- [x] 4.2 Implement `embed`/`embed_batch` using `openai.embeddings.create`, with dimension validation
- [x] 4.3 Implement `rewrite` using `openai.chat.completions.create` with the rewrite prompt as user message
- [x] 4.4 Map OpenAI SDK errors (RateLimitError → Transient, AuthenticationError → Permanent, etc.)
- [x] 4.5 Add `openai` as optional dependency in `scripts/pyproject.toml`

## 5. LLM Provider Abstraction — Delegation Wiring

- [x] 5.1 Refactor `scripts/embeddings/generator.py` `EmbeddingGenerator` to delegate `embed`/`embed_batch` to provider instance from factory; keep public API unchanged
- [x] 5.2 Refactor `scripts/embeddings/rewriter.py` `EmbeddingRewriter` to delegate `rewrite`/`rewrite_with_executor` to provider instance; keep public API unchanged
- [x] 5.3 Verify `embedding_cli.py` works unchanged with both `[llm]` and legacy `[gemini]` config

## 6. Embedding Reliability — Progress & Accounting Infrastructure

- [x] 6.1 Create `BuildReport` dataclass in `embedding_cli.py` with fields: `total_pending`, `succeeded`, `skipped` (dict[str, int]), `failed` (dict[str, int]), `duration_secs`; include `add_skipped(reason, problem_id)`, `add_failed(reason, problem_id)`, `add_succeeded()` methods
- [x] 6.2 Implement atomic progress file writer: writes `scripts/logs/{job_id}.progress.json` via temp file + `os.rename`; schema: `{ phase, rewrite_progress, embed_progress, started_at }`
- [x] 6.3 Accept `--job-id` CLI argument in `embedding_cli.py` (passed by Rust trigger); if absent, generate a default UUID

## 7. Embedding Reliability — Rewrite Worker Fixes

- [x] 7.1 Update rewrite worker to log problem_id on every failure type (timeout, API error, empty result) with categorized reason (`rewrite_timeout`, `rewrite_error`, `rewrite_empty`)
- [x] 7.2 Track empty-content-after-HTML-parsing as `empty_content` skip reason (distinct from rewrite failures); log problem_id
- [x] 7.3 Update progress file during rewrite phase with `{ done, total, skipped }` counts; ensure monotonic increments

## 8. Embedding Reliability — Batch Embed Retry & Bisection

- [x] 8.1 Replace `_flush_embeddings` with retry-then-bisect logic: retry full batch up to `max_retries` with exponential backoff; on exhaustion, bisect into halves recursively
- [x] 8.2 At batch_size=1 permanent failure, record problem_id as `embed_permanent` in `BuildReport`
- [x] 8.3 Update progress file during embedding phase with `{ done, total }` counts
- [x] 8.4 Ensure retry only triggers on `TransientProviderError`; `PermanentProviderError` propagates immediately to bisection

## 9. Embedding Reliability — Summary & Exit Code

- [x] 9.1 Output `EMBEDDING_SUMMARY:<json>` line to stdout on completion inside `finally` block; validate invariant `succeeded + sum(skipped) + sum(failed) == total_pending`
- [x] 9.2 Set exit code: 0 if `sum(failed.values()) == 0`, else 1
- [x] 9.3 Write final progress file state with `phase: "completed"` or `phase: "failed"`

## 10. Admin Embedding Page — Rust Backend

- [x] 10.1 Add `EmbeddingJob` struct to `src/models.rs` (parallel to `CrawlerJob` with `job_id`, `source`, `args`, `started_at`, `finished_at`, `status`, `stdout`, `stderr`)
- [x] 10.2 Add `embedding_lock: Mutex<Option<EmbeddingJob>>` and `embedding_history: Mutex<VecDeque<EmbeddingJob>>` to `AppState` in `src/main.rs`
- [x] 10.3 Implement `GET /admin/api/embeddings/stats` handler: query `problems`/`problem_embeddings`/`vec_embeddings` via `spawn_blocking` to return per-source `{ total, with_content, embedded, pending }`
- [x] 10.4 Implement `POST /admin/api/embeddings/trigger` handler: validate typed request (`source`, `rebuild`, `dry_run`, `batch_size`, `filter`); check `embedding_lock`; spawn subprocess with `--job-id`; return 202
- [x] 10.5 Implement `GET /admin/api/embeddings/status` handler: return current job from `embedding_lock` + read `scripts/logs/{job_id}.progress.json` for live progress; fallback to `{ "phase": "unknown" }` on missing/malformed file
- [x] 10.6 Implement `GET /admin/api/embeddings/{job_id}/output` handler: check in-memory history then fallback to log files
- [x] 10.7 Register all embedding routes in `src/admin/mod.rs` under the protected `route_layer`

## 11. Admin Embedding Page — DB Queries

- [x] 11.1 Add `src/db/embeddings.rs` with `get_embedding_stats(pool, source) -> EmbeddingStats` function querying across `problems`, `problem_embeddings`, `vec_embeddings` tables
- [x] 11.2 Register module in `src/db/mod.rs`

## 12. Admin Embedding Page — Frontend

- [x] 12.1 Create `templates/admin/embeddings.html` Askama template extending `base.html` with stats cards, trigger form (source selector, rebuild/dry-run checkboxes, batch size input), status card, and history table with log modal
- [x] 12.2 Add embedding page handler in `src/admin/pages.rs` and corresponding Askama struct
- [x] 12.3 Add `EMBEDDING_CONFIG` object and `startEmbeddingPolling` function in `static/admin.js` following existing crawler patterns; implement 3-second polling with rewrite/embed phase progress display
- [x] 12.4 Add sidebar navigation link for "Embeddings" in `templates/base.html`

## 13. Admin Embedding Page — i18n

- [x] 13.1 Add embedding-related i18n keys to `static/i18n/en.json` (~25 keys: nav, title, stats labels, trigger button, status messages, flag descriptions)
- [x] 13.2 Add corresponding keys to `static/i18n/zh-TW.json`
- [x] 13.3 Add corresponding keys to `static/i18n/zh-CN.json`

## 14. Integration Verification

- [x] 14.1 Verify `cargo build --release` succeeds with all Rust changes
- [x] 14.2 Verify `cargo clippy` passes without warnings
- [x] 14.3 Verify Python `embedding_cli.py --stats` works with legacy `[gemini]` config (backward compat)
- [x] 14.4 Verify Python `embedding_cli.py --embed-text "test"` works with new `[llm]` config
- [x] 14.5 Verify admin embeddings page renders and stats load correctly
