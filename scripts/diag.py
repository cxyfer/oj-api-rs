import argparse
import asyncio
import json
import sys

from utils.base_crawler import BaseCrawler, _DEFAULT_UA

ECHO_URL = "https://httpbin.org/get"
VALID_TARGETS = ("global", "leetcode", "atcoder", "codeforces")


class DiagCrawler(BaseCrawler):
    def __init__(self, target: str) -> None:
        super().__init__("" if target == "global" else target)
        self._target = target

    async def run(self) -> dict:
        cfg = self._http_config
        config_echo = {
            "user_agent": cfg.user_agent or _DEFAULT_UA,
            "proxy": cfg.proxy,
            "resolved_https_proxy": cfg.resolve_proxy("https"),
        }

        result = await self._probe()

        return {
            "crawler": self._target,
            "config": config_echo,
            "result": result,
        }

    async def _probe(self) -> dict:
        async with self._create_aiohttp_session() as session:
            proxy = self._get_aiohttp_request_proxy("https")
            async with session.get(
                ECHO_URL, headers=self._headers(), proxy=proxy, timeout=10
            ) as resp:
                data = await resp.json()
                return {
                    "origin": data.get("origin", ""),
                    "user_agent": data.get("headers", {}).get("User-Agent", ""),
                }


async def main() -> None:
    parser = argparse.ArgumentParser(description="Crawler diagnostic tool")
    parser.add_argument(
        "--test", required=True, choices=VALID_TARGETS, help="Target crawler config"
    )
    args = parser.parse_args()

    crawler = DiagCrawler(args.test)
    result = await crawler.run()
    json.dump(result, sys.stdout, ensure_ascii=False, indent=2)
    print()


if __name__ == "__main__":
    asyncio.run(main())
