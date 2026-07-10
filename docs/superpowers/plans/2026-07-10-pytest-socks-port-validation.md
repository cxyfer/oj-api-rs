# Pytest 與 SOCKS Proxy Port 驗證實作計畫

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 正式加入 pytest、統一 Python 測試命令，並在設定載入時拒絕 SOCKS proxy port `0`。

**Architecture:** `scripts/pyproject.toml` 與 `scripts/uv.lock` 定義可重現的 pytest 開發環境；pytest 作為唯一 Python test runner，同時收集既有 unittest 與 pytest-style tests。Proxy URL 規則維持集中在 `scripts/utils/config.py::_validate_proxy_url()`，由 config-load regression test 鎖定 port `0` 的錯誤分類。

**Tech Stack:** Python 3.10+、uv、pytest、ruff、urllib.parse、TOML

## Global Constraints

- Canonical Python test command 必須是 `uv --directory scripts run --frozen pytest`。
- SOCKS port `0` 必須在 config load 時以 `Proxy URL has invalid port` 拒絕。
- SOCKS port `1..=65535` 保持接受；缺少 port 保持 `Proxy URL missing port`。
- 不新增 `pytest-asyncio`，不修改 credentials、transport、proxy precedence 或支援 scheme。
- 不執行 git commit 或 push。

---

### Task 1: 正式加入 pytest 並統一測試入口

**Files:**
- Modify: `scripts/pyproject.toml:21-30`
- Modify: `scripts/uv.lock`
- Modify: `CLAUDE.md:15-24`

**Interfaces:**
- Consumes: uv dependency group `dev` 與 pytest 對 `unittest.TestCase` 的原生收集能力。
- Produces: 可由乾淨 scripts 環境執行的 `uv --directory scripts run --frozen pytest`。

- [ ] **Step 1: 在 dev dependency 宣告 pytest**

將 `scripts/pyproject.toml` 更新為：

```toml
[dependency-groups]
dev = ["pytest>=8.0.0", "ruff>=0.9.0"]

[tool.pytest.ini_options]
python_files = ["test_*.py"]
```

保留既有 `[tool.ruff]` 與 `[tool.ruff.format]` 設定。

- [ ] **Step 2: 同步 uv lockfile**

Run:

```bash
uv --directory scripts lock
```

Expected: `scripts/uv.lock` 加入 pytest 及其解析後依賴，command exit `0`。

- [ ] **Step 3: 驗證 frozen lock 與全量 pytest**

Run:

```bash
uv --directory scripts lock --check
uv --directory scripts run --frozen pytest
```

Expected: lock check exit `0`；pytest 能收集既有 unittest tests 與 `scripts/test_crawler_proxy.py`，不再出現 `ModuleNotFoundError: pytest`。

- [ ] **Step 4: 文件化唯一 Python 測試命令**

在根目錄 `CLAUDE.md` 的 Commands code block 中，緊接 `cargo test` 後加入：

```bash
# Python tests (runs both unittest-style and pytest-style tests)
uv --directory scripts run --frozen pytest
```

不保留或新增 `python -m unittest` 作為 canonical command。

- [ ] **Step 5: 驗證文件命令可直接執行**

Run:

```bash
uv --directory scripts run --frozen pytest
```

Expected: full Python suite exit `0`。

---

### Task 2: 以 TDD 拒絕 SOCKS proxy port 0

**Files:**
- Modify: `scripts/test_crawler_proxy.py:111-141`
- Modify: `scripts/utils/config.py:266-280`

**Interfaces:**
- Consumes: `ConfigManager(config_path: Optional[str])` 於 `_load_config()` 呼叫 `_validate_crawler_proxy_urls()`；`_validate_proxy_url(url: str) -> None`。
- Produces: SOCKS port `0` 在 config load 時拋出包含 `invalid port` 的 `ValueError`。

- [ ] **Step 1: 寫入 port 0 regression test**

在 `scripts/test_crawler_proxy.py` 的 config validation tests 後加入：

```python
def test_config_manager_rejects_socks_port_zero_at_load(tmp_path):
    path = write_config(
        tmp_path, '[crawler]\nsocks5_proxy = "socks5h://127.0.0.1:0"\n'
    )

    with pytest.raises(ValueError, match="invalid port"):
        ConfigManager(str(path))
```

- [ ] **Step 2: 執行新測試並確認先失敗**

Run:

```bash
uv --directory scripts run --frozen pytest test_crawler_proxy.py::test_config_manager_rejects_socks_port_zero_at_load -q
```

Expected: FAIL，訊息為 `Failed: DID NOT RAISE <class 'ValueError'>`。

- [ ] **Step 3: 實作最小 port 驗證**

將 `scripts/utils/config.py::_validate_proxy_url()` 的 SOCKS port 判斷更新為：

```python
    if parsed.scheme in {"socks5", "socks5h"}:
        if port is None:
            raise ValueError(f"Proxy URL missing port: '{url}'")
        if port == 0:
            raise ValueError(f"Proxy URL has invalid port: '{url}'")
```

`parsed.port` 對非數字與超過 `65535` 的值仍由既有 `except ValueError` 轉換成相同的 invalid-port 錯誤。

- [ ] **Step 4: 執行 focused regression tests**

Run:

```bash
uv --directory scripts run --frozen pytest test_crawler_proxy.py -q
```

Expected: 所有 proxy tests PASS，包含 missing port、invalid scheme、port `0` 與有效 SOCKS5H URL。

---

### Task 3: 全量品質與變更範圍驗證

**Files:**
- Verify: `scripts/pyproject.toml`
- Verify: `scripts/uv.lock`
- Verify: `scripts/test_crawler_proxy.py`
- Verify: `scripts/utils/config.py`
- Verify: `CLAUDE.md`
- Verify: `docs/superpowers/specs/2026-07-10-pytest-socks-port-validation-design.md`
- Verify: `docs/superpowers/plans/2026-07-10-pytest-socks-port-validation.md`

**Interfaces:**
- Consumes: Task 1 的 frozen pytest environment 與 Task 2 的 validation behavior。
- Produces: 可交付的未提交 working-tree diff 與完整驗證證據。

- [ ] **Step 1: 驗證 lockfile 與 lint**

Run:

```bash
uv --directory scripts lock --check
uv --directory scripts run --frozen ruff check .
```

Expected: both commands exit `0`。

- [ ] **Step 2: 執行 canonical 全量 Python tests**

Run:

```bash
uv --directory scripts run --frozen pytest
```

Expected: all collected Python tests PASS，且 `test_crawler_proxy.py` 被收集。

- [ ] **Step 3: 檢查格式**

Run:

```bash
uv --directory scripts run --frozen ruff format --check .
```

Expected: exit `0`；若只回報本次修改檔案格式問題，執行 `uv --directory scripts run --frozen ruff format test_crawler_proxy.py utils/config.py` 後重跑 check。

- [ ] **Step 4: 檢查最終 diff 與狀態**

Run:

```bash
git diff --check
git diff --stat
git diff -- scripts/pyproject.toml scripts/uv.lock scripts/test_crawler_proxy.py scripts/utils/config.py CLAUDE.md docs/superpowers/specs/2026-07-10-pytest-socks-port-validation-design.md docs/superpowers/plans/2026-07-10-pytest-socks-port-validation.md
git status --short
```

Expected: 無 whitespace errors；只有核准範圍內的 tracked changes；不建立 commit、不 push。
