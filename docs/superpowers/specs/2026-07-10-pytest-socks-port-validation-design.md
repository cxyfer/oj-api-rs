# Pytest 與 SOCKS Proxy Port 驗證設計

## 背景

`test_crawler_proxy.py` 已改用 pytest fixtures，但 `scripts` 專案沒有宣告 pytest，導致乾淨環境中的既有 `unittest` 命令無法匯入該測試模組。另一方面，新增的 SOCKS proxy URL 驗證只拒絕缺少 port，仍接受無法作為 outbound proxy endpoint 的 port `0`。

## 目標

- 將 pytest 納入 `scripts` 的正式 dev dependency。
- 將 Python 測試統一為 `uv --directory scripts run --frozen pytest`。
- 讓 pytest 同時收集既有 `unittest.TestCase` 與新的 pytest-style tests。
- 在設定載入時拒絕 SOCKS proxy port `0`。
- 維持其他 proxy scheme、credentials 與 crawler transport 行為不變。

## 設計

### Dependency 與測試命令

在 `scripts/pyproject.toml` 的 `dev` dependency group 加入 pytest，並以 `uv lock` 同步 `scripts/uv.lock`。不加入 `pytest-asyncio`，因目前 async regression tests 使用 `asyncio.run()`，沒有 async pytest plugin 需求。

在 pytest 設定中維持 `test_*.py` 收集規則，使既有 unittest 測試可由 pytest 原生相容層執行。根目錄 `CLAUDE.md` 的 Commands 區加入唯一 canonical Python 測試命令：

```bash
uv --directory scripts run --frozen pytest
```

Rust 的 `cargo test` 命令不變。

### SOCKS port 驗證

在 `scripts/utils/config.py::_validate_proxy_url()` 保留現有錯誤分類：

- 無 port：`Proxy URL missing port`
- parser 判定 port 超出範圍或非數字：`Proxy URL has invalid port`
- SOCKS port `0`：`Proxy URL has invalid port`
- SOCKS port `1..=65535`：接受

本次僅收緊 `socks5` 與 `socks5h`，不新增 HTTP/HTTPS port 必填規則。

### Regression test

在 `scripts/test_crawler_proxy.py` 新增 config-load 測試：`socks5h://127.0.0.1:0` 必須在 `ConfigManager` 初始化時以 `invalid port` 拒絕。既有 missing-port 測試繼續驗證原本的錯誤訊息。

## 驗證

執行：

```bash
uv --directory scripts lock --check
uv --directory scripts run --frozen ruff check .
uv --directory scripts run --frozen pytest
```

最後確認所有既有 unittest 與新增 pytest tests 都被收集、worktree 僅包含預期檔案修改，且未建立 commit 或 push。

## 非目標

- 不修改 SOCKS credentials 的空值表示；已驗證目前 `python-socks` 對 `None` 與空字串都只宣告 anonymous authentication。
- 不增加 live proxy integration test。
- 不改變 crawler transport 選擇、proxy precedence 或支援的 scheme。
