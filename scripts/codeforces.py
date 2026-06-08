import argparse
import asyncio
import csv
import json
import os
import re
import sys
import time
from datetime import date as Date, datetime, timezone
from pathlib import Path
from typing import Optional, Union

from bs4 import BeautifulSoup
from curl_cffi.requests import AsyncSession

from utils.base_crawler import BaseCrawler
from utils.config import get_config
from utils.database import DailyChallengeDatabaseManager, ProblemsDatabaseManager
from utils.html_converter import (
    fix_relative_urls_in_soup,
    normalize_math_delimiters,
    normalize_newlines,
    table_to_markdown,
)
from utils.job_progress import append_crawler_progress
from utils.logger import get_leetcode_logger

logger = get_leetcode_logger()

# When using curl_cffi, keep headers minimal and let impersonate handle defaults
RATE_LIMIT_MARKERS = (
    "too many requests",
    "please wait",
    "captcha",
    "call limit exceeded",
    "attention required",
    "cloudflare",
)

SHEEP_RAW_URL = "https://raw.githubusercontent.com/Yawn-Sean/Daily_CF_Problems/main/daily_problems/{year}/{month}/{month_day}/problems.md"
MD_LINK_RE = re.compile(r"\[([^\]]+)\]\((https?://(?:www\.)?codeforces\.com/[^)\s]+)\)")
CF_URL_RE = re.compile(
    r"https?://(?:www\.)?codeforces\.com/"
    r"(?:(?:contest/(\d+)/problem/([A-Za-z0-9]+))|"
    r"(?:problemset/problem/(\d+)/([A-Za-z0-9]+))|"
    r"(?:gym/(\d+)/problem/([A-Za-z0-9]+)))"
)
DATE_HEADERS = {"date", "日期", "時間", "时间", "day"}
DAILY_DATE_FORMATS = (
    "%Y-%m-%d",
    "%Y/%m/%d",
    "%Y-%m-%d %H:%M:%S",
    "%Y/%m/%d %H:%M:%S",
    "%Y-%m-%dT%H:%M:%S",
)
RATING_HEADERS = {"rating", "difficulty", "難度", "难度", "分數", "分数"}


def _rating_from_text(text: str) -> Optional[int]:
    match = re.search(r"\d+", text or "")
    return int(match.group(0)) if match else None


def _problem_id_from_url(url: str) -> Optional[tuple[str, str, str, bool]]:
    match = CF_URL_RE.search(url)
    if not match:
        return None
    contest_id = match.group(1) or match.group(3) or match.group(5)
    index = match.group(2) or match.group(4) or match.group(6)
    if not contest_id or not index:
        return None
    is_gym = match.group(5) is not None
    problem_id = f"GYM{contest_id}{index}" if is_gym else f"{contest_id}{index}"
    contest = f"GYM{contest_id}" if is_gym else contest_id
    return problem_id, contest, index, is_gym


def _codeforces_links(text: str) -> list[tuple[str, str]]:
    links: list[tuple[str, str]] = []
    seen_ids: set[str] = set()
    for label, url in MD_LINK_RE.findall(text or ""):
        parsed = _problem_id_from_url(url)
        if not parsed:
            continue
        problem_id = parsed[0]
        if problem_id in seen_ids:
            continue
        links.append((label.strip(), url.strip()))
        seen_ids.add(problem_id)
    for match in CF_URL_RE.finditer(text or ""):
        url = match.group(0)
        parsed = _problem_id_from_url(url)
        if not parsed:
            continue
        problem_id = parsed[0]
        if problem_id not in seen_ids:
            links.append(("", url))
            seen_ids.add(problem_id)
    return links


