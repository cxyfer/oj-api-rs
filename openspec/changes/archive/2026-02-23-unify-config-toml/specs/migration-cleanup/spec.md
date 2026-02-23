## ADDED Requirements

### Requirement: Single config.toml at project root
The project SHALL use a single `config.toml` at the project root as the sole configuration source for both Rust and Python. A `config.toml.example` SHALL be provided with all keys, defaults, and comments.

#### Scenario: config.toml.example provided
- **WHEN** a user clones the repository
- **THEN** `config.toml.example` exists at the project root with all configurable keys and their default values

#### Scenario: config.toml in gitignore
- **WHEN** checking `.gitignore`
- **THEN** `config.toml` is listed (contains secrets)

### Requirement: Delete legacy config files
The migration SHALL delete `.env.example` and `scripts/config.toml`. The `.env` entry in `.gitignore` MAY be removed.

#### Scenario: Legacy files removed
- **WHEN** the migration is complete
- **THEN** `.env.example` does not exist and `scripts/config.toml` does not exist

#### Scenario: Idempotent cleanup
- **WHEN** the cleanup runs and legacy files are already absent
- **THEN** no errors occur and no filesystem changes are made

### Requirement: Update project documentation
`CLAUDE.md` and `README.md` SHALL be updated to reflect the new config.toml approach. All references to `.env`, `dotenvy`, and `ADMIN_SECRET` env var SHALL be replaced with config.toml equivalents.

#### Scenario: CLAUDE.md updated
- **WHEN** reading `CLAUDE.md` after migration
- **THEN** configuration section references `config.toml` and does not mention `.env`

#### Scenario: README.md updated
- **WHEN** reading `README.md` after migration
- **THEN** setup instructions reference `config.toml.example` and docker run uses `-v ./config.toml:/app/config.toml:ro`
