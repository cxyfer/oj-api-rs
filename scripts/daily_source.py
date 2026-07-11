import argparse
import asyncio
import csv
import io
import json
import re
import sys
from datetime import date as Date, datetime
from pathlib import Path
from typing import Any, Optional

from atcoder import AtCoderClient
from codeforces import CodeforcesClient
from leetcode import LeetCodeClient
from luogu import LuoguClient
from utils.base_crawler import BaseCrawler
from utils.config import get_config
from utils.database import DailyChallengeDatabaseManager, ProblemsDatabaseManager
from utils.logger import get_leetcode_logger

logger = get_leetcode_logger()

SHEEP_RAW_URL = "https://raw.githubusercontent.com/Yawn-Sean/Daily_CF_Problems/main/daily_problems/{year}/{month}/{month_day}/problems.md"
TENCENT_DOCS_MCP_URL = "https://docs.qq.com/openapi/mcp"
TENCENT_DOCS_0X3F_FILE_ID = "DWGFoRGVZRmxNaXFz"
TENCENT_DOCS_0X3F_SHEET_ID = "BB08J2"
TENCENT_DOCS_0X3F_SHEET_NAME = "🎈算法趣题"

MD_LINK_RE = re.compile(r"\[([^\]]+)\]\((https?://[^)\s]+)\)")
CF_URL_RE = re.compile(
    r"https?://(?:www\.)?codeforces\.com/"
    r"(?:(?:contest/(\d+)/problem/([A-Za-z0-9]+))|"
    r"(?:problemset/problem/(\d+)/([A-Za-z0-9]+))|"
    r"(?:gym/(\d+)/problem/([A-Za-z0-9]+)))"
)
LEETCODE_URL_RE = re.compile(
    r"https?://(?:www\.)?leetcode\.(?:com|cn)/(?:contest/[^/]+/)?problems/([A-Za-z0-9-]+)/?"
)
ATCODER_URL_RE = re.compile(
    r"https?://atcoder\.jp/contests/([^/\s)]+)/tasks/([^/\s)]+)"
)
LUOGU_URL_RE = re.compile(
    r"https?://(?:www\.)?luogu\.com(?:\.cn)?/problem/([A-Za-z0-9_]+)"
)
DAILY_SOURCE_URL_RE = re.compile(
    r"https?://(?:www\.)?(?:"
    r"leetcode\.(?:com|cn)/(?:contest/[^/]+/)?problems/[A-Za-z0-9-]+/?|"
    r"atcoder\.jp/contests/[^/\s)]+/tasks/[^/\s)]+|"
    r"codeforces\.com/(?:contest/\d+/problem/[A-Za-z0-9]+|"
    r"problemset/problem/\d+/[A-Za-z0-9]+|gym/\d+/problem/[A-Za-z0-9]+)|"
    r"luogu\.com(?:\.cn)?/problem/[A-Za-z0-9_]+)"
)
DATE_HEADERS = {"date", "日期", "時間", "时间", "day"}
DAILY_DATE_FORMATS = (
    "%Y-%m-%d",
    "%Y/%m/%d",
    "%Y年%m月%d日",
    "%Y-%m-%d %H:%M:%S",
    "%Y/%m/%d %H:%M:%S",
    "%Y-%m-%dT%H:%M:%S",
)
RATING_HEADERS = {"rating", "difficulty", "難度", "难度", "分數", "分数"}


def _rating_from_text(text: str) -> Optional[int]:
    match = re.search(r"\d+", text or "")
    return int(match.group(0)) if match else None


def _codeforces_parts(url: str) -> Optional[tuple[str, str, str]]:
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
    return problem_id, contest, index


def _problem_snapshot(
    *,
    source: str,
    problem_id: str,
    slug: str,
    title: str,
    link: str,
    rating: Optional[int] = None,
    contest: Optional[str] = None,
    problem_index: Optional[str] = None,
    hint: Optional[str] = None,
) -> dict:
    return {
        "id": problem_id,
        "source": source,
        "slug": slug,
        "title": title,
        "title_cn": "",
        "difficulty": None,
        "ac_rate": None,
        "rating": rating,
        "contest": contest,
        "problem_index": problem_index,
        "tags": [],
        "link": link,
        "category": "Algorithms",
        "paid_only": 0,
        "content": hint or None,
        "content_cn": None,
        "similar_questions": None,
    }


