# Config Token Fallback Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve Tencent Docs token precedence from `config.toml` while repairing the Rust resolver and synchronizing tests with the supported environment fallback behavior.

**Architecture:** Both runtimes resolve the same two sources in order: a trimmed non-empty `daily_sources.tencent_docs.token`, then a trimmed non-empty environment variable named by `token_env`. A blank `token_env` intentionally disables environment fallback; it is not a configuration error. The Rust API uses the resolver to decide whether the 0x3f fallback is eligible, and the Python crawler uses its matching resolver to obtain the token.

**Tech Stack:** Rust, serde, TOML, Python 3, unittest, uv.

## Global Constraints

- Preserve the precedence `token` → `token_env` environment variable.
- Trim whitespace from configured and environment token values before use.
- Treat a blank `token_env` as an explicitly disabled environment fallback.
- Do not log or expose the token value.
- Make no unrelated daily-source behavior changes.

---

## File Structure

- `src/config.rs`: defines the Rust Tencent Docs source config, resolver, and unit tests used by the API fallback eligibility check.
- `scripts/test_daily_challenge_storage.py`: verifies matching Python resolution semantics and current MCP exception messages.

### Task 1: Repair Rust token resolution

**Files:**
- Modify: `src/config.rs:120-135`
- Test: `src/config.rs:263-311`

**Interfaces:**
- Consumes: `TencentDocsDailySourceConfig { token: String, token_env: String }`.
- Produces: `pub fn resolve_token(&self) -> Option<String>` returning the trimmed config token first, then the trimmed environment value, then `None`.

- [ ] **Step 1: Write the failing Rust regression test for blank environment fallback**

Add this test after `daily_sources_direct_token_does_not_require_environment_fallback`:

```rust
#[test]
fn daily_sources_blank_token_env_disables_environment_fallback() {
    let config: Config = toml::from_str(
        "[daily_sources.tencent_docs]\ntoken = \"  \"\ntoken_env = \"  \"\n",
    )
    .unwrap();

    assert_eq!(config.daily_sources.tencent_docs.resolve_token(), None);
}
```

- [ ] **Step 2: Run the focused Rust tests and confirm the current compilation failure**

Run:

```bash
cargo test config::tests --lib
```

Expected: FAIL with `no method named ok` at `src/config.rs:133` and `no method named has_token_source` in the tests.

- [ ] **Step 3: Implement the minimal resolver and remove obsolete source-presence assertions**

Replace `resolve_token` with:

```rust
pub fn resolve_token(&self) -> Option<String> {
    let token = self.token.trim();
    if !token.is_empty() {
        return Some(token.to_string());
    }

    let token_env = self.token_env.trim();
    if token_env.is_empty() {
        return None;
    }

    std::env::var(token_env)
        .ok()
        .and_then(|token| (!token.trim().is_empty()).then(|| token.trim().to_string()))
}
```

In `daily_sources_direct_token_does_not_require_environment_fallback`, remove:

```rust
assert!(config.daily_sources.tencent_docs.has_token_source());
```

In `daily_sources_without_direct_token_or_environment_fallback_is_invalid`, rename it to `daily_sources_without_direct_token_or_environment_fallback_returns_none` and replace its assertion with:

```rust
assert_eq!(config.daily_sources.tencent_docs.resolve_token(), None);
```

- [ ] **Step 4: Run the focused Rust tests**

Run:

```bash
cargo test config::tests --lib
```

Expected: PASS; the config test module compiles and all its tests pass.

- [ ] **Step 5: Commit the Rust resolver repair**

```bash
git add src/config.rs
git commit -m "🐛 fix(config): preserve token environment fallback"
```

### Task 2: Synchronize Python behavior tests

**Files:**
- Modify: `scripts/test_daily_challenge_storage.py:41-44`
- Modify: `scripts/test_daily_challenge_storage.py:503-509`
- Test: `scripts/test_daily_challenge_storage.py:23-68`

**Interfaces:**
- Consumes: `ConfigManager.tencent_docs_token_env` and `ConfigManager.resolve_tencent_docs_token()`.
- Produces: test coverage confirming blank `token_env` disables fallback and the error assertions reflect `daily_source.extract_tencent_docs_csv()`.

- [ ] **Step 1: Update the blank `token_env` test to state the supported compatibility contract**

Replace `test_empty_token_env_is_rejected` with:

```python
def test_empty_token_env_disables_environment_fallback(self):
    config = self._config('[daily_sources.tencent_docs]\ntoken_env = "  "\n')
    self.assertEqual(config.tencent_docs_token_env, "")
    self.assertIsNone(config.resolve_tencent_docs_token())
```

- [ ] **Step 2: Update the MCP exception assertions to match the implementation**

Replace the assertions in `test_extract_tencent_docs_csv_raises_on_error_payloads` with:

```python
with self.assertRaisesRegex(ValueError, "JSON-RPC request failed"):
    extract_tencent_docs_csv({"error": {"code": -32600, "message": "bad"}})
with self.assertRaisesRegex(ValueError, "MCP tool request failed"):
    extract_tencent_docs_csv(
        {"result": {"structuredContent": {"error": "permission denied"}}}
    )
```

- [ ] **Step 3: Run focused Python tests**

Run:

```bash
cd scripts && uv run python3 -m unittest test_daily_challenge_storage.TencentDocsConfigTests test_daily_challenge_storage.DailyChallengeStorageTests.test_extract_tencent_docs_csv_raises_on_error_payloads
```

Expected: PASS with all selected tests successful.

- [ ] **Step 4: Run formatting and the full targeted validation suite**

Run:

```bash
uv run ruff format --check test_daily_challenge_storage.py utils/config.py daily_source.py
uv run ruff check test_daily_challenge_storage.py utils/config.py daily_source.py
cd .. && cargo fmt --check && cargo test config::tests --lib
```

Expected: all commands exit 0.

- [ ] **Step 5: Commit the synchronized Python tests**

```bash
git add scripts/test_daily_challenge_storage.py
git commit -m "🧪 test(scripts): align token fallback coverage"
```

## Self-Review

- Spec coverage: Task 1 implements and tests config-first, environment-fallback behavior in Rust; Task 2 confirms the matching Python semantics and repairs stale exception assertions.
- Placeholder scan: no placeholders or deferred requirements remain.
- Type consistency: Rust exposes `resolve_token(&self) -> Option<String>`; Python exposes `resolve_tencent_docs_token() -> Optional[str]`; all planned callers and tests use these existing signatures.