def _problem_from_link(
    label: str,
    url: str,
    rating: Optional[int] = None,
    hint: Optional[str] = None,
) -> Optional[dict]:
    parsed = _problem_id_from_url(url)
    if not parsed:
        return None
    problem_id, contest, problem_index, _is_gym = parsed
    title = label.strip() or problem_id
    return {
        "id": problem_id,
        "source": "codeforces",
        "slug": problem_id,
        "title": title,
        "title_cn": "",
        "difficulty": None,
        "ac_rate": None,
        "rating": rating,
        "contest": contest,
        "problem_index": problem_index,
        "tags": [],
        "link": url,
        "category": "Algorithms",
        "paid_only": 0,
        "content": hint or None,
        "content_cn": None,
        "similar_questions": None,
    }


def parse_sheep_daily_markdown(text: str) -> list[dict]:
    problems: list[dict] = []
    for line in text.splitlines():
        stripped = line.strip()
        if (
            not stripped.startswith("|")
            or "---" in stripped
            or "Difficulty" in stripped
        ):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if len(cells) < 2:
            continue
        rating = _rating_from_text(cells[0])
        hint = cells[2] if len(cells) > 2 and cells[2] else None
        for label, url in _codeforces_links(cells[1]):
            problem = _problem_from_link(label, url, rating=rating, hint=hint)
            if problem:
                problems.append(problem)
    return problems


def _read_daily_table(path: Path) -> list[dict[str, str]]:
    text = path.read_text(encoding="utf-8-sig")
    sample = text[:2048]
    try:
        dialect = csv.Sniffer().sniff(sample, delimiters=",\t;")
    except csv.Error:
        dialect = csv.excel_tab if "\t" in sample else csv.excel
    return list(csv.DictReader(text.splitlines(), dialect=dialect))


def _parse_daily_date(value: str) -> Optional[Date]:
    value = (value or "").strip()
    if not value:
        return None
    for fmt in DAILY_DATE_FORMATS:
        try:
            return datetime.strptime(value, fmt).date()
        except ValueError:
            continue
    return None


def _row_date_matches(row: dict[str, str], date: str) -> bool:
    target_date = _parse_daily_date(date)
    if target_date is None:
        return False

    date_values = [
        value or ""
        for key, value in row.items()
        if (key or "").strip().lower() in DATE_HEADERS
    ]
    if date_values:
        return any(_parse_daily_date(value) == target_date for value in date_values)
    return any(_parse_daily_date(value or "") == target_date for value in row.values())


def _row_rating(row: dict[str, str]) -> Optional[int]:
    for key, value in row.items():
        if (key or "").strip().lower() in RATING_HEADERS:
            rating = _rating_from_text(value or "")
            if rating is not None:
                return rating
    return None


def parse_0x3f_daily_file(path: Path, date: str) -> list[dict]:
    problems: list[dict] = []
    if not path.exists():
        raise FileNotFoundError(path)
    for row in _read_daily_table(path):
        if not _row_date_matches(row, date):
            continue
        row_text = " ".join(value or "" for value in row.values())
        rating = _row_rating(row)
        for label, url in _codeforces_links(row_text):
            problem = _problem_from_link(label, url, rating=rating)
            if problem:
                problems.append(problem)
    return problems


