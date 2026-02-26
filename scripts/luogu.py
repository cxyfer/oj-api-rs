import argparse
import asyncio
import json
import math
import os
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from bs4 import BeautifulSoup

from utils.base_crawler import BaseCrawler
from utils.config import get_config
from utils.database import ProblemsDatabaseManager
from utils.logger import get_leetcode_logger

logger = get_leetcode_logger()

CURL_IMPERSONATE = "chrome124"

RATE_LIMIT_MARKERS = (
    "too many requests",
    "just a moment...",
    "attention required",
    "captcha",
    "checking your browser",
)

DIFFICULTY_MAP = {
    0: "暂无评定",
    1: "入门",
    2: "普及−",
    3: "普及/提高−",
    4: "普及+/提高",
    5: "提高+/省选−",
    6: "省选/NOI−",
    7: "NOI/NOI+/CTSC",
}


class LuoguClient(BaseCrawler):
    PROBLEM_LIST_URL = "https://www.luogu.com.cn/problem/list"
    TAGS_URL = "https://www.luogu.com.cn/_lfe/tags/zh-CN"
    PROBLEM_URL_TEMPLATE = "https://www.luogu.com.cn/problem/{pid}"

    def _headers(self, referer: Optional[str] = None) -> dict:
        # Don't set User-Agent — let curl_cffi impersonation handle it
        # to keep UA consistent with TLS fingerprint (same as codeforces.py)
        headers: dict[str, str] = {}
        if referer:
            headers["Referer"] = referer
        return headers

    def __init__(
        self,
        data_dir: str = "data",
        db_path: str = "data/data.db",
        rate_limit: float = 2.0,
        max_retries: int = 3,
        backoff_base: float = 2.0,
        max_backoff: float = 60.0,
        batch_size: int = 10,
    ) -> None:
        super().__init__(crawler_name="luogu")
        self.data_dir = Path(data_dir)
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.progress_file = self.data_dir / "luogu_progress.json"
        self.tags_file = self.data_dir / "luogu_tags.json"
        self.problems_db = ProblemsDatabaseManager(db_path)
        self.rate_limit = max(rate_limit, 1.0)
        self.max_retries = max_retries
        self.backoff_base = backoff_base
        self.max_backoff = max_backoff
        self.batch_size = max(batch_size, 1)
        self._last_request_at = time.monotonic() - self.rate_limit

    async def _throttle(self) -> None:
        elapsed = time.monotonic() - self._last_request_at
        wait_for = self.rate_limit - elapsed
        if wait_for > 0:
            await asyncio.sleep(wait_for)
        self._last_request_at = time.monotonic()

    def _is_rate_limited(self, html: str) -> bool:
        if not html:
            return False
        text = html.lower()
        if "<title>just a moment...</title>" in text:
            return True
        if "<title>attention required" in text:
            return True
        if any(marker in text for marker in RATE_LIMIT_MARKERS):
            return True
        # Structural check: if neither lentille-context nor known data
        # containers exist AND the page is suspiciously short, treat as blocked
        if "lentille-context" not in html and len(html) < 2000:
            logger.debug("Short page without lentille-context (%d bytes)", len(html))
            return True
        return False

    async def _fetch_text(
        self, session, url: str, referer: Optional[str] = None
    ) -> Optional[str]:
        for attempt in range(1, self.max_retries + 1):
            await self._throttle()
            try:
                headers = self._headers(referer)
                response = await session.get(url, headers=headers, timeout=30)
                if response.status_code in {403, 429, 503}:
                    backoff = min(
                        self.max_backoff, self.backoff_base * (2 ** (attempt - 1))
                    )
                    logger.warning(
                        "HTTP %s from %s. Backing off %.1fs",
                        response.status_code,
                        url,
                        backoff,
                    )
                    await asyncio.sleep(backoff)
                    continue
                if response.status_code >= 400:
                    logger.warning("HTTP %s from %s", response.status_code, url)
                    return None
                text = response.text
            except asyncio.CancelledError:
                raise
            except Exception as exc:
                if attempt >= self.max_retries:
                    logger.error("Failed to fetch %s: %s", url, exc)
                    return None
                backoff = min(
                    self.max_backoff, self.backoff_base * (2 ** (attempt - 1))
                )
                logger.warning("Fetch failed (%s), retry in %.1fs", exc, backoff)
                await asyncio.sleep(backoff)
                continue
            if self._is_rate_limited(text):
                backoff = min(
                    self.max_backoff, self.backoff_base * (2 ** (attempt - 1))
                )
                logger.warning(
                    "Rate limited content detected (%s). Backing off %.1fs",
                    url,
                    backoff,
                )
                await asyncio.sleep(backoff)
                continue
            return text
        return None

    def _extract_lentille_context(self, html: str) -> Optional[dict]:
        soup = BeautifulSoup(html, "html.parser")
        for script in soup.find_all("script", {"type": "application/json"}):
            if (
                script.get("lentille-context") is not None
                or script.get("id") == "lentille-context"
            ):
                try:
                    return json.loads(script.string)
                except (json.JSONDecodeError, TypeError) as exc:
                    logger.error("Failed to parse lentille-context JSON: %s", exc)
                    return None
        logger.error("lentille-context script tag not found")
        return None

    def _save_tags(self, api_data: dict) -> None:
        tags = api_data.get("tags", [])
        payload = {
            "tags": [
                {
                    "id": t["id"],
                    "name": t["name"],
                    "type": t.get("type"),
                    "parent": t.get("parent"),
                }
                for t in tags
                if "id" in t and "name" in t
            ],
            "types": api_data.get("types", []),
            "tag_map": {
                str(t["id"]): t["name"] for t in tags if "id" in t and "name" in t
            },
            "version": api_data.get("version"),
            "last_updated": datetime.now(timezone.utc).isoformat(),
        }
        tmp_path = self.tags_file.with_suffix(".tmp")
        try:
            with tmp_path.open("w", encoding="utf-8") as f:
                json.dump(payload, f, indent=2, ensure_ascii=False)
                f.flush()
                os.fsync(f.fileno())
            tmp_path.replace(self.tags_file)
        except Exception as exc:
            logger.warning("Failed to write tags file: %s", exc)
            if tmp_path.exists():
                tmp_path.unlink(missing_ok=True)

    def _load_cached_tag_map(self) -> dict[str, str]:
        if not self.tags_file.exists():
            return {}
        try:
            with self.tags_file.open("r", encoding="utf-8") as f:
                data = json.load(f)
            return data.get("tag_map", {})
        except Exception as exc:
            logger.warning("Failed to read tags file: %s", exc)
            return {}

    async def _fetch_tags_map(self, session) -> dict[str, str]:
        await self._throttle()
        try:
            headers = self._headers(referer="https://www.luogu.com.cn/")
            response = await session.get(self.TAGS_URL, headers=headers, timeout=30)
            if response.status_code == 200:
                data = json.loads(response.text)
                tags = data.get("tags", [])
                tag_map = {
                    str(t["id"]): t["name"] for t in tags if "id" in t and "name" in t
                }
                self._save_tags(data)
                return tag_map
        except Exception as exc:
            logger.warning("Failed to fetch tags: %s", exc)
        cached = self._load_cached_tag_map()
        if cached:
            logger.info("Using cached tag_map from %s", self.tags_file.name)
            return cached
        # Legacy fallback: old progress file may still have tags_map
        progress = self.get_progress()
        legacy = progress.get("tags_map")
        if legacy:
            logger.info("Using legacy tags_map from progress file")
            return legacy
        logger.warning("No tag_map available, tags will be raw IDs")
        return {}

    @staticmethod
    def _map_difficulty(value) -> Optional[str]:
        if value is None:
            return None
        try:
            return DIFFICULTY_MAP.get(int(value))
        except (ValueError, TypeError):
            return None

    def _map_problem(self, raw: dict, tag_map: dict[str, str]) -> Optional[dict]:
        pid = raw.get("pid")
        if not pid:
            logger.warning("Skipping problem with missing pid")
            return None
        raw_tags = raw.get("tags", [])
        tags = [tag_map.get(str(t), str(t)) for t in raw_tags]
        total_submit = raw.get("totalSubmit", 0)
        total_accepted = raw.get("totalAccepted", 0)
        ac_rate = (
            round(total_accepted * 100 / total_submit, 2)
            if isinstance(total_submit, (int, float)) and total_submit > 0
            else None
        )
        return {
            "id": str(pid),
            "source": "luogu",
            "slug": str(pid),
            "title": raw.get("title", ""),
            "title_cn": raw.get("title", ""),
            "difficulty": self._map_difficulty(raw.get("difficulty")),
            "ac_rate": ac_rate,
            "rating": None,
            "contest": None,
            "problem_index": None,
            "tags": json.dumps(tags, ensure_ascii=False),
            "link": self.PROBLEM_URL_TEMPLATE.format(pid=pid),
            "category": "Algorithms",
            "paid_only": 0,
            "content": None,
            "content_cn": None,
            "similar_questions": None,
        }

    def get_progress(self) -> dict:
        if not self.progress_file.exists():
            return {
                "completed_pages": [],
                "last_completed_page": None,
                "total_count_snapshot": None,
                "last_updated": None,
            }
        try:
            with self.progress_file.open("r", encoding="utf-8") as f:
                return json.load(f)
        except Exception as exc:
            logger.warning("Failed to read progress file: %s", exc)
            return {
                "completed_pages": [],
                "last_completed_page": None,
                "total_count_snapshot": None,
                "last_updated": None,
            }

    def save_progress(
        self,
        page: int,
        total_count: Optional[int] = None,
    ) -> None:
        progress = self.get_progress()
        completed = set(progress.get("completed_pages", []))
        completed.add(str(page))
        progress["completed_pages"] = sorted(completed, key=lambda x: int(x))
        progress["last_completed_page"] = page
        progress["last_updated"] = datetime.now(timezone.utc).isoformat()
        if total_count is not None:
            progress["total_count_snapshot"] = total_count
        progress.pop("tags_map", None)
        tmp_path = self.progress_file.with_suffix(".tmp")
        try:
            with tmp_path.open("w", encoding="utf-8") as f:
                json.dump(progress, f, indent=2, ensure_ascii=False)
                f.flush()
                os.fsync(f.fileno())
            tmp_path.replace(self.progress_file)
        except Exception as exc:
            logger.warning("Failed to write progress file: %s", exc)
            try:
                if tmp_path.exists():
                    tmp_path.unlink()
            except OSError:
                pass

    async def sync(self, overwrite: bool = False) -> None:
        async with self._create_curl_session(impersonate=CURL_IMPERSONATE) as session:
            progress = self.get_progress()
            completed_pages = set(progress.get("completed_pages", []))

            # Fetch first page to determine total (also establishes session cookies)
            url = f"{self.PROBLEM_LIST_URL}?page=1"
            html = await self._fetch_text(
                session, url, referer="https://www.luogu.com.cn/"
            )
            if not html:
                logger.error("Failed to fetch first page")
                return
            ctx = self._extract_lentille_context(html)
            if not ctx:
                return
            problems_data = ctx.get("data", {}).get("problems", {})
            total_count = problems_data.get("count", 0)
            if total_count == 0:
                logger.warning("Total count is 0, nothing to sync")
                return
            total_pages = math.ceil(total_count / 50)
            logger.info("Total problems: %s, pages: %s", total_count, total_pages)

            # Fetch tags after session is established
            tag_map = await self._fetch_tags_map(session)

            # Process first page if not already done
            if "1" not in completed_pages:
                result = problems_data.get("result", [])
                mapped = [p for raw in result if (p := self._map_problem(raw, tag_map))]
                if mapped:
                    count = self.problems_db.update_problems(
                        mapped, force_update=overwrite
                    )
                    verb = "upserted" if overwrite else "inserted"
                    logger.info("Page 1: %s %s/%s problems", verb, count, len(mapped))
                self.save_progress(1, total_count=total_count)

            page = 2
            while page <= total_pages:
                if str(page) in completed_pages:
                    page += 1
                    continue
                url = f"{self.PROBLEM_LIST_URL}?page={page}"
                html = await self._fetch_text(
                    session, url, referer="https://www.luogu.com.cn/problem/list"
                )
                if not html:
                    logger.warning("Failed to fetch page %s, stopping", page)
                    break
                ctx = self._extract_lentille_context(html)
                if not ctx:
                    logger.warning("No lentille-context on page %s, stopping", page)
                    break
                problems_data = ctx.get("data", {}).get("problems", {})
                result = problems_data.get("result", [])
                if not result:
                    logger.info("Empty result on page %s, stopping", page)
                    break
                # Dynamically update total_pages
                new_count = problems_data.get("count", total_count)
                if new_count != total_count:
                    total_count = new_count
                    total_pages = math.ceil(total_count / 50)
                mapped = [p for raw in result if (p := self._map_problem(raw, tag_map))]
                if mapped:
                    count = self.problems_db.update_problems(
                        mapped, force_update=overwrite
                    )
                    verb = "upserted" if overwrite else "inserted"
                    logger.info(
                        "Page %s/%s: %s %s/%s problems",
                        page,
                        total_pages,
                        verb,
                        count,
                        len(mapped),
                    )
                self.save_progress(page)
                page += 1

        logger.info("Sync completed")

    def _compose_content_markdown(self, content: dict, samples: list) -> str:
        sections = []
        if content.get("background"):
            sections.append(f"## 题目背景\n\n{content['background']}")
        if content.get("description"):
            sections.append(f"## 题目描述\n\n{content['description']}")
        if content.get("formatI"):
            sections.append(f"## 输入格式\n\n{content['formatI']}")
        if content.get("formatO"):
            sections.append(f"## 输出格式\n\n{content['formatO']}")
        if samples:
            parts = []
            for i, sample in enumerate(samples, 1):
                inp = sample[0] if len(sample) > 0 else ""
                out = sample[1] if len(sample) > 1 else ""
                parts.append(f"### 样例输入 #{i}\n\n```\n{inp}\n```")
                parts.append(f"### 样例输出 #{i}\n\n```\n{out}\n```")
            sections.append("## 样例\n\n" + "\n\n".join(parts))
        if content.get("hint"):
            sections.append(f"## 说明/提示\n\n{content['hint']}")
        return "\n\n".join(sections)

    async def fetch_problem_content(self, session, pid: str) -> Optional[str]:
        url = self.PROBLEM_URL_TEMPLATE.format(pid=pid)
        html = await self._fetch_text(
            session, url, referer="https://www.luogu.com.cn/problem/list"
        )
        if not html:
            return None
        ctx = self._extract_lentille_context(html)
        if not ctx:
            return None
        problem = ctx.get("data", {}).get("problem", {})
        content = problem.get("content")
        if not content:
            logger.warning("No content for %s", pid)
            return None
        samples = problem.get("samples", [])
        return self._compose_content_markdown(content, samples)

    async def sync_content(self) -> None:
        missing = self.problems_db.get_problem_ids_missing_content(source="luogu")
        if not missing:
            logger.info("No problems with missing content")
            return
        logger.info("Fetching content for %s problems", len(missing))
        batch = []
        fetched = 0
        failed = False
        async with self._create_curl_session(impersonate=CURL_IMPERSONATE) as session:
            for pid in missing:
                md = await self.fetch_problem_content(session, pid)
                if md is None or md == "":
                    continue
                batch.append((md, "luogu", pid))
                fetched += 1
                if len(batch) >= self.batch_size:
                    count, ok = self.problems_db.batch_update_content(
                        batch, batch_size=self.batch_size
                    )
                    if not ok:
                        logger.warning("Some content updates failed")
                        failed = True
                    logger.info("Updated content for %s problems", count)
                    batch = []
            if batch:
                count, ok = self.problems_db.batch_update_content(
                    batch, batch_size=self.batch_size
                )
                if not ok:
                    logger.warning("Some content updates failed")
                    failed = True
                logger.info("Updated content for %s problems", count)
        if failed:
            logger.warning(
                "Content sync completed with errors, fetched %s/%s",
                fetched,
                len(missing),
            )
        else:
            logger.info("Content sync completed, fetched %s/%s", fetched, len(missing))

    def show_status(self) -> None:
        progress = self.get_progress()
        completed = progress.get("completed_pages", [])
        db_count = self.problems_db.count_missing_content(source="luogu")
        logger.info(
            "Completed pages: %s, last_completed_page: %s, "
            "total_count_snapshot: %s, last_updated: %s",
            len(completed),
            progress.get("last_completed_page"),
            progress.get("total_count_snapshot"),
            progress.get("last_updated"),
        )
        logger.info("Problems with missing content: %s", db_count)

    def show_missing_content_stats(self) -> None:
        count = self.problems_db.count_missing_content(source="luogu")
        logger.info("Missing content: %s", count)


