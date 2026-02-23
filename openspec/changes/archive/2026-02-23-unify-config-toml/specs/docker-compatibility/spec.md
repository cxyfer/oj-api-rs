## ADDED Requirements

### Requirement: Docker config volume mount
The Docker image SHALL support configuration via volume-mounted `config.toml` at `/app/config.toml`. The `config.toml.example` SHALL be copied into the image at `/app/config.toml.example` as a reference.

#### Scenario: Volume mount config
- **WHEN** container runs with `-v ./config.toml:/app/config.toml:ro`
- **THEN** Rust reads `/app/config.toml` and Python (from `/app/scripts/`) reads `../config.toml` which resolves to `/app/config.toml`

#### Scenario: Missing config in container
- **WHEN** container runs without mounting config.toml
- **THEN** the process exits with a clear error about missing config file

### Requirement: Remove .env dependency from Docker
The Dockerfile and `.dockerignore` SHALL NOT reference `.env` files. The `CMD` SHALL NOT depend on environment variables for configuration (except optional `CONFIG_PATH` override).

#### Scenario: Docker build without .env
- **WHEN** `docker build` is run with no `.env` file present
- **THEN** build succeeds without warnings about missing .env

#### Scenario: Docker run with config only
- **WHEN** container runs with only `config.toml` mounted (no `-e` env vars)
- **THEN** the application starts and reads all config from the mounted file
