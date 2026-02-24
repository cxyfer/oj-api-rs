# Proposal: Embedding Page + LLM Provider Abstraction

## Context

The OJ API project uses Python scripts invoked via `tokio::process::Command` for embedding/rewriting operations. Currently:
- All LLM calls are hardcoded to Gemini API format (`google-genai` SDK)
- Admin panel has no UI for managing or monitoring the embedding pipeline
- The batch embedding script has reliability issues causing incomplete vectorization

## Requirements

### R1: Admin Embedding Management Page

**Goal**: New `/admin/embeddings` page with statistics and trigger capability.

**UI Components**:
- Stats cards showing per-source counts: total problems, problems with content, already embedded, pending embedding
- Trigger button to start batch embedding (follows existing Crawler job pattern)
- Job status polling (3s interval) with progress display while running
- Job history and log viewing (reuse crawler log modal pattern)

**Constraints**:
- Must follow existing admin page conventions: Askama template extending `base.html`, vanilla JS, dark theme CSS
- Reuse existing `CRAWLER_CONFIG` / `renderArgs()` patterns in `admin.js` for embedding args (source selection, rebuild flag, etc.)
- Admin auth middleware applied (session cookie or `x-admin-secret` header)
- i18n support for all 3 languages (en, zh-TW, zh-CN)
- No search/query UI — statistics and trigger only

**Backend Endpoints**:
- `GET /admin/embeddings` → render embeddings page template
- `GET /admin/api/embeddings/stats` → JSON stats per source
- `POST /admin/api/embeddings/trigger` → start embedding job (similar to crawler trigger)
- `GET /admin/api/embeddings/status` → polling endpoint for job status
- `GET /admin/api/embeddings/{job_id}/output` → log output

**Implementation Note**: Consider reusing the existing crawler job infrastructure (`CrawlerJob`, `crawler_lock`, `crawler_history`) or creating a parallel `EmbeddingJob` mechanism. The embedding trigger invokes `uv run python3 embedding_cli.py --build --source <source>`.

### R2: Python Embedding Script Reliability Fixes

**Goal**: Ensure `embedding_cli.py --build` reliably processes ALL eligible problems.

**Issues Discovered**:

1. **Batch failure drops entire batch** (`_flush_embeddings`, line 230-234): If `generator.embed_batch()` throws, all problems in the batch (up to 32) are silently lost with only a log error. No retry, no per-item fallback.
   - Fix: Retry the failed batch. If still fails, fall back to per-item embedding with individual error tracking.

2. **Rewrite timeout silently skips** (`build_embeddings`, line 181-189): `asyncio.TimeoutError` on rewrite increments `skipped_count` but the problem ID is not logged.
   - Fix: Log the skipped problem ID. Add retry logic (at least 1 retry after timeout). Report all skipped IDs in final summary.

3. **Empty content after HTML parsing silently skips** (line 167-174): Problems with valid HTML but no extractable text are skipped without specific tracking.
   - Fix: Log these problem IDs distinctly. Include in final summary as "empty content" category.

4. **Final summary report**: After `--build` completes, print a structured summary: total processed, succeeded, skipped (by reason), failed (by reason).

**Constraints**:
- Keep existing `tenacity` retry patterns for API calls
- Do not change the producer-consumer architecture (rewrite_queue → embed_queue)
- Maintain backward compatibility of CLI arguments

### R3: LLM Provider Abstraction (Python)

**Goal**: Abstract Gemini-specific API calls into a provider pattern supporting both Gemini and OpenAI-compatible APIs.

**Architecture**:
- Base class `LLMProvider` in `scripts/utils/llm_provider.py` (or `scripts/embeddings/providers/`)
- Two concrete implementations: `GeminiProvider`, `OpenAIProvider`
- Provider selection driven by `config.toml`

**Config Structure** (new `[llm]` top-level section):
```toml
[llm]
provider = "gemini"  # or "openai"
api_key = "..."
base_url = "..."     # optional, for custom endpoints

[llm.models.embedding]
name = "gemini-embedding-001"  # or "text-embedding-3-small" for openai
dim = 768
task_type = "SEMANTIC_SIMILARITY"  # gemini-specific, ignored by openai
batch_size = 32

[llm.models.rewrite]
name = "gemini-2.0-flash"  # or "gpt-4o-mini" for openai
temperature = 0.3
timeout = 60
max_retries = 2
workers = 8
```

**Backward Compatibility**: If `[llm]` section is absent, fall back to reading `[gemini]` section with `provider = "gemini"` implied.

**Provider Interface**:
```python
class LLMProvider(ABC):
    @abstractmethod
    async def embed(self, text: str) -> list[float]: ...

    @abstractmethod
    async def embed_batch(self, texts: list[str]) -> list[list[float]]: ...

    @abstractmethod
    async def rewrite(self, prompt: str) -> str: ...
```

**Constraints**:
- Rust side does NOT change — still calls `uv run python3 embedding_cli.py ...`
- OpenAI provider must support custom `base_url` (for third-party OpenAI-compatible endpoints like local LLMs, Azure OpenAI, etc.)
- API key resolution priority: model-level → section-level → environment variables
- Environment variable names: `OPENAI_API_KEY` for openai provider, existing `GOOGLE_API_KEY` / `GEMINI_API_KEY` for gemini
- `EmbeddingRewriter` and `EmbeddingGenerator` classes become thin wrappers that delegate to the provider

## Success Criteria

1. **Admin Page**: Navigate to `/admin/embeddings`, see per-source stats (total/with-content/embedded/pending), click trigger, see polling progress, view logs after completion
2. **Reliability**: Running `--build` on a dataset where some problems previously failed now processes them. Final summary shows 0 unaccounted problems (every problem is either succeeded, skipped-with-reason, or failed-with-reason)
3. **Provider Switch**: Change `[llm].provider` from `"gemini"` to `"openai"`, set appropriate model names and API key, run `--embed-text "test"` → returns valid embedding JSON
4. **Backward Compat**: Remove `[llm]` section, keep only `[gemini]` → everything works as before

## Dependencies & Risks

- **Risk**: OpenAI embedding API returns different dimensions than Gemini (e.g., 1536 vs 768). Mitigation: `dim` is configurable per model, and dimension mismatch check already exists.
- **Risk**: Crawler lock contention — if embedding job reuses crawler lock, can't run both simultaneously. Mitigation: Use separate lock for embedding jobs.
- **Dependency**: R3 (provider abstraction) should be implemented before R1 (admin page) since the trigger mechanism invokes the Python script.
- **Dependency**: R2 (reliability fixes) can be done in parallel with R3.
