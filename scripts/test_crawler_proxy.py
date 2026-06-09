import unittest
from unittest.mock import patch

from aiohttp_socks import ProxyConnector
from utils.base_crawler import BaseCrawler
from utils.config import CrawlerHttpConfig, _validate_proxy_url


class CrawlerProxyTests(unittest.IsolatedAsyncioTestCase):
    def _crawler(self, config: CrawlerHttpConfig) -> BaseCrawler:
        with patch("utils.base_crawler.get_config") as mock_get_config:
            mock_get_config.return_value.get_crawler_config.return_value = config
            return BaseCrawler("test")

    async def test_aiohttp_socks5h_connector_uses_remote_dns(self):
        crawler = self._crawler(CrawlerHttpConfig(proxy="socks5h://127.0.0.1:1080"))

        connector = crawler._create_socks_connector("socks5h://127.0.0.1:1080")

        self.assertIsInstance(connector, ProxyConnector)
        self.assertEqual(connector._proxy_host, "127.0.0.1")
        self.assertEqual(connector._proxy_port, 1080)
        self.assertIs(connector._rdns, True)
        await connector.close()

    async def test_aiohttp_socks5h_connector_preserves_credentials(self):
        crawler = self._crawler(
            CrawlerHttpConfig(proxy="socks5h://user:pass@127.0.0.1:1080")
        )

        connector = crawler._create_socks_connector(
            "socks5h://user:pass@127.0.0.1:1080"
        )

        self.assertEqual(connector._proxy_username, "user")
        self.assertEqual(connector._proxy_password, "pass")
        self.assertIs(connector._rdns, True)
        await connector.close()

    def test_aiohttp_socks5h_request_proxy_is_connector_managed(self):
        crawler = self._crawler(CrawlerHttpConfig(proxy="socks5h://127.0.0.1:1080"))

        self.assertIsNone(crawler._get_aiohttp_request_proxy("https"))

    def test_curl_session_preserves_socks5h_proxy(self):
        crawler = self._crawler(CrawlerHttpConfig(proxy="socks5h://127.0.0.1:1080"))

        with patch("curl_cffi.requests.AsyncSession") as mock_session:
            crawler._create_curl_session(impersonate="chrome124")

        mock_session.assert_called_once_with(
            impersonate="chrome124",
            trust_env=False,
            proxies={
                "http": "socks5h://127.0.0.1:1080",
                "https": "socks5h://127.0.0.1:1080",
            },
        )

    def test_validate_proxy_url_accepts_socks5h_with_port(self):
        _validate_proxy_url("socks5h://user:pass@127.0.0.1:1080")

    def test_validate_proxy_url_rejects_socks_without_port(self):
        with self.assertRaisesRegex(ValueError, "missing port"):
            _validate_proxy_url("socks5h://127.0.0.1")


if __name__ == "__main__":
    unittest.main()
