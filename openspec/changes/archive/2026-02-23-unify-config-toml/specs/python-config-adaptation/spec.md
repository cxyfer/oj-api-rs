## ADDED Requirements

### Requirement: Fixed relative path config discovery
The `ConfigManager` SHALL default to loading `../config.toml` relative to the `scripts/` working directory. The explicit `config_path` parameter SHALL remain supported for override.

#### Scenario: Default discovery from scripts directory
- **WHEN** Python script runs with cwd=`scripts/` and `../config.toml` exists
- **THEN** `ConfigManager()` loads `../config.toml` successfully

#### Scenario: Explicit path override
- **WHEN** `ConfigManager(config_path="/custom/config.toml")` is called
- **THEN** the system loads from `/custom/config.toml` regardless of cwd

#### Scenario: Config file not found
- **WHEN** `../config.toml` does not exist and no explicit path is given
- **THEN** `ConfigManager` raises `FileNotFoundError` with a message containing the resolved path

### Requirement: Key migration from llm.gemini to gemini
All config key paths SHALL migrate from `llm.gemini.*` to `gemini.*`. The `gemini_api_key` property SHALL read from `gemini.api_key`. The `get_embedding_model_config()` method SHALL read from `gemini.models.embedding`. The `get_rewrite_model_config()` method SHALL read from `gemini.models.rewrite`.

#### Scenario: gemini_api_key property
- **WHEN** config.toml has `[gemini]\napi_key = "test-key"`
- **THEN** `config.gemini_api_key` returns `"test-key"`

#### Scenario: Embedding model config
- **WHEN** config.toml has `[gemini.models.embedding]\nname = "gemini-embedding-001"\ndim = 768`
- **THEN** `config.get_embedding_model_config()` returns `EmbeddingModelConfig(name="gemini-embedding-001", dim=768, ...)`

#### Scenario: Rewrite model config
- **WHEN** config.toml has `[gemini.models.rewrite]\nname = "gemini-2.0-flash"\ntemperature = 0.3`
- **THEN** `config.get_rewrite_model_config()` returns `RewriteModelConfig(name="gemini-2.0-flash", temperature=0.3, ...)`

### Requirement: Database path resolution relative to config file
The `database_path` property SHALL resolve `database.path` relative to the config file's parent directory, not relative to the process cwd. Absolute paths SHALL remain unchanged.

#### Scenario: Relative path from project root config
- **WHEN** config.toml is at `/project/config.toml` with `database.path = "data/data.db"`
- **THEN** `config.database_path` resolves to `/project/data/data.db`

#### Scenario: Absolute path unchanged
- **WHEN** config.toml has `database.path = "/var/db/data.db"`
- **THEN** `config.database_path` returns `/var/db/data.db`

#### Scenario: Cross-runtime consistency
- **WHEN** Rust runs from `/project/` and Python runs from `/project/scripts/`, both reading `/project/config.toml` with `database.path = "data/data.db"`
- **THEN** both resolve to `/project/data/data.db`

### Requirement: GEMINI_API_KEY env var override
The `_apply_env_overrides` method SHALL map `GEMINI_API_KEY` environment variable to `gemini.api_key` in the config. This override SHALL take precedence over the file value.

#### Scenario: Env var overrides file value
- **WHEN** config.toml has `gemini.api_key = "file-key"` and `GEMINI_API_KEY=env-key` is set
- **THEN** `config.gemini_api_key` returns `"env-key"`

#### Scenario: Env var not set
- **WHEN** `GEMINI_API_KEY` is unset and config.toml has `gemini.api_key = "file-key"`
- **THEN** `config.gemini_api_key` returns `"file-key"`

## REMOVED Requirements

### Requirement: Discord bot properties
**Reason**: `discord_token`, `post_time`, `timezone`, `log_directory` properties and `get_cache_expire_seconds`, `get_llm_model_config` methods are dead code inherited from the Discord bot project.
**Migration**: No migration needed — these were never used in oj-api-rs.

### Requirement: Discord/schedule env overrides
**Reason**: `DISCORD_TOKEN`, `POST_TIME`, `TIMEZONE` mappings in `_apply_env_overrides` belong to the Discord bot project.
**Migration**: No migration needed — remove from env_mappings dict.