def _problem_from_url(
    label: str,
    url: str,
    rating: Optional[int] = None,
    hint: Optional[str] = None,
) -> Optional[dict]:
    title_hint = label.strip()

    if parsed := _codeforces_parts(url):
        problem_id, contest, problem_index = parsed
        return _problem_snapshot(
            source="codeforces",
            problem_id=problem_id,
            slug=problem_id,
            title=title_hint or problem_id,
            link=url,
            rating=rating,
            contest=contest,
            problem_index=problem_index,
            hint=hint,
        )

    if match := LEETCODE_URL_RE.search(url):
        slug = match.group(1)
        return _problem_snapshot(
            source="leetcode",
            problem_id=slug,
            slug=slug,
            title=title_hint or slug.replace("-", " ").title(),
            link=url,
            rating=rating,
            hint=hint,
        )

    if match := ATCODER_URL_RE.search(url):
        contest, task_id = match.groups()
        task_id = task_id.lower()
        return _problem_snapshot(
            source="atcoder",
            problem_id=task_id,
            slug=task_id,
            title=title_hint or task_id,
            link=url,
            rating=rating,
            contest=contest,
            hint=hint,
        )

    if match := LUOGU_URL_RE.search(url):
        pid = match.group(1).upper()
        return _problem_snapshot(
            source="luogu",
            problem_id=pid,
            slug=pid,
            title=title_hint or pid,
            link=url,
            rating=rating,
            hint=hint,
        )

    return None


def _daily_source_links(text: str) -> list[tuple[str, str]]:
    links: list[tuple[str, str]] = []
    seen_urls: set[str] = set()
    for label, url in MD_LINK_RE.findall(text or ""):
        if not _problem_from_url(label, url):
            continue
        normalized = url.strip()
        if normalized in seen_urls:
            continue
        links.append((label.strip(), normalized))
        seen_urls.add(normalized)
    for match in DAILY_SOURCE_URL_RE.finditer(text or ""):
        url = match.group(0).strip()
        if url not in seen_urls:
            links.append(("", url))
            seen_urls.add(url)
    return links


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
        for label, url in _daily_source_links(cells[1]):
            problem = _problem_from_url(label, url, rating=rating, hint=hint)
            if problem:
                problems.append(problem)
    return _dedupe_problems(problems)


def _read_daily_table_text(text: str) -> list[dict[str, str]]:
    sample = text[:2048]
    try:
        dialect = csv.Sniffer().sniff(sample, delimiters=",\t;")
    except csv.Error:
        dialect = csv.excel_tab if "\t" in sample else csv.excel
    return list(csv.DictReader(io.StringIO(text), dialect=dialect))


def _read_daily_table(path: Path) -> list[dict[str, str]]:
    return _read_daily_table_text(path.read_text(encoding="utf-8-sig"))


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


def _parse_0x3f_daily_rows(rows: list[dict[str, str]], date: str) -> list[dict]:
    problems: list[dict] = []
    for row in rows:
        if not _row_date_matches(row, date):
            continue
        row_text = " ".join(value or "" for value in row.values())
        rating = _row_rating(row)
        for label, url in _daily_source_links(row_text):
            problem = _problem_from_url(label, url, rating=rating)
            if problem:
                problems.append(problem)
    return _dedupe_problems(problems)


def parse_0x3f_daily_csv(text: str, date: str) -> list[dict]:
    return _parse_0x3f_daily_rows(_read_daily_table_text(text), date)


def parse_0x3f_daily_file(path: Path, date: str) -> list[dict]:
    if not path.exists():
        raise FileNotFoundError(path)
    return _parse_0x3f_daily_rows(_read_daily_table(path), date)


