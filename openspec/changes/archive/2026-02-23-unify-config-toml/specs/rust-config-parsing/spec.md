## ADDED Requirements

### Requirement: TOML file loading
The system SHALL load configuration from a TOML file at `config.toml` relative to the process working directory. The file path SHALL be overridable via the `CONFIG_PATH` environment variable. On missing file, the system SHALL print a clear error message including the resolved path and exit with non-zero status.

#### Scenario: Default config path
- **WHEN** `CONFIG_PATH` is unset and `config.toml` exists in cwd
- **THEN** the system loads configuration from `./config.toml`

#### Scenario: CONFIG_PATH override
- **WHEN** `CONFIG_PATH=/custom/path.toml` is set and the file exists
- **THEN** the system loads configuration from `/custom/path.toml`

#### Scenario: Missing config file
- **WHEN** `config.toml` does not exist at the resolved path
- **THEN** the system prints "Configuration file not found: <resolved_path>" to stderr and exits with non-zero status

#### Scenario: Invalid TOML syntax
- **WHEN** the config file contains invalid TOML
- **THEN** the system prints a parse error with location details and exits with non-zero status

### Requirement: Nested serde deserialization with defaults
The `Config` struct SHALL use `serde::Deserialize` with nested sub-structs: `ServerConfig`, `DatabaseConfig`, `CrawlerConfig`, `EmbeddingConfig`, `LoggingConfig`. All fields SHALL have `#[serde(default)]` with values matching current hardcoded defaults. Unknown keys SHALL be silently ignored (lenient mode).

#### Scenario: Full config provided
- **WHEN** all known fields are present in config.toml
- **THEN** all fields are populated from the file values

#### Scenario: Empty config file
- **WHEN** config.toml contains only `[server]\nadmin_secret = "mysecret"`
- **THEN** all other fields use defaults: listen_addr=`0.0.0.0:3000`, database.path=`data/data.db`, database.pool_max_size=8, database.busy_timeout_ms=5000, crawler.timeout_secs=300, embedding.timeout_secs=30, embedding.over_fetch_factor=4, embedding.concurrency=4, server.graceful_shutdown_secs=10, logging.rust_log=`info`

#### Scenario: Unknown keys ignored
- **WHEN** config.toml contains `[gemini]\napi_key = "abc"` or any other unknown section/key
- **THEN** Rust deserialization succeeds without error and unknown keys are ignored

### Requirement: Rust does not parse Gemini section
The Rust `Config` struct SHALL NOT include any Gemini-related fields. The `[gemini]` section is exclusively consumed by Python scripts. The `gemini_api_key` field SHALL be removed from the Rust Config struct.

#### Scenario: Gemini section present
- **WHEN** config.toml contains a `[gemini]` section with arbitrary nested keys
- **THEN** Rust config parsing succeeds and the Gemini data is not accessible from Rust

#### Scenario: Gemini section absent
- **WHEN** config.toml has no `[gemini]` section
- **THEN** Rust config parsing succeeds identically

### Requirement: admin_secret warning on unsafe values
The system SHALL emit a high-visibility warning via `tracing::warn!` when `server.admin_secret` is empty or equals `"changeme"`. The system SHALL still start successfully.

#### Scenario: admin_secret is "changeme"
- **WHEN** config.toml has `admin_secret = "changeme"`
- **THEN** the system emits a warning containing "admin_secret" and "changeme" and starts normally

#### Scenario: admin_secret is empty
- **WHEN** config.toml has `admin_secret = ""`
- **THEN** the system emits a warning about empty admin_secret and starts normally

#### Scenario: admin_secret is a real value
- **WHEN** config.toml has `admin_secret = "s3cureP@ss"`
- **THEN** the system starts without any admin_secret warning

### Requirement: RUST_LOG env var from config
The system SHALL set the `RUST_LOG` environment variable from `logging.rust_log` before `tracing_subscriber` initialization, but only if `RUST_LOG` is not already set by the environment.

#### Scenario: RUST_LOG not set externally
- **WHEN** `RUST_LOG` env var is unset and config has `logging.rust_log = "debug"`
- **THEN** `RUST_LOG` is set to `"debug"` before tracing init

#### Scenario: RUST_LOG already set
- **WHEN** `RUST_LOG=trace` is set in environment and config has `logging.rust_log = "info"`
- **THEN** `RUST_LOG` remains `"trace"` (env takes precedence)

### Requirement: Embedding concurrency from config
The `Semaphore` for embedding request concurrency SHALL be initialized from `embedding.concurrency` (default: 4). The value SHALL be validated at startup to be in range 1..=32.

#### Scenario: Default concurrency
- **WHEN** `embedding.concurrency` is not specified
- **THEN** Semaphore is initialized with 4 permits

#### Scenario: Custom concurrency
- **WHEN** `embedding.concurrency = 8`
- **THEN** Semaphore is initialized with 8 permits

#### Scenario: Invalid concurrency zero
- **WHEN** `embedding.concurrency = 0`
- **THEN** the system prints an error about invalid concurrency range and exits

#### Scenario: Invalid concurrency too high
- **WHEN** `embedding.concurrency = 64`
- **THEN** the system prints an error about invalid concurrency range and exits

### Requirement: Remove dotenvy dependency
The `dotenvy` crate SHALL be removed from `Cargo.toml` dependencies. The `Config::from_env()` method SHALL be replaced with a TOML-based loader. The `toml` crate SHALL be added as a dependency.

#### Scenario: Build without dotenvy
- **WHEN** `cargo build` is run after the change
- **THEN** compilation succeeds without `dotenvy` in the dependency tree
