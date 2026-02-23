## ADDED Requirements

### Requirement: Remove GEMINI_API_KEY env passthrough
Rust subprocess launches SHALL NOT inject `GEMINI_API_KEY` via `cmd.env()`. Python scripts SHALL read Gemini config directly from the shared `config.toml`.

#### Scenario: Embedding subprocess without env passthrough
- **WHEN** Rust spawns `embedding_cli.py --embed-text "test"`
- **THEN** the subprocess command does NOT include `GEMINI_API_KEY` in its environment overrides

#### Scenario: Python reads Gemini key from config
- **WHEN** `embedding_cli.py` runs and `config.toml` has `gemini.api_key = "key123"`
- **THEN** the script reads the key from config.toml via `ConfigManager` without relying on env vars

### Requirement: CONFIG_PATH forwarding for non-standard paths
When `CONFIG_PATH` env var is set on the Rust process, Rust SHALL forward it to Python subprocesses via `cmd.env("CONFIG_PATH", &resolved_path)`. When `CONFIG_PATH` is unset, no config-related env vars SHALL be passed.

#### Scenario: CONFIG_PATH set on parent
- **WHEN** Rust starts with `CONFIG_PATH=/custom/config.toml` and spawns a crawler subprocess
- **THEN** the child process environment includes `CONFIG_PATH=/custom/config.toml`

#### Scenario: CONFIG_PATH unset on parent
- **WHEN** Rust starts without `CONFIG_PATH` and spawns a crawler subprocess
- **THEN** the child process environment does NOT include `CONFIG_PATH`

### Requirement: Python ConfigManager respects CONFIG_PATH
The `ConfigManager` SHALL check `CONFIG_PATH` env var before falling back to the default `../config.toml` relative path. Precedence: explicit constructor arg > `CONFIG_PATH` env var > default relative path.

#### Scenario: CONFIG_PATH env var set
- **WHEN** `CONFIG_PATH=/custom/config.toml` is in the environment and `ConfigManager()` is called without explicit path
- **THEN** config is loaded from `/custom/config.toml`

#### Scenario: Explicit arg overrides CONFIG_PATH
- **WHEN** `CONFIG_PATH=/custom/config.toml` is set and `ConfigManager(config_path="/other.toml")` is called
- **THEN** config is loaded from `/other.toml`