def _dedupe_problems(problems: list[dict]) -> list[dict]:
    deduped: list[dict] = []
    seen: set[str] = set()
    for problem in problems:
        problem_id = problem.get("id")
        source = problem.get("source")
        if not problem_id or not source:
            continue
        key = f"{source}:{problem_id}"
        if key in seen:
            continue
        deduped.append(problem)
        seen.add(key)
    return deduped


def _strip_csv_fence(text: str) -> str:
    stripped = (text or "").strip()
    if stripped.startswith("```csv") and stripped.endswith("```"):
        return stripped[6:-3].strip()
    if stripped.startswith("```") and stripped.endswith("```"):
        return stripped[3:-3].strip()
    return stripped


def tencent_docs_mcp_structured_content(payload: dict[str, Any]) -> dict[str, Any]:
    if payload.get("error"):
        raise ValueError("Tencent Docs MCP JSON-RPC request failed")
    result = payload.get("result") or {}
    structured = result.get("structuredContent")
    if isinstance(structured, dict):
        return structured

    texts = [
        item.get("text", "")
        for item in result.get("content") or []
        if isinstance(item, dict) and item.get("type") == "text"
    ]
    text = "\n".join(texts).strip()
    if not text:
        return {}
    try:
        parsed = json.loads(text)
    except json.JSONDecodeError:
        return {"csv_data": _strip_csv_fence(text)}
    return parsed if isinstance(parsed, dict) else {}


def extract_tencent_docs_csv(payload: dict[str, Any]) -> str:
    content = tencent_docs_mcp_structured_content(payload)
    error = str(content.get("error") or "").strip()
    if error:
        raise ValueError("Tencent Docs MCP tool request failed")
    for key in ("csv_data", "csv", "data"):
        value = content.get(key)
        if isinstance(value, str) and value.strip():
            return _strip_csv_fence(value)
    return ""


class TencentDocsMcpClient:
    def __init__(self, session, token: str, url: str = TENCENT_DOCS_MCP_URL):
        self.session = session
        self.token = token
        self.url = url
        self._next_id = 1

    async def call_tool(self, name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        payload = {
            "jsonrpc": "2.0",
            "id": self._next_id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments},
        }
        self._next_id += 1
        response = await self.session.post(
            self.url,
            headers={
                "Authorization": self.token,
                "Content-Type": "application/json",
                "Accept": "application/json",
            },
            json=payload,
            timeout=60,
        )
        if response.status_code >= 400:
            raise ValueError(f"Tencent Docs MCP HTTP {response.status_code}")
        try:
            return json.loads(response.text)
        except json.JSONDecodeError as exc:
            raise ValueError("Tencent Docs MCP returned invalid JSON") from exc

    async def get_sheet_info(self, file_id: str) -> dict[str, Any]:
        payload = await self.call_tool("sheet.get_sheet_info", {"file_id": file_id})
        return tencent_docs_mcp_structured_content(payload)

    async def get_cell_csv(
        self,
        file_id: str,
        sheet_id: str,
        row_count: Optional[int] = None,
        col_count: Optional[int] = None,
    ) -> str:
        arguments: dict[str, Any] = {
            "file_id": file_id,
            "sheet_id": sheet_id,
            "start_row": 0,
            "start_col": 0,
            "return_csv": True,
        }
        if row_count and row_count > 0:
            arguments["end_row"] = row_count - 1
        if col_count and col_count > 0:
            arguments["end_col"] = col_count - 1
        payload = await self.call_tool("sheet.get_cell_data", arguments)
        return extract_tencent_docs_csv(payload)


