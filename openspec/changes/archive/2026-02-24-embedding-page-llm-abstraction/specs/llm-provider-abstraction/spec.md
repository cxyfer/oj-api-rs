## ADDED Requirements

### Requirement: Abstract LLM provider interface
The system SHALL define an abstract `LLMProvider` base class in `scripts/embeddings/providers/base.py` with three abstract methods: `embed(text) -> list[float]`, `embed_batch(texts) -> list[list[float]]`, `rewrite(prompt) -> str`. All provider implementations SHALL implement this interface.

#### Scenario: Provider interface enforced
- **WHEN** a new provider class does not implement all three methods
- **THEN** instantiation raises `TypeError`

### Requirement: Gemini provider implementation
The system SHALL provide `GeminiProvider` in `scripts/embeddings/providers/gemini.py` that wraps the `google-genai` SDK. The implementation SHALL preserve existing retry logic (tenacity with exponential backoff for 429/503 errors). The `google-genai` SDK SHALL be lazily imported at class instantiation, not at module level.

#### Scenario: Gemini embed returns correct dimensions
- **WHEN** `GeminiProvider.embed("test text")` is called with config `dim=768`
- **THEN** returned vector has exactly 768 dimensions

#### Scenario: Gemini batch embed count matches input
- **WHEN** `GeminiProvider.embed_batch(["a", "b", "c"])` is called
- **THEN** returned list has exactly 3 vectors

#### Scenario: Gemini SDK not installed
- **WHEN** `google-genai` package is not installed AND provider is "openai"
- **THEN** system starts successfully without import error

### Requirement: OpenAI-compatible provider implementation
The system SHALL provide `OpenAICompatProvider` in `scripts/embeddings/providers/openai_compat.py` that wraps the `openai` SDK. It SHALL support custom `base_url` for third-party endpoints (local LLMs, Azure OpenAI). The `openai` SDK SHALL be lazily imported at class instantiation.

#### Scenario: OpenAI embed with custom base_url
- **WHEN** config has `provider = "openai"` and `base_url = "http://localhost:11434/v1"`
- **THEN** provider connects to the specified endpoint

#### Scenario: OpenAI rewrite via chat completion
- **WHEN** `OpenAICompatProvider.rewrite(prompt)` is called
- **THEN** it uses the chat completions API with the prompt as a user message

#### Scenario: OpenAI SDK not installed
- **WHEN** `openai` package is not installed AND provider is "gemini"
- **THEN** system starts successfully without import error

### Requirement: Provider factory resolves from config
The system SHALL provide `create_provider(config)` factory in `scripts/embeddings/providers/factory.py`. The factory SHALL read `[llm].provider` to determine which provider to instantiate, passing the resolved model configs. If `provider` value is not "gemini" or "openai", factory SHALL raise `ValueError`.

#### Scenario: Factory creates Gemini provider
- **WHEN** config has `[llm].provider = "gemini"`
- **THEN** factory returns a `GeminiProvider` instance

#### Scenario: Factory creates OpenAI provider
- **WHEN** config has `[llm].provider = "openai"`
- **THEN** factory returns an `OpenAICompatProvider` instance

#### Scenario: Factory rejects unknown provider
- **WHEN** config has `[llm].provider = "anthropic"`
- **THEN** factory raises `ValueError` with message indicating valid options

### Requirement: Provider-agnostic error types
The system SHALL define `TransientProviderError` (retryable: rate limits, temporary unavailability) and `PermanentProviderError` (not retryable: auth failure, invalid model). Each provider adapter SHALL map SDK-specific errors to these types. Retry logic SHALL only retry on `TransientProviderError`.

#### Scenario: Gemini 429 maps to TransientProviderError
- **WHEN** Gemini API returns HTTP 429
- **THEN** provider raises `TransientProviderError`

#### Scenario: OpenAI auth failure maps to PermanentProviderError
- **WHEN** OpenAI API returns HTTP 401
- **THEN** provider raises `PermanentProviderError`

