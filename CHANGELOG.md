# Changelog

## [0.4.0] - 2026-05-31

### Features

- **api**: add batch problem fetch endpoint for resolving up to 50 `(source, id)` pairs in one request (#19)
- **api**: add OpenAPI 3.1 generation with `/openapi.json` and Scalar UI at `/docs` (#20)
- **crawler**: unify crawler CLI operations across admin-triggered and script workflows (#25)
- **api**: add random problem endpoint with cross-platform difficulty mapping (#27)

### Bug Fixes

- **api**: change similar text search from GET query parameters to POST JSON body to avoid long-query URL limits (#26)
- **embedding**: increase example timeout defaults for longer rewrite and embedding operations
- **api**: improve similar search error reporting and validation feedback (#28)
- **ci**: stabilize nightly release creation and remove the unnecessary SHA tag from nightly Docker images

### Breaking Changes

- **api**: `GET /api/v1/similar/text?q=...` is replaced by `POST /api/v1/similar/text` with a JSON request body. Clients using text-based similar search must update the HTTP method and payload format.

### Tests

- **api**: add HTTP integration tests for public, admin, and health endpoints (#22)

### Docs

- **api**: document batch, OpenAPI, Scalar UI, random problem, and updated similar-search behavior in README, homepage docs, and OpenSpec specs (#19, #20, #26, #27, #29, #30)

### Chore

- **admin**: split admin handlers into focused modules for maintainability (#21)
- **docker**: add Docker Compose configurations for local development and GHCR deployment (#23)
- **ci**: add rolling nightly GHCR pre-release workflow
- **scripts**: bump crawler script dependencies for `google-genai` and `openai` (#31)

## [0.3.3] - 2026-04-28

### Features

- **docs**: add a public homepage plus dedicated API and MCP guide pages with locale-aware examples and expandable reference details (#17)

### Bug Fixes

- **mcp**: make MCP tool schemas more Gemini-compatible for clients that validate tool definitions strictly (#18)
- **docs**: harden public docs rendering to keep homepage and reference content stable across locales (#17)

### Docs

- **api**: expand the built-in API and MCP documentation so users can discover endpoints and setup flows directly from the site (#17)

## [0.3.2] - 2026-04-18

### Features

- **api**: add native HTTP MCP support at `/mcp`, so MCP clients can use the same server process, auth gate, and data path as the REST API (#16)

### Bug Fixes

- **mcp**: harden review follow-up helpers by tightening HTML detection, preserving SSE test payload formatting, and clarifying byte-based output truncation behavior (#16)

### Docs

- **readme**: add native HTTP MCP setup instructions for Claude Code and Codex (#16)

## [0.3.1] - 2026-04-17

### Bug Fixes

- **luogu**: fix progress logging output and apply formatting cleanup (#14)
- **api**: normalize and hydrate `similar_questions` across detail-style responses, including admin detail, daily, and resolve endpoints (#15)

### Breaking Changes

- **api**: change `similar_questions` in problem-detail responses from `string[]` to hydrated summary objects for `GET /api/v1/problems/{source}/{id}`, `GET /api/v1/daily`, `GET /api/v1/resolve/{query}`, and `GET /admin/api/problems/{source}/{id}`. Clients must now read `similar_questions[*].slug` / `title` / `link` instead of treating the field as a plain slug array.

### Docs

- **api**: document the hydrated `similar_questions` response contract in README and OpenSpec specs
- **changelog**: explicitly record the `similar_questions` response-schema change as a breaking API update

## [0.3.0] - 2026-03-18

### Features

- **admin**: add cancel controls for crawler and embedding jobs, with shutdown cleanup to prevent orphaned subprocesses (#10)
- **admin**: unify retained crawler and embedding job logs with live multi-stream viewing and `python.log` support (#12)

### Bug Fixes

- **luogu**: avoid prematurely marking the final page as complete during sync, and retry it when total page count grows (#11)
- **admin**: harden job progress persistence, log polling, and embedding terminal-progress visibility during retained job recovery (#12)

### Docs

- **readme**: refresh platform status and include `luogu.py` in the crawler script list

## [0.2.1] - 2026-03-01

### Features

- **daily**: add LeetCode CN daily challenge support (#4)
- **spoj**: add SPOJ source crawling and Luogu training list support (#5)
- **api**: add natural sort ordering for OJ problem IDs (#7)
- **daily**: replace `?wait=true` with `?async=true`; waiting is now the default behavior (#8, #9)

### Tests

- **detect**: add comprehensive test coverage for source detection (#6)

### Chore

- **style**: apply `cargo fmt` formatting and fix `cargo clippy` warnings

## [0.2.0] - 2026-02-26

### Features

- **luogu**: add Luogu as a new online judge source with full crawler support (#1)
- **admin**: add 8-tier Luogu difficulty badges with official color scheme and source-aware dynamic filter dropdown (#3)
- **crawler**: add `Cancelled` job status with race-condition-safe cancel flow (#2)
- **api**: add `POST /admin/api/crawlers/cancel` and `/admin/api/embeddings/cancel` endpoints (#2)
- **config**: add `embedding.batch_timeout_secs` independent timeout option (default 600s) (#2)

### Bug Fixes

- **crawler**: spawn subprocesses in dedicated process groups via `setpgid(0,0)`; kill entire pgid on timeout/cancel to prevent orphaned child processes (#2)
- **crawler**: add PID safety guard (reject pid ≤ 1) and `ESRCH` handling for already-exited processes (#2)
- **admin**: fix plain-text problem content rendering with `white-space: pre-wrap` (#3)
- **admin**: fix difficulty dropdown option text visibility on dark theme (#3)
- **admin**: fix NOI/NOI+/CTSC badge readability (solid `#0e1d69` background + white text) (#3)
- **admin**: remove unused rating column from problems table (#3)

### Docs

- **readme**: document missing API endpoints and query params
- **config**: document `embedding.batch_timeout_secs` in `config.toml.example` (#2)

### Chore

- **style**: apply `cargo fmt`, `cargo clippy`, and `ruff` formatting fixes

## [0.1.4] - 2026-02-25

### Bug Fixes

- **docker**: remove `embeddings/` from `.dockerignore` to fix `ModuleNotFoundError` in container

### Chore

- **docker**: set `PYTHONPATH` to fix `ModuleNotFoundError` for local scripts
- **scripts**: format Python code with ruff
- **src**: format Rust code with `cargo fmt` and fix `cargo clippy` warnings

### Docs

- **readme**: add `--restart` and `--name` flags to `docker run` example
- **readme**: add development section with ruff usage
- **readme**: add rust development section with cargo commands

## [0.1.3] - 2026-02-25

### Chore

- **server**: change default listen port from 3000 to 7856

## [0.1.2] - 2026-02-25

### Features

- **embedding**: add LLM provider abstraction with Gemini and OpenAI adapters
- **admin**: add embedding management page with stats, trigger, and progress
- **api**: wrap similar endpoints with `rewritten_query` field
- **admin**: show dual progress bars for embedding pipeline

### Bug Fixes

- **embedding**: ensure progress JSON reflects final status after job completion

### Refactor

- **embedding**: remove rust-side timeout for embedding trigger

## [0.1.1] - 2026-02-24

### Features

- **api**: add `GET /status` endpoint with per-platform stats
- **crawler**: unify proxy and user-agent config via `BaseCrawler`
- **diag**: add crawler diagnostic script for UA and proxy verification

### Bug Fixes

- **resolve**: resolve LeetCode slug to numeric ID via DB lookup
- **similar**: accept `?q=` alias and strip surrounding quotes
- **i18n**: add missing `sources` object to zh-CN locale

## [0.1.0] - Initial Release