class DailySourceClient(BaseCrawler):
    def __init__(self, data_dir: str = "data", db_path: str = "data/data.db") -> None:
        super().__init__(crawler_name="daily_source")
        self.data_dir = Path(data_dir)
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.db_path = db_path
        self.problems_db = ProblemsDatabaseManager(db_path)
        self.daily_db = DailyChallengeDatabaseManager(db_path)

    def _daily_refs(self, problems: list[dict]) -> list[str]:
        refs: list[str] = []
        seen: set[str] = set()
        for problem in problems:
            problem_id = problem.get("id")
            source = problem.get("source")
            if not problem_id or not source:
                continue
            ref = f"{source}:{problem_id}"
            if ref not in seen:
                refs.append(ref)
                seen.add(ref)
        return refs

    def _resolve_local_leetcode_ids(self, problems: list[dict]) -> list[dict]:
        resolved: list[dict] = []
        for problem in problems:
            problem_id = str(problem.get("id") or "").strip()
            if problem.get("source") != "leetcode" or problem_id.isdigit():
                resolved.append(problem)
                continue

            numeric_id = self.problems_db.get_numeric_problem_id_by_slug(
                "leetcode", problem.get("slug")
            )
            if not numeric_id:
                resolved.append(problem)
                continue

            canonical_problem = dict(problem)
            canonical_problem["id"] = numeric_id
            resolved.append(canonical_problem)
        return resolved

    def _store_daily_source(
        self, date: str, daily_source: str, problems: list[dict]
    ) -> bool:
        refs = self._daily_refs(problems)
        if not refs:
            logger.warning("No parseable %s daily problems for %s", daily_source, date)
            return False
        return self.daily_db.update_daily_source(date, daily_source, problems, refs)

    def _enrichment_candidates(self, problems: list[dict]) -> list[dict]:
        candidates: list[dict] = []
        for problem in problems:
            problem_id = problem.get("id")
            source = problem.get("source")
            if not problem_id or not source:
                continue
            existing = self.problems_db.get_problem(id=problem_id, source=source)
            existing_title = str((existing or {}).get("title") or "").strip()
            existing_content = str((existing or {}).get("content") or "").strip()
            snapshot_title = str(problem.get("title") or "").strip()
            snapshot_content = str(problem.get("content") or "").strip()
            has_codeforces_placeholder_title = (
                source == "codeforces" and existing_title == str(problem_id).strip()
            )
            has_missing_codeforces_tags = source == "codeforces" and not (
                (existing or {}).get("tags") or []
            )
            if (
                existing is None
                or (not existing_title and not existing_content)
                or (
                    existing_title == snapshot_title
                    and existing_content == snapshot_content
                )
                or has_codeforces_placeholder_title
                or has_missing_codeforces_tags
            ):
                candidates.append(problem)
        return candidates

    async def _enrich_problem(self, problem: dict) -> bool:
        source = problem.get("source")
        problem_id = problem.get("id")
        if not source or not problem_id:
            return False

        client_args = {"data_dir": str(self.data_dir), "db_path": self.db_path}
        if source == "codeforces":
            if problem_id.upper().startswith("GYM"):
                return bool(
                    await CodeforcesClient(**client_args).fetch_single_problem(
                        problem_id[3:],
                        stored_problem_id=problem_id,
                        prefer_source_details=True,
                    )
                )
            return bool(
                await CodeforcesClient(**client_args).fetch_single_problem(
                    problem_id, prefer_source_details=True
                )
            )
        if source == "atcoder":
            contest = problem.get("contest")
            target_id = f"{contest}/{problem_id}" if contest else problem_id
            return bool(
                await AtCoderClient(**client_args).fetch_single_problem(
                    target_id, prefer_source_details=True
                )
            )
        if source == "luogu":
            return bool(
                await LuoguClient(**client_args).fetch_single_problem(
                    problem_id, prefer_source_details=True
                )
            )
        if source == "leetcode":
            domain = (
                "cn"
                if re.match(
                    r"https?://(?:www\.)?leetcode\.cn(?:/|$)",
                    problem.get("link") or "",
                    re.IGNORECASE,
                )
                else "com"
            )
            result = await LeetCodeClient(domain=domain, **client_args).get_problem(
                problem_id=problem_id,
                domain=domain,
                prefer_source_details=True,
            )
            return bool(result)
        return False

    async def _store_and_enrich_daily_source(
        self, date: str, daily_source: str, problems: list[dict]
    ) -> bool:
        problems = self._resolve_local_leetcode_ids(problems)
        candidates = self._enrichment_candidates(problems)
        stored = self._store_daily_source(date, daily_source, problems)
        if not stored:
            return False

        for problem in candidates:
            source = problem.get("source")
            problem_id = problem.get("id")
            try:
                if not await self._enrich_problem(problem):
                    logger.warning(
                        "Failed to enrich daily source problem source=%s id=%s",
                        source,
                        problem_id,
                    )
            except Exception:
                logger.exception(
                    "Error enriching daily source problem source=%s id=%s",
                    source,
                    problem_id,
                )
        return stored

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
            response = await session.get(url, timeout=30)
            if response.status_code >= 400:
                logger.warning("HTTP %s while fetching %s", response.status_code, url)
                return False
            markdown = response.text
        if not markdown:
            logger.warning("No Sheep daily markdown for %s", date)
            return False
        problems = parse_sheep_daily_markdown(markdown)
        return await self._store_and_enrich_daily_source(date, "sheep", problems)

    async def fetch_0x3f_daily_online(self, date: str) -> bool:
        config = get_config()
        token = config.resolve_tencent_docs_token()
        if not token:
            raise ValueError(
                "Tencent Docs token is missing or empty in config and environment fallback"
            )

        async with self._create_curl_session(impersonate="chrome124") as session:
            mcp = TencentDocsMcpClient(session, token)
            info = await mcp.get_sheet_info(TENCENT_DOCS_0X3F_FILE_ID)
            sheet = self._find_0x3f_sheet(info)
            csv_text = await mcp.get_cell_csv(
                TENCENT_DOCS_0X3F_FILE_ID,
                TENCENT_DOCS_0X3F_SHEET_ID,
                row_count=sheet.get("row_count") if sheet else None,
                col_count=sheet.get("col_count") if sheet else None,
            )
        problems = parse_0x3f_daily_csv(csv_text, date)
        return await self._store_and_enrich_daily_source(date, "0x3f", problems)

    @staticmethod
    def _find_0x3f_sheet(info: dict[str, Any]) -> Optional[dict[str, Any]]:
        for sheet in info.get("sheets") or []:
            if sheet.get("sheet_id") == TENCENT_DOCS_0X3F_SHEET_ID:
                return sheet
        raise ValueError(
            f"Tencent Docs sheet {TENCENT_DOCS_0X3F_SHEET_ID} ({TENCENT_DOCS_0X3F_SHEET_NAME}) not found"
        )

    async def import_0x3f_daily(self, date: str, daily_file: str | None) -> bool:
        if daily_file:
            try:
                problems = parse_0x3f_daily_file(Path(daily_file), date)
            except FileNotFoundError:
                logger.error("0x3f daily file not found: %s", daily_file)
                return False
            return await self._store_and_enrich_daily_source(date, "0x3f", problems)
        return await self.fetch_0x3f_daily_online(date)

    async def fetch_daily_source(
        self, daily_source: str, date: str, daily_file: str | None = None
    ) -> bool:
        if daily_source == "sheep":
            return await self.fetch_sheep_daily(date)
        if daily_source == "0x3f":
            return await self.import_0x3f_daily(date, daily_file)
        raise ValueError(f"unsupported daily source: {daily_source}")


async def main() -> None:
    parser = argparse.ArgumentParser(description="Additional daily source importer")
    parser.add_argument(
        "--daily-source",
        choices=("sheep", "0x3f"),
        required=True,
        help="Import a curated daily source",
    )
    parser.add_argument(
        "--date", required=True, help="Daily source date, format YYYY-MM-DD"
    )
    parser.add_argument(
        "--daily-file",
        type=str,
        help="Local CSV/TSV export for --daily-source 0x3f",
    )
    parser.add_argument("--data-dir", type=str, default=None, help="Data directory")
    parser.add_argument("--db-path", type=str, default=None, help="Database path")

    args = parser.parse_args()
    config = get_config()
    data_dir = args.data_dir or str(Path(config.database_path).resolve().parent)
    db_path = args.db_path or str(Path(config.database_path).resolve())
    client = DailySourceClient(data_dir=data_dir, db_path=db_path)

    try:
        ok = await client.fetch_daily_source(
            args.daily_source, args.date, args.daily_file
        )
    except ValueError as exc:
        parser.error(str(exc))
    if not ok:
        sys.exit(2)


if __name__ == "__main__":
    asyncio.run(main())
