from __future__ import annotations

from typing import Any, Optional

import aiohttp
from aiohttp_socks import ProxyConnector

from .config import CrawlerHttpConfig, get_config

_DEFAULT_UA = "Mozilla/5.0 (compatible; OJ-API-Bot/1.0)"


class BaseCrawler:
    def __init__(self, crawler_name: str) -> None:
        self._crawler_name = crawler_name
        self._http_config: CrawlerHttpConfig = get_config().get_crawler_config(
            crawler_name
        )

    def _headers(self, referer: Optional[str] = None) -> dict:
        ua = self._http_config.user_agent or _DEFAULT_UA
        headers: dict[str, str] = {"User-Agent": ua}
        if referer:
            headers["Referer"] = referer
        return headers

    def _create_aiohttp_session(self, **kwargs: Any) -> aiohttp.ClientSession:
        kwargs.setdefault("trust_env", False)
        proxy_url = self._http_config.resolve_proxy("https")
        if proxy_url and proxy_url.startswith(("socks5://", "socks5h://")):
            kwargs["connector"] = ProxyConnector.from_url(proxy_url)
        return aiohttp.ClientSession(**kwargs)

    def _get_aiohttp_request_proxy(self, scheme: str = "https") -> Optional[str]:
        proxy_url = self._http_config.resolve_proxy(scheme)
        if proxy_url and proxy_url.startswith(("socks5://", "socks5h://")):
            return None
        return proxy_url

    def _create_curl_session(self, **kwargs: Any) -> Any:
        from curl_cffi.requests import AsyncSession

        kwargs.setdefault("trust_env", False)
        proxies: dict[str, str] = {}
        for scheme in ("http", "https"):
            url = self._http_config.resolve_proxy(scheme)
            if url:
                proxies[scheme] = url
        if proxies:
            kwargs.setdefault("proxies", proxies)
        return AsyncSession(**kwargs)