class CodeforcesClient(BaseCrawler):
    API_BASE = "https://codeforces.com/api"
    PROBLEMSET_API = f"{API_BASE}/problemset.problems"
    CONTEST_LIST_API = f"{API_BASE}/contest.list"
    CONTEST_STANDINGS_API = f"{API_BASE}/contest.standings"
    PROBLEM_URL_TEMPLATE = "https://codeforces.com/contest/{contest_id}/problem/{index}"

    def __init__(
        self,
        data_dir: str = "data",
        db_path: str = "data/data.db",
        rate_limit: float = 3.0,
        max_retries: int = 3,
        backoff_base: float = 2.0,
        max_backoff: float = 60.0,
    ) -> None:
        super().__init__(crawler_name="codeforces")
        self.data_dir = Path(data_dir)
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.progress_file = self.data_dir / "codeforces_progress.json"
        self.problems_db = ProblemsDatabaseManager(db_path)
        self.daily_db = DailyChallengeDatabaseManager(db_path)
        self.rate_limit = max(rate_limit, 2.0)
        self.max_retries = max_retries
        self.backoff_base = backoff_base
        self.max_backoff = max_backoff
        self._last_request_at = time.monotonic() - rate_limit

    def _headers(self, referer: Optional[str] = None) -> dict:
        headers: dict[str, str] = {}
        if referer:
            headers["Referer"] = referer
        return headers

    async def _throttle(self) -> None:
        elapsed = time.monotonic() - self._last_request_at
        wait_for = self.rate_limit - elapsed
        if wait_for > 0:
            await asyncio.sleep(wait_for)
        self._last_request_at = time.monotonic()

    def _is_rate_limited(self, html: str) -> bool:
        if not html:
            return False

        # If a problem statement is present, we're likely not blocked
        if "div.problem-statement" in html or 'class="problem-statement"' in html:
            return False

        text = html.lower()
        # Check for Cloudflare challenge titles
        if "<title>attention required! | cloudflare</title>" in text:
            return True
        if "<title>just a moment...</title>" in text:
            return True

        # Only scan very short pages for markers to reduce false positives
        if len(html) < 5000:
            if "/enter" in text:
                return True
            return any(marker in text for marker in RATE_LIMIT_MARKERS)

        return False

    async def _fetch_text(
        self,
        session: AsyncSession,
        url: str,
        referer: Optional[str] = None,
    ) -> Optional[str]:
        for attempt in range(1, self.max_retries + 1):
            await self._throttle()
            try:
                headers = self._headers(referer)
                # Use impersonate="chrome124" to mimic real browser TLS fingerprints
                response = await session.get(url, headers=headers, timeout=30)

                if response.status_code in {429, 403, 503}:
                    backoff = min(
                        self.max_backoff, self.backoff_base * (2 ** (attempt - 1))
                    )
                    logger.warning(
                        "Blocked or Rate limited (%s, status=%s). Backing off %.1fs",
                        url,
                        response.status_code,
                        backoff,
                    )
                    await asyncio.sleep(backoff)
                    continue

                if response.status_code >= 400:
                    logger.warning(
                        "HTTP %s while fetching %s", response.status_code, url
                    )
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
                    "Rate limited page content detected (%s). Backing off %.1fs",
                    url,
                    backoff,
                )
                await asyncio.sleep(backoff)
                continue
            return text
        return None

    async def _fetch_json(self, session: AsyncSession, url: str) -> Optional[dict]:
        text = await self._fetch_text(session, url)
        if not text:
            return None
        try:
            return json.loads(text)
        except json.JSONDecodeError as exc:
            logger.error("Invalid JSON from %s: %s", url, exc)
            return None

    def _build_problem_from_api(
        self, problem: dict, stats: dict, contest_name: Optional[str] = None
    ) -> Optional[dict]:
        contest_id = problem.get("contestId")
        index = problem.get("index")
        title = problem.get("name")
        if contest_id is None or not index or not title:
            return None
        slug = f"{contest_id}{index}"
        return {
            "id": slug,
            "source": "codeforces",
            "slug": slug,
            "title": title,
            "title_cn": "",
            "difficulty": None,
            "ac_rate": None,
            "rating": problem.get("rating"),
            "contest": contest_name if contest_name else str(contest_id),
            "problem_index": index,
            "tags": problem.get("tags", []),
            "link": self.PROBLEM_URL_TEMPLATE.format(
                contest_id=contest_id, index=index
            ),
            "category": "Algorithms",
            "paid_only": 0,
            "content": None,
            "content_cn": None,
            "similar_questions": None,
        }

    def _serialize_tags(self, tags) -> str:
        if tags is None:
            return json.dumps([])
        if isinstance(tags, str):
            try:
                json.loads(tags)
                return tags
            except json.JSONDecodeError:
                return json.dumps([tags])
        return json.dumps(list(tags))

    def _merge_problemset(self, problems: list[dict], stats: list[dict]) -> list[dict]:
        stats_map = {
            (item.get("contestId"), item.get("index")): item for item in stats or []
        }
        merged: list[dict] = []
        for problem in problems or []:
            key = (problem.get("contestId"), problem.get("index"))
            merged_problem = self._build_problem_from_api(
                problem, stats_map.get(key, {})
            )
            if merged_problem:
                merged.append(merged_problem)
        return merged

    async def sync_problemset(self) -> list[dict]:
        async with self._create_curl_session(impersonate="chrome124") as session:
            payload = await self._fetch_json(session, self.PROBLEMSET_API)
        if not payload:
            return []
        if payload.get("status") != "OK":
            logger.warning("Problemset API error: %s", payload.get("comment"))
            return []

        result = payload.get("result") or {}
        problems = self._merge_problemset(
            result.get("problems", []), result.get("problemStatistics", [])
        )
        if not problems:
            return []

        problems_for_insert = [
            {**problem, "tags": self._serialize_tags(problem.get("tags"))}
            for problem in problems
        ]
        inserted = self.problems_db.update_problems(problems_for_insert)
        logger.info(
            "Problemset sync: %s problems fetched, %s inserted, %s skipped (existing)",
            len(problems),
            inserted,
            len(problems) - inserted,
        )
        return problems

    async def fetch_contest_list(self, include_gym: bool = False) -> list[int]:
        gym_flag = "true" if include_gym else "false"
        url = f"{self.CONTEST_LIST_API}?gym={gym_flag}"
        async with self._create_curl_session(impersonate="chrome124") as session:
            payload = await self._fetch_json(session, url)
        if not payload:
            return []
        if payload.get("status") != "OK":
            logger.warning("Contest list API error: %s", payload.get("comment"))
            return []

        contests = payload.get("result", [])
        finished = [
            contest for contest in contests if contest.get("phase") == "FINISHED"
        ]
        filtered = finished
        if not include_gym:
            filtered = [contest for contest in finished if contest.get("type") != "GYM"]
        contest_ids = [
            contest.get("id") for contest in filtered if contest.get("id") is not None
        ]
        contest_ids.sort(reverse=True)
        return contest_ids

    async def fetch_contest_problems(
        self, contest_id: int, session: AsyncSession
    ) -> list[dict]:
        url = f"{self.CONTEST_STANDINGS_API}?contestId={contest_id}"
        payload = await self._fetch_json(session, url)
        if not payload:
            return []
        if payload.get("status") != "OK":
            logger.warning(
                "Contest %s standings API error: %s", contest_id, payload.get("comment")
            )
            return []

        result = payload.get("result") or {}
        problems = result.get("problems", [])
        contest_info = result.get("contest", {})
        contest_name = contest_info.get("name")

        parsed: list[dict] = []
        for problem in problems:
            if problem.get("contestId") is None:
                problem = {**problem, "contestId": contest_id}
            built = self._build_problem_from_api(problem, {}, contest_name=contest_name)
            if built:
                parsed.append(built)
        return parsed

    def _fix_relative_urls(self, html: str, base_url: str) -> str:
        soup = BeautifulSoup(html, "html.parser")
        fix_relative_urls_in_soup(soup, base_url)
        return str(soup)

    def _clean_problem_markdown(
        self, html: str, base_url: str = "https://codeforces.com"
    ) -> str:
        if not html:
            return ""

        soup = BeautifulSoup(html, "html.parser")
        fix_relative_urls_in_soup(soup, base_url)

        # MathJax 必須在移除 script 前處理
        for script in soup.select("script[type^='math/tex']"):
            latex = script.get_text().strip()
            is_display = "mode=display" in (script.get("type") or "")
            for sibling in (script.find_previous_sibling(), script.find_next_sibling()):
                if not sibling or not getattr(sibling, "get", None):
                    continue
                classes = sibling.get("class") or []
                if any(cls.startswith("MathJax") for cls in classes):
                    sibling.decompose()
            if is_display:
                script.replace_with(f"\n$$\n{latex}\n$$\n")
            else:
                script.replace_with(f"${latex}$")

        for tag in soup.select(
            "span.MathJax, span.MathJax_Preview, div.MathJax_Display"
        ):
            tag.decompose()

        for selector in (
            ".header",
            ".ojb-overlay",
            ".html2md-panel",
            ".likeForm",
            ".monaco-editor",
            ".overlay",
        ):
            for element in soup.select(selector):
                element.decompose()

        for tag in soup.select("script, style"):
            tag.decompose()

        for sample in soup.select("div.sample-tests"):
            text = sample.get_text("\n", strip=True)
            if text:
                sample.replace_with(f"\n\n```\n{text}\n```\n\n")
            else:
                sample.decompose()

        for pre in soup.find_all("pre"):
            code = pre.get_text("\n").strip("\n")
            pre.replace_with(f"\n\n```\n{code}\n```\n\n")

        for section in soup.select("div.section-title"):
            title = section.get_text(strip=True)
            section.replace_with(f"\n\n## {title}\n")

        for section in soup.select("div.property-title"):
            title = section.get_text(strip=True)
            section.replace_with(f"**{title}**: ")

        for span in soup.select("span.tex-font-style-bf"):
            text = span.get_text(strip=True)
            span.replace_with(f"**{text}**")

        for deleted in soup.find_all("del"):
            deleted.replace_with(f"~~{deleted.get_text()}~~")

        for strong in soup.find_all("strong"):
            strong.replace_with(f"**{strong.get_text()}**")
        for em in soup.find_all("em"):
            em.replace_with(f"*{em.get_text()}*")
        for code in soup.find_all("code"):
            code.replace_with(f"`{code.get_text()}`")

        for img in soup.find_all("img", src=True):
            alt = img.get("alt") or ""
            img.replace_with(f"![{alt}]({img['src']})")
        for link in soup.find_all("a", href=True):
            text = link.get_text(strip=True) or link["href"]
            link.replace_with(f"[{text}]({link['href']})")

        for table in soup.find_all("table"):
            markdown = table_to_markdown(table)
            if markdown:
                table.replace_with(markdown)
            else:
                table.decompose()

        for br in soup.find_all("br"):
            br.replace_with("\n")

        text = soup.get_text("\n")
        text = normalize_math_delimiters(text)
        lines = [line.rstrip() for line in text.splitlines()]
        text = "\n".join(lines)
        return normalize_newlines(text).strip()

    def _extract_problem_statement(self, html: str) -> Optional[str]:
        soup = BeautifulSoup(html, "html.parser")
        statement = soup.select_one("div.problem-statement")
        if not statement:
            return None
        return self._clean_problem_markdown(
            str(statement), base_url="https://codeforces.com"
        )

    async def fetch_problem_content(
        self, session: AsyncSession, contest_id: int, index: str
    ) -> Optional[str]:
        base_url = self.PROBLEM_URL_TEMPLATE.format(contest_id=contest_id, index=index)
        referer = f"https://codeforces.com/contest/{contest_id}"
        html = await self._fetch_text(session, f"{base_url}?locale=en", referer=referer)
        if not html:
            logger.warning("Empty content while fetching %s", base_url)
            return None
        content = self._extract_problem_statement(html)
        if not content:
            logger.warning("Problem statement missing for %s", base_url)
        return content

    async def fetch_content_by_url(
        self, session: AsyncSession, url: str
    ) -> Optional[str]:
        separator = "&" if "?" in url else "?"
        html = await self._fetch_text(
            session, f"{url}{separator}locale=en", referer=url
        )
        if not html:
            return None
        if "/enter" in html.lower():
            logger.warning("Login required while fetching %s", url)
            return None
        content = self._extract_problem_statement(html)
        if not content:
            logger.warning("Problem statement missing for %s", url)
        return content

    async def fetch_single_contest(self, contest_id: int) -> int:
        async with self._create_curl_session(impersonate="chrome124") as session:
            problems = await self.fetch_contest_problems(contest_id, session)
            if not problems:
                return 0
            for problem in problems:
                content = await self.fetch_problem_content(
                    session, contest_id, problem["problem_index"]
                )
                if content:
                    problem["content"] = content
                self.problems_db.update_problem(problem)
            logger.info("Fetched contest %s: %s problems", contest_id, len(problems))
            return len(problems)

    async def fetch_all_problems(
        self, resume: bool = True, include_gym: bool = False
    ) -> int:
        contests = await self.fetch_contest_list(include_gym=include_gym)
        progress = self.get_progress() if resume else {"fetched_contests": []}
        fetched = {
            str(contest_id) for contest_id in progress.get("fetched_contests", [])
        }
        remaining = [c for c in contests if str(c) not in fetched]
        logger.info(
            "Contest list: %s available, %s fetched, %s remaining",
            len(contests),
            len(fetched),
            len(remaining),
        )
        total = 0
        async with self._create_curl_session(impersonate="chrome124") as session:
            for contest_id in contests:
                if str(contest_id) in fetched:
                    continue
                problems = await self.fetch_contest_problems(contest_id, session)
                if not problems:
                    continue
                for problem in problems:
                    content = await self.fetch_problem_content(
                        session, contest_id, problem["problem_index"]
                    )
                    if content:
                        problem["content"] = content
                    self.problems_db.update_problem(problem)
                total += len(problems)
                self.save_progress(contest_id)
                logger.info(
                    "Fetched contest %s: %s problems", contest_id, len(problems)
                )
        logger.info("Total fetched: %s problems", total)
        return total

    async def fill_missing_content(self) -> int:
        missing = self.problems_db.get_problems_missing_content(source="codeforces")
        if not missing:
            logger.info("No problems missing content.")
            return 0

        total = len(missing)
        filled = 0
        logger.info("Fetching missing content for %s problems...", total)

        async with self._create_curl_session(impersonate="chrome124") as session:
            for index, (problem_id, link) in enumerate(missing, start=1):
                content = await self.fetch_content_by_url(session, link)
                if content:
                    self.problems_db.update_problem(
                        {"id": problem_id, "source": "codeforces", "content": content}
                    )
                    filled += 1
                if index % 50 == 0 or index == total:
                    logger.info("Processed %s/%s, filled %s", index, total, filled)
        return filled

    async def reprocess_content(self) -> int:
        problems = self.problems_db.get_problem_contents(source="codeforces")
        if not problems:
            logger.info("No Codeforces problems to reprocess.")
            return 0

        total = len(problems)
        logger.info("Reprocessing content for %s Codeforces problems...", total)

        updates: list[tuple[str, str, str]] = []
        total_updated = 0
        failed = False
        batch_size = 100

        for index, (problem_id, content) in enumerate(problems, start=1):
            if not content:
                continue
            cleaned = self._clean_problem_markdown(content)
            if cleaned != content:
                updates.append((cleaned, "codeforces", problem_id))

            if len(updates) >= batch_size:
                count, ok = self.problems_db.batch_update_content(updates)
                total_updated += count
                if not ok:
                    failed = True
                updates.clear()

            if index % 50 == 0 or index == total:
                logger.info(
                    "Processed %s/%s, updated so far: %s", index, total, total_updated
                )

        if updates:
            count, ok = self.problems_db.batch_update_content(updates)
            total_updated += count
            if not ok:
                failed = True

        if failed:
            logger.warning("Some updates failed during reprocessing")
        logger.info("Reprocessed %s/%s Codeforces problems", total_updated, total)
        return total_updated

    def _daily_refs(self, problems: list[dict]) -> list[str]:
        refs: list[str] = []
        seen: set[str] = set()
        for problem in problems:
            problem_id = problem.get("id")
            if not problem_id:
                continue
            ref = f"codeforces:{problem_id}"
            if ref not in seen:
                refs.append(ref)
                seen.add(ref)
        return refs

    def _store_daily_source(
        self, date: str, daily_source: str, problems: list[dict]
    ) -> bool:
        refs = self._daily_refs(problems)
        if not refs:
            logger.warning("No parseable %s daily problems for %s", daily_source, date)
            return False
        return self.daily_db.update_daily_source(date, daily_source, problems, refs)

    @staticmethod
    def _sheep_daily_url(date: str) -> str:
        parsed = datetime.strptime(date, "%Y-%m-%d")
        return SHEEP_RAW_URL.format(
            year=f"{parsed.year:04d}",
            month=f"{parsed.month:02d}",
            month_day=f"{parsed.month:02d}{parsed.day:02d}",
        )

    async def fetch_sheep_daily(self, date: str) -> bool:
        url = self._sheep_daily_url(date)
        async with self._create_curl_session(impersonate="chrome124") as session:
            markdown = await self._fetch_text(session, url)
        if not markdown:
            logger.warning("No Sheep daily markdown for %s", date)
            return False
        problems = parse_sheep_daily_markdown(markdown)
        return self._store_daily_source(date, "sheep", problems)

    def import_0x3f_daily(self, date: str, daily_file: str | None) -> bool:
        if not daily_file:
            raise ValueError("--daily-file is required for --daily-source 0x3f")
        try:
            problems = parse_0x3f_daily_file(Path(daily_file), date)
        except FileNotFoundError:
            logger.error("0x3f daily file not found: %s", daily_file)
            return False
        return self._store_daily_source(date, "0x3f", problems)

    async def fetch_daily_source(
        self, daily_source: str, date: str, daily_file: str | None = None
    ) -> bool:
        if daily_source == "sheep":
            return await self.fetch_sheep_daily(date)
        if daily_source == "0x3f":
            return self.import_0x3f_daily(date, daily_file)
        raise ValueError(f"unsupported daily source: {daily_source}")

    def show_status(self) -> None:
        progress = self.get_progress()
        fetched = progress.get("fetched_contests", [])
        missing = self.problems_db.count_missing_content(source="codeforces")
        logger.info(
            "Progress: %s contests fetched. last_contest_id=%s last_updated=%s",
            len(fetched),
            progress.get("last_contest_id"),
            progress.get("last_updated"),
        )
        logger.info("Missing content: %s", missing)

    def get_progress(self) -> dict:
        if not self.progress_file.exists():
            return {
                "fetched_contests": [],
                "last_updated": None,
                "last_contest_id": None,
            }
        try:
            with self.progress_file.open("r", encoding="utf-8") as f:
                return json.load(f)
        except Exception as exc:
            logger.warning("Failed to read progress file: %s", exc)
            return {
                "fetched_contests": [],
                "last_updated": None,
                "last_contest_id": None,
            }

    def save_progress(self, contest_id: Union[int, str]) -> None:
        progress = self.get_progress()
        fetched = set(progress.get("fetched_contests", []))
        if contest_id is not None:
            fetched.add(str(contest_id))
        progress["fetched_contests"] = sorted(fetched)
        progress["last_contest_id"] = (
            str(contest_id) if contest_id is not None else None
        )
        progress["last_updated"] = datetime.now(timezone.utc).isoformat()

        tmp_path = self.progress_file.with_suffix(".tmp")
        try:
            with tmp_path.open("w", encoding="utf-8") as f:
                json.dump(progress, f, indent=2, sort_keys=True)
                f.flush()
                os.fsync(f.fileno())
            # Use temp file for atomic writes to avoid corrupting the progress file
            tmp_path.replace(self.progress_file)
            append_crawler_progress(f"Fetched contest {contest_id}")
        except Exception as exc:
            logger.warning("Failed to write progress file: %s", exc)
            try:
                if tmp_path.exists():
                    tmp_path.unlink()
            except OSError:
                logger.warning("Failed to cleanup temp progress file: %s", tmp_path)