async def main() -> None:
    parser = argparse.ArgumentParser(description="Luogu crawler")
    parser.add_argument("--sync", action="store_true", help="Sync problem list")
    parser.add_argument(
        "--fill-missing-content",
        action="store_true",
        help="Fetch content for problems missing it",
    )
    parser.add_argument(
        "--missing-content-stats",
        action="store_true",
        help="Show missing content count",
    )
    parser.add_argument("--status", action="store_true", help="Show sync status")
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing problems instead of skipping",
    )
    parser.add_argument(
        "--rate-limit",
        type=float,
        default=1.0,
        help="Seconds between requests (min 1.0)",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=10,
        help="DB write batch size for content sync (default: 10)",
    )
    parser.add_argument("--data-dir", type=str, default=None, help="Data directory")
    parser.add_argument("--db-path", type=str, default=None, help="Database path")

    args = parser.parse_args()
    do_sync_content = args.fill_missing_content

    if not (args.sync or do_sync_content or args.missing_content_stats or args.status):
        parser.print_help()
        return

    config = get_config()
    data_dir = args.data_dir or str(Path(config.database_path).resolve().parent)
    db_path = args.db_path or str(Path(config.database_path).resolve())

    client = LuoguClient(
        data_dir=data_dir,
        db_path=db_path,
        rate_limit=args.rate_limit,
        batch_size=args.batch_size,
    )

    if args.status:
        client.show_status()
    if args.sync:
        await client.sync(overwrite=args.overwrite)
    if do_sync_content:
        await client.sync_content()
    if args.missing_content_stats:
        client.show_missing_content_stats()


if __name__ == "__main__":
    asyncio.run(main())