#### Scenario: Retry only on transient errors
- **WHEN** `PermanentProviderError` is raised during embed_batch
- **THEN** no retry is attempted and error propagates immediately

### Requirement: Config fallback chain from [llm] to [gemini]
`ConfigManager` SHALL resolve LLM configuration with strict precedence: if `[llm]` section exists, use it; if `[llm]` is absent but `[gemini]` exists, use `[gemini]` with `provider = "gemini"` implied and emit a deprecation warning; if both absent, raise `ValueError`. The same `config.toml` SHALL always resolve to the same provider and model (deterministic).

#### Scenario: New [llm] config used
- **WHEN** config.toml has `[llm]` section with `provider = "openai"`
- **THEN** system uses OpenAI provider with settings from `[llm]`

#### Scenario: Legacy [gemini] fallback
- **WHEN** config.toml has only `[gemini]` section (no `[llm]`)
- **THEN** system uses Gemini provider with settings from `[gemini]`
- **AND** a deprecation warning is logged

#### Scenario: Both sections present
- **WHEN** config.toml has both `[llm]` and `[gemini]` sections
- **THEN** `[llm]` takes precedence and `[gemini]` is ignored

#### Scenario: Neither section present
- **WHEN** config.toml has neither `[llm]` nor `[gemini]` section
- **THEN** system raises `ValueError` with clear error message

### Requirement: API key resolution chain
API key SHALL be resolved per-capability with precedence: `[llm.models.<capability>].api_key` → `[llm].api_key` → environment variable. Environment variable names: `OPENAI_API_KEY` for openai provider, `GOOGLE_API_KEY` or `GEMINI_API_KEY` for gemini provider.

#### Scenario: Model-level API key takes precedence
- **WHEN** `[llm.models.embedding].api_key` is set AND `[llm].api_key` is also set
- **THEN** model-level key is used for embedding operations

#### Scenario: Environment variable fallback
- **WHEN** no API key in config AND `OPENAI_API_KEY` env var is set AND provider is "openai"
- **THEN** environment variable is used

#### Scenario: No API key available
- **WHEN** no API key in config or environment for the selected provider
- **THEN** provider initialization raises `ValueError` with clear error message

### Requirement: Embed output dimension validation
After every `embed()` or `embed_batch()` call, the provider SHALL validate that each returned vector has dimension equal to `config.dim`. If dimension mismatches, provider SHALL raise `PermanentProviderError` with descriptive message including expected vs actual dimensions.

#### Scenario: Dimension mismatch detected
- **WHEN** provider returns vectors with 1536 dimensions but config expects 768
- **THEN** `PermanentProviderError` is raised with message "expected dim=768, got 1536"

### Requirement: Mixed provider support for embed and rewrite
The system SHALL support using different providers for embedding and rewriting operations. Each capability (`embedding`, `rewrite`) MAY independently specify its provider, model, API key, and base_url via `[llm.models.embedding]` and `[llm.models.rewrite]` sections.

#### Scenario: Gemini for rewrite, OpenAI for embedding
- **WHEN** config has `[llm].provider = "gemini"` but `[llm.models.embedding].provider = "openai"`
- **THEN** rewrite uses Gemini, embedding uses OpenAI

### Requirement: EmbeddingGenerator and EmbeddingRewriter delegate to provider
`EmbeddingGenerator` and `EmbeddingRewriter` classes SHALL become thin wrappers that instantiate the appropriate provider via factory and delegate all LLM calls. Their public API (`embed`, `embed_batch`, `rewrite`, `rewrite_with_executor`) SHALL remain unchanged.

#### Scenario: Generator delegates embed_batch to provider
- **WHEN** `EmbeddingGenerator.embed_batch(texts)` is called
- **THEN** it delegates to `provider.embed_batch(texts)` and returns the result

#### Scenario: Existing code using EmbeddingRewriter unchanged
- **WHEN** `embedding_cli.py` creates `EmbeddingRewriter(config)` and calls `rewriter.rewrite(text)`
- **THEN** behavior is identical to pre-abstraction implementation
