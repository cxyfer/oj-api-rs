## 1. Python embed-text error envelope

- [x] 1.1 Add a small sanitized stage-aware error envelope helper for `embedding_cli.py --embed-text`
- [x] 1.2 Wrap provider/config initialization failures and emit `stage = "config"` before non-zero exit
- [x] 1.3 Wrap query rewrite failures and emit `stage = "rewrite"` before non-zero exit
- [x] 1.4 Wrap embedding generation failures and emit `stage = "embedding"` before non-zero exit
- [x] 1.5 Preserve the existing success stdout shape with `embedding` and `rewritten`

## 2. Rust POST /api/v1/similar mapping

- [x] 2.1 Add Rust structs/parsing for the `--embed-text` success and error stdout envelopes
- [x] 2.2 Map `config`, `rewrite`, `embedding`, and `output` stages to sanitized RFC 7807 details
- [x] 2.3 Preserve generic fallback behavior for missing, invalid, or unknown non-zero subprocess output
- [x] 2.4 Log subprocess exit status, stderr, and parsed stage/kind without exposing them in public responses

## 3. Verification

- [x] 3.1 Add or update Python tests for `--embed-text` rewrite, embedding, config, and success output behavior
- [x] 3.2 Add or update Rust tests for `POST /api/v1/similar` stage-specific 502 mappings and fallback behavior
- [x] 3.3 Run focused Python and Rust tests covering the changed files
- [x] 3.4 Run formatting/lint checks for touched Rust and Python files
