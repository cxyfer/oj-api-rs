import asyncio
from pathlib import Path

import pytest
from aiohttp_socks import ProxyConnector
from utils.base_crawler import BaseCrawler
from utils.config import ConfigManager, CrawlerHttpConfig, _validate_proxy_url


def make_crawler(
    monkeypatch: pytest.MonkeyPatch, config: CrawlerHttpConfig
) -> BaseCrawler:
    class StubConfig:
        def get_crawler_config(self, _crawler_name: str) -> CrawlerHttpConfig:
            return config

    monkeypatch.setattr("utils.base_crawler.get_config", lambda: StubConfig())
    return BaseCrawler("test")


def write_config(tmp_path: Path, content: str) -> Path:
    path = tmp_path / "config.toml"
    path.write_text(content, encoding="utf-8")
    return path


def test_aiohttp_socks5h_connector_uses_remote_dns(monkeypatch):
    async def run():
        connector = make_crawler(
            monkeypatch, CrawlerHttpConfig(proxy="socks5h://127.0.0.1:1080")
        )._create_socks_connector("socks5h://127.0.0.1:1080")

        try:
            assert isinstance(connector, ProxyConnector)
            assert connector._proxy_host == "127.0.0.1"
            assert connector._proxy_port == 1080
            assert connector._rdns is True
        finally:
            await connector.close()

    asyncio.run(run())


def test_aiohttp_socks5h_connector_preserves_credentials(monkeypatch):
    async def run():
        connector = make_crawler(
            monkeypatch, CrawlerHttpConfig(proxy="socks5h://user:pass@127.0.0.1:1080")
        )._create_socks_connector("socks5h://user:pass@127.0.0.1:1080")

        try:
            assert connector._proxy_username == "user"
            assert connector._proxy_password == "pass"
            assert connector._rdns is True
        finally:
            await connector.close()

    asyncio.run(run())


def test_aiohttp_socks5h_session_uses_remote_dns_connector(monkeypatch):
    async def run():
        session = make_crawler(
            monkeypatch, CrawlerHttpConfig(proxy="socks5h://127.0.0.1:1080")
        )._create_aiohttp_session()

        try:
            connector = session.connector
            assert isinstance(connector, ProxyConnector)
            assert connector._proxy_host == "127.0.0.1"
            assert connector._proxy_port == 1080
            assert connector._rdns is True
        finally:
            await session.close()

    asyncio.run(run())


def test_aiohttp_socks5h_request_proxy_is_connector_managed(monkeypatch):
    proxy = make_crawler(
        monkeypatch, CrawlerHttpConfig(proxy="socks5h://127.0.0.1:1080")
    )._get_aiohttp_request_proxy("https")

    assert proxy is None


def test_curl_session_preserves_socks5h_proxy(monkeypatch):
    calls = []

    def fake_async_session(**kwargs):
        calls.append(kwargs)
        return object()

    monkeypatch.setattr("curl_cffi.requests.AsyncSession", fake_async_session)

    make_crawler(
        monkeypatch, CrawlerHttpConfig(proxy="socks5h://127.0.0.1:1080")
    )._create_curl_session(impersonate="chrome124")

    assert calls == [
        {
            "impersonate": "chrome124",
            "trust_env": False,
            "proxies": {
                "http": "socks5h://127.0.0.1:1080",
                "https": "socks5h://127.0.0.1:1080",
            },
        }
    ]


def test_validate_proxy_url_accepts_socks5h_with_port():
    _validate_proxy_url("socks5h://user:pass@127.0.0.1:1080")


def test_validate_proxy_url_rejects_socks_without_port():
    with pytest.raises(ValueError, match="missing port"):
        _validate_proxy_url("socks5h://127.0.0.1")


def test_config_manager_accepts_socks5h_proxy_at_load(tmp_path):
    path = write_config(
        tmp_path, '[crawler]\nsocks5_proxy = "socks5h://127.0.0.1:1080"\n'
    )

    ConfigManager(str(path))


def test_config_manager_rejects_invalid_proxy_at_load(tmp_path):
    path = write_config(tmp_path, '[crawler]\nproxy = "not-a-url"\n')

    with pytest.raises(ValueError, match="Invalid proxy scheme"):
        ConfigManager(str(path))


def test_config_manager_rejects_per_crawler_socks_without_port_at_load(tmp_path):
    path = write_config(
        tmp_path, '[crawler.leetcode]\nsocks5_proxy = "socks5h://127.0.0.1"\n'
    )

    with pytest.raises(ValueError, match="missing port"):
        ConfigManager(str(path))