async def main() -> None:
    parser = argparse.ArgumentParser(description="Codeforces CLI tool")
    parser.add_argument(
        "--sync-problemset",
        action="store_true",
        help="Sync from Codeforces problemset API",
    )
    parser.add_argument(
        "--fetch-contest",
        action="store_true",
        help="Fetch contest problems and content (resumes by default)",
    )
    parser.add_argument(
        "--fetch-all",
        action="store_true",
        help="Alias for --fetch-contest",
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="Accepted for compatibility; contest fetching resumes by default",
    )
    parser.add_argument(
        "--no-resume",
        action="store_true",
        help="Disable progress-based skipping for contest fetching",
    )
    parser.add_argument("--contest", type=int, help="Fetch a single contest by ID")
    parser.add_argument(
        "--daily-source",
        choices=("sheep", "0x3f"),
        help="Import a curated daily source",
    )
    parser.add_argument("--date", type=str, help="Daily source date, format YYYY-MM-DD")
    parser.add_argument(
        "--daily-file",
        type=str,
        help="Local CSV/TSV export for --daily-source 0x3f",
    )
    parser.add_argument("--status", action="store_true", help="Show progress status")
    parser.add_argument(
        "--fill-missing-content",
        action="store_true",
        help="Fetch missing problem content",
    )
    parser.add_argument(
        "--missing-content-stats",
        action="store_true",
        help="Show missing content count",
    )
    parser.add_argument(
        "--missing-problems",
        action="store_true",
        help="Print IDs of problems missing content",
    )
    parser.add_argument(
        "--reprocess-content",
        action="store_true",
        help="Reprocess Codeforces problem content with new cleaning rules",
    )
    parser.add_argument(
        "--include-gym",
        action="store_true",
        help="Include gym contests in contest list",
    )
    parser.add_argument(
        "--rate-limit",
        type=float,
        default=2.0,
        help="Seconds between requests (default: 2.0)",
    )
    parser.add_argument("--data-dir", type=str, default=None, help="Data directory")
    parser.add_argument("--db-path", type=str, default=None, help="Database path")

    args = parser.parse_args()
    config = get_config()
    data_dir = args.data_dir or str(Path(config.database_path).resolve().parent)
    db_path = args.db_path or str(Path(config.database_path).resolve())

    client = CodeforcesClient(
        data_dir=data_dir,
        db_path=db_path,
        rate_limit=args.rate_limit,
    )

    if not (
        args.sync_problemset
        or args.fetch_contest
        or args.fetch_all
        or args.contest
        or args.daily_source
        or args.status
        or args.fill_missing_content
        or args.missing_content_stats
        or args.missing_problems
        or args.reprocess_content
    ):
        parser.print_help()
        return

    if args.status:
        client.show_status()

    if args.sync_problemset:
        await client.sync_problemset()

    if args.fetch_contest or args.fetch_all:
        await client.fetch_all_problems(
            resume=not args.no_resume, include_gym=args.include_gym
        )

    if args.contest:
        await client.fetch_single_contest(args.contest)

    if args.daily_source:
        if not args.date:
            parser.error("--date is required with --daily-source")
        ok = await client.fetch_daily_source(
            args.daily_source, args.date, args.daily_file
        )
        if not ok:
            sys.exit(2)

    if args.fill_missing_content:
        await client.fill_missing_content()

    if args.missing_content_stats:
        count = client.problems_db.count_missing_content(source="codeforces")
        print(f"Missing content: {count}")

    if args.missing_problems:
        missing = client.problems_db.get_problems_missing_content(source="codeforces")
        for problem_id, _ in missing:
            print(problem_id)

    if args.reprocess_content:
        updated = await client.reprocess_content()
        print(f"Reprocessed content: {updated}")


if __name__ == "__main__":
    asyncio.run(main())
