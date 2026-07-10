import asyncio
import json
import os
import sqlite3
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import daily_source
from daily_source import (
    DailySourceClient,
    extract_tencent_docs_csv,
    parse_0x3f_daily_csv,
    parse_0x3f_daily_file,
    parse_sheep_daily_markdown,
)
from leetcode import LeetCodeClient
from utils.config import ConfigManager
from utils.database import DailyChallengeDatabaseManager, ProblemsDatabaseManager


class TencentDocsConfigTests(unittest.TestCase):
    def _config(self, toml_text: str) -> ConfigManager:
        tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(tmpdir.cleanup)
        config_path = Path(tmpdir.name) / "config.toml"
        config_path.write_text(toml_text, encoding="utf-8")
        return ConfigManager(config_path=str(config_path))

    def test_token_env_defaults_to_tencent_docs_token(self):
        config = self._config("")
        self.assertEqual(config.tencent_docs_token_env, "TENCENT_DOCS_TOKEN")

    def test_token_env_uses_custom_name(self):
        config = self._config(
            '[daily_sources.tencent_docs]\ntoken_env = "MY_TENCENT_TOKEN"\n'
        )
        self.assertEqual(config.tencent_docs_token_env, "MY_TENCENT_TOKEN")

    def test_empty_token_env_disables_environment_fallback(self):
        config = self._config('[daily_sources.tencent_docs]\ntoken_env = "  "\n')
        self.assertEqual(config.tencent_docs_token_env, "")
        with patch.dict(os.environ, {"TENCENT_DOCS_TOKEN": "env-token"}):
            self.assertIsNone(config.resolve_tencent_docs_token())

    def test_resolve_token_prefers_config_value_and_trims_whitespace(self):
        config = self._config(
            "[daily_sources.tencent_docs]\n"
            'token = " config-token "\n'
            'token_env = "OJ_TEST_TENCENT_TOKEN"\n'
        )
        with patch.dict(os.environ, {"OJ_TEST_TENCENT_TOKEN": "env-token"}):
            self.assertEqual(config.tencent_docs_token, "config-token")
            self.assertEqual(config.resolve_tencent_docs_token(), "config-token")

    def test_resolve_token_falls_back_to_environment_when_config_is_blank(self):
        config = self._config(
            "[daily_sources.tencent_docs]\n"
            'token = "  "\n'
            'token_env = "OJ_TEST_TENCENT_TOKEN"\n'
        )
        with patch.dict(os.environ, {"OJ_TEST_TENCENT_TOKEN": " secret "}):
            self.assertEqual(config.resolve_tencent_docs_token(), "secret")
        with patch.dict(os.environ, {"OJ_TEST_TENCENT_TOKEN": "   "}):
            self.assertIsNone(config.resolve_tencent_docs_token())
        with patch.dict(os.environ, {}, clear=True):
            self.assertIsNone(config.resolve_tencent_docs_token())


class DailyChallengeStorageTests(unittest.TestCase):
    def setUp(self):
        self._tmpdir = tempfile.TemporaryDirectory()
        self.db_path = str(Path(self._tmpdir.name) / "data.db")

    def tearDown(self):
        self._tmpdir.cleanup()

    def test_update_daily_stores_compact_com_row(self):
        manager = DailyChallengeDatabaseManager(self.db_path)

        self.assertTrue(
            manager.update_daily({"date": "2026-01-01", "domain": "com", "id": 1234})
        )

        self.assertEqual(
            self._daily_rows(), [("2026-01-01", "leetcode.com", '["leetcode:1234"]')]
        )

    def test_update_daily_maps_cn_source(self):
        manager = DailyChallengeDatabaseManager(self.db_path)

        self.assertTrue(
            manager.update_daily({"date": "2026-01-01", "domain": "cn", "id": 1})
        )

        self.assertEqual(
            self._daily_rows(), [("2026-01-01", "leetcode.cn", '["leetcode:1"]')]
        )

    def test_update_daily_preserves_explicit_ref_order_and_colons(self):
        manager = DailyChallengeDatabaseManager(self.db_path)

        self.assertTrue(
            manager.update_daily(
                {
                    "date": "2026-01-01",
                    "source": "custom.daily",
                    "problems": ["leetcode:1234", "leetcode:1", "custom:abc:123"],
                }
            )
        )

        stored = self._daily_rows()[0]
        self.assertEqual(stored[:2], ("2026-01-01", "custom.daily"))
        self.assertEqual(
            json.loads(stored[2]), ["leetcode:1234", "leetcode:1", "custom:abc:123"]
        )

    def test_update_daily_accepts_plain_string_problem_ref(self):
        manager = DailyChallengeDatabaseManager(self.db_path)

        self.assertTrue(
            manager.update_daily(
                {
                    "date": "2026-01-01",
                    "source": "leetcode.com",
                    "problems": "leetcode:1234",
                }
            )
        )

        self.assertEqual(
            self._daily_rows(), [("2026-01-01", "leetcode.com", '["leetcode:1234"]')]
        )

    def test_legacy_migration_converts_rows_and_slug_fallback(self):
        ProblemsDatabaseManager(self.db_path).update_problem(
            self._problem(id="42", slug="two-sum"),
            force_update=True,
        )
        self._create_legacy_daily_table()
        with sqlite3.connect(self.db_path) as conn:
            conn.execute(
                "INSERT INTO daily_challenge (date, domain, id, slug) VALUES (?, ?, ?, ?)",
                ("2026-01-01", "com", 1234, "daily-com"),
            )
            conn.execute(
                "INSERT INTO daily_challenge (date, domain, id, slug) VALUES (?, ?, ?, ?)",
                ("2026-01-02", "cn", None, "two-sum"),
            )
            conn.execute(
                "INSERT INTO daily_challenge (date, domain, id, slug) VALUES (?, ?, ?, ?)",
                ("2026-01-03", "cn", None, "missing"),
            )

        DailyChallengeDatabaseManager(self.db_path)

        self.assertEqual(
            self._daily_rows(),
            [
                ("2026-01-01", "leetcode.com", '["leetcode:1234"]'),
                ("2026-01-02", "leetcode.cn", '["leetcode:42"]'),
            ],
        )

    def test_legacy_migration_preserves_snapshot_when_problem_row_is_missing(self):
        ProblemsDatabaseManager(self.db_path)
        self._create_full_legacy_daily_table()
        with sqlite3.connect(self.db_path) as conn:
            conn.execute(
                """
                INSERT INTO daily_challenge (
                    date, domain, id, slug, title, title_cn, difficulty, ac_rate,
                    rating, contest, problem_index, tags, link, category, paid_only,
                    content, content_cn, similar_questions
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    "2026-01-01",
                    "com",
                    1234,
                    "legacy-only",
                    "Legacy Only",
                    "舊快取",
                    "Easy",
                    50.0,
                    1200.0,
                    "weekly-contest-1",
                    "A",
                    '["Array"]',
                    "https://leetcode.com/problems/legacy-only/",
                    "Algorithms",
                    0,
                    "legacy content",
                    "舊內容",
                    '["two-sum"]',
                ),
            )

        DailyChallengeDatabaseManager(self.db_path)
        stored = ProblemsDatabaseManager(self.db_path).get_problem(
            id="1234", source="leetcode"
        )

        self.assertEqual(
            self._daily_rows(), [("2026-01-01", "leetcode.com", '["leetcode:1234"]')]
        )
        self.assertEqual(stored["slug"], "legacy-only")
        self.assertEqual(stored["title"], "Legacy Only")
        self.assertEqual(stored["content"], "legacy content")
        self.assertEqual(stored["tags"], ["Array"])
        self.assertEqual(stored["similar_questions"], ["two-sum"])

    def test_hydrate_cached_daily_fetches_missing_leetcode_problem(self):
        problems_db = ProblemsDatabaseManager(self.db_path)
        client = LeetCodeClient.__new__(LeetCodeClient)
        client.problems_db = problems_db
        calls = []

        async def get_problem(problem_id=None, slug=None, domain=None):
            calls.append((problem_id, slug, domain))
            return self._problem(id=problem_id, slug="two-sum")

        client.get_problem = get_problem
        daily = {
            "date": "2026-01-01",
            "source": "leetcode.com",
            "domain": "com",
            "problems": ["leetcode:1"],
        }

        hydrated = asyncio.run(client._hydrate_cached_daily(daily, "com"))

        self.assertIsNotNone(hydrated)
        self.assertEqual(calls, [("1", None, "com")])
        self.assertEqual(hydrated["resolved_problems"][0]["id"], "1")

    def test_daily_history_uses_hydrated_compact_problem_metadata(self):
        client = LeetCodeClient.__new__(LeetCodeClient)
        client.domain = "com"

        async def get_daily_challenge(date_str=None, domain=None):
            return {
                "date": date_str,
                "source": "leetcode.com",
                "problems": ["leetcode:1"],
                "resolved_problems": [self._problem(id="1", slug="two-sum")],
            }

        client.get_daily_challenge = get_daily_challenge

        history = asyncio.run(client.get_daily_history("2026-01-01", years=1))

        self.assertEqual(
            history,
            [
                {
                    "date": "2025-01-01",
                    "id": "1",
                    "title": "Two Sum",
                    "difficulty": "Easy",
                    "link": "https://leetcode.com/problems/two-sum/",
                }
            ],
        )

    def test_parse_sheep_daily_markdown_extracts_regular_and_gym_links(self):
        markdown = """
| Difficulty | Problems | Hints |
| -------- | -------- | -------- |
| *1400 | [1930A](https://codeforces.com/contest/1930/problem/A) | Sort greedily. |
| *2100 | [GYM106539D](https://codeforces.com/gym/106539/problem/D) | Probability. |
"""

        problems = parse_sheep_daily_markdown(markdown)

        self.assertEqual(
            [problem["id"] for problem in problems], ["1930A", "GYM106539D"]
        )
        self.assertEqual(problems[0]["rating"], 1400)
        self.assertEqual(problems[0]["content"], "Sort greedily.")
        self.assertEqual(problems[1]["contest"], "GYM106539")
        self.assertEqual(problems[1]["problem_index"], "D")

    def test_parse_sheep_daily_markdown_dedupes_links_by_problem_id(self):
        markdown = (
            "| Difficulty | Problems | Hints |\n"
            "| -------- | -------- | -------- |\n"
            "| *1200 | [Maximise The Score](https://codeforces.com/contest/1930/problem/A/) | Hint |\n"
        )

        problems = parse_sheep_daily_markdown(markdown)

        self.assertEqual(len(problems), 1)
        self.assertEqual(problems[0]["id"], "1930A")
        self.assertEqual(problems[0]["title"], "Maximise The Score")

    def test_store_sheep_daily_writes_problem_snapshots_and_daily_refs(self):
        client = self._codeforces_client_no_config()
        problems = parse_sheep_daily_markdown(
            "| Difficulty | Problems | Hints |\n"
            "| -------- | -------- | -------- |\n"
            "| *1200 | [1930A](https://codeforces.com/problemset/problem/1930/A) | Hint |\n"
        )

        self.assertTrue(client._store_daily_source("2026-06-02", "sheep", problems))

        stored = ProblemsDatabaseManager(self.db_path).get_problem(
            id="1930A", source="codeforces"
        )
        self.assertEqual(stored["title"], "1930A")
        self.assertEqual(stored["rating"], 1200.0)
        self.assertEqual(
            self._daily_rows(),
            [("2026-06-02", "sheep", '["codeforces:1930A"]')],
        )

    def test_store_daily_source_rolls_back_problem_snapshots_when_daily_write_fails(
        self,
    ):
        client = self._codeforces_client_no_config()
        problems = parse_sheep_daily_markdown(
            "| Difficulty | Problems | Hints |\n"
            "| -------- | -------- | -------- |\n"
            "| *1200 | [1930A](https://codeforces.com/problemset/problem/1930/A) | Hint |\n"
        )
        with sqlite3.connect(self.db_path) as conn:
            conn.execute("DROP TABLE daily_challenge")
            conn.execute(
                "CREATE TABLE daily_challenge (date TEXT PRIMARY KEY, source TEXT NOT NULL)"
            )

        self.assertFalse(client._store_daily_source("2026-06-02", "sheep", problems))

        self.assertIsNone(
            ProblemsDatabaseManager(self.db_path).get_problem(
                id="1930A", source="codeforces"
            )
        )

    def test_store_daily_source_fills_sparse_codeforces_metadata(self):
        client = self._codeforces_client_no_config()
        sparse = {
            "id": "1930A",
            "source": "codeforces",
            "slug": "1930A",
            "title": "",
            "title_cn": "",
            "difficulty": None,
            "ac_rate": None,
            "rating": None,
            "contest": None,
            "problem_index": None,
            "tags": [],
            "link": "",
            "category": None,
            "paid_only": 0,
            "content": None,
            "content_cn": None,
            "similar_questions": [],
        }
        self.assertTrue(client.problems_db.update_problem(sparse, force_update=True))
        problems = parse_sheep_daily_markdown(
            "| Difficulty | Problems | Hints |\n"
            "| -------- | -------- | -------- |\n"
            "| *1200 | [Daily Title](https://codeforces.com/problemset/problem/1930/A) | Daily hint |\n"
        )

        self.assertTrue(client._store_daily_source("2026-06-02", "sheep", problems))

        stored = ProblemsDatabaseManager(self.db_path).get_problem(
            id="1930A", source="codeforces"
        )
        self.assertEqual(stored["title"], "Daily Title")
        self.assertEqual(stored["rating"], 1200.0)
        self.assertEqual(stored["contest"], "1930")
        self.assertEqual(stored["problem_index"], "A")
        self.assertEqual(
            stored["link"], "https://codeforces.com/problemset/problem/1930/A"
        )
        self.assertEqual(stored["content"], "Daily hint")
        self.assertEqual(
            self._daily_rows(),
            [("2026-06-02", "sheep", '["codeforces:1930A"]')],
        )

    def test_store_daily_source_preserves_existing_codeforces_metadata(self):
        client = self._codeforces_client_no_config()
        existing = {
            "id": "1930A",
            "source": "codeforces",
            "slug": "1930A",
            "title": "Full Metadata Title",
            "title_cn": "",
            "difficulty": "hard",
            "ac_rate": 65.5,
            "rating": 2200,
            "contest": "1930",
            "problem_index": "A",
            "tags": ["math"],
            "link": "https://codeforces.com/contest/1930/problem/A",
            "category": "Algorithms",
            "paid_only": 0,
            "content": "Full statement",
            "content_cn": None,
            "similar_questions": [],
        }
        self.assertTrue(client.problems_db.update_problem(existing, force_update=True))
        problems = parse_sheep_daily_markdown(
            "| Difficulty | Problems | Hints |\n"
            "| -------- | -------- | -------- |\n"
            "| *1200 | [Daily Title](https://codeforces.com/problemset/problem/1930/A) | Daily hint |\n"
        )

        self.assertTrue(client._store_daily_source("2026-06-02", "sheep", problems))

        stored = ProblemsDatabaseManager(self.db_path).get_problem(
            id="1930A", source="codeforces"
        )
        self.assertEqual(stored["title"], "Full Metadata Title")
        self.assertEqual(stored["difficulty"], "hard")
        self.assertEqual(stored["ac_rate"], 65.5)
        self.assertEqual(stored["rating"], 2200.0)
        self.assertEqual(stored["tags"], ["math"])
        self.assertEqual(stored["content"], "Full statement")
        self.assertEqual(
            self._daily_rows(),
            [("2026-06-02", "sheep", '["codeforces:1930A"]')],
        )

    def test_parse_0x3f_daily_file_extracts_requested_date_urls(self):
        daily_file = Path(self._tmpdir.name) / "0x3f.csv"
        daily_file.write_text(
            "日期,難度,題目\n"
            "2026-06-01,800,https://codeforces.com/contest/1/problem/A\n"
            "2026/6/2,1700,https://codeforces.com/problemset/problem/1930/A\n"
            "2026-06-02 00:00:00,1800,https://codeforces.com/contest/1930/problem/B\n",
            encoding="utf-8",
        )

        problems = parse_0x3f_daily_file(daily_file, "2026-06-02")

        self.assertEqual([problem["id"] for problem in problems], ["1930A", "1930B"])
        self.assertEqual([problem["rating"] for problem in problems], [1700, 1800])

    def test_parse_0x3f_daily_csv_extracts_mixed_oj_urls(self):
        csv_text = (
            "日期,難度,題目\n"
            '2026-06-02,1800,"'
            "[Two Sum](https://leetcode.com/problems/two-sum/) "
            "[ABC001 A](https://atcoder.jp/contests/abc001/tasks/abc001_1) "
            "[GYM106539D](https://codeforces.com/gym/106539/problem/D) "
            "[P1001](https://www.luogu.com.cn/problem/P1001)"
            '"\n'
        )

        problems = parse_0x3f_daily_csv(csv_text, "2026-06-02")

        self.assertEqual(
            [(problem["source"], problem["id"]) for problem in problems],
            [
                ("leetcode", "two-sum"),
                ("atcoder", "abc001_1"),
                ("codeforces", "GYM106539D"),
                ("luogu", "P1001"),
            ],
        )
        self.assertEqual([problem["rating"] for problem in problems], [1800] * 4)

    def test_parse_0x3f_daily_csv_dedupes_luogu_hosts(self):
        csv_text = (
            "日期,題目\n"
            "2026-06-02,"
            "https://www.luogu.com.cn/problem/P1001 "
            "https://www.luogu.com/problem/P1001\n"
        )

        problems = parse_0x3f_daily_csv(csv_text, "2026-06-02")

        self.assertEqual(
            [(problem["source"], problem["id"]) for problem in problems],
            [("luogu", "P1001")],
        )

    def test_extract_tencent_docs_csv_from_structured_and_text_payloads(self):
        self.assertEqual(
            extract_tencent_docs_csv(
                {
                    "result": {
                        "structuredContent": {"csv_data": "```csv\n日期,題目\n```"}
                    }
                }
            ),
            "日期,題目",
        )
        self.assertEqual(
            extract_tencent_docs_csv(
                {"result": {"content": [{"type": "text", "text": "a,b\n1,2"}]}}
            ),
            "a,b\n1,2",
        )

    def test_extract_tencent_docs_csv_raises_on_error_payloads(self):
        with self.assertRaisesRegex(ValueError, "JSON-RPC request failed"):
            extract_tencent_docs_csv({"error": {"code": -32600, "message": "bad"}})
        with self.assertRaisesRegex(ValueError, "MCP tool request failed"):
            extract_tencent_docs_csv(
                {"result": {"structuredContent": {"error": "permission denied"}}}
            )

    def _patched_token_config(self, token):
        class TokenConfig:
            tencent_docs_token_env = "TENCENT_DOCS_TOKEN"

            def resolve_tencent_docs_token(self):
                return token

        original_get_config = daily_source.get_config
        daily_source.get_config = lambda: TokenConfig()
        self.addCleanup(setattr, daily_source, "get_config", original_get_config)

    def test_fetch_0x3f_daily_online_stores_row_via_mcp(self):
        client = self._codeforces_client_no_config()
        self._patched_token_config("token")

        class FakeMcp:
            def __init__(self, session, token):
                self.token = token

            async def get_sheet_info(self, file_id):
                assert file_id == daily_source.TENCENT_DOCS_0X3F_FILE_ID
                return {
                    "sheets": [
                        {
                            "sheet_id": daily_source.TENCENT_DOCS_0X3F_SHEET_ID,
                            "row_count": 3,
                            "col_count": 3,
                        }
                    ]
                }

            async def get_cell_csv(
                self, file_id, sheet_id, row_count=None, col_count=None
            ):
                assert sheet_id == daily_source.TENCENT_DOCS_0X3F_SHEET_ID
                return (
                    "日期,難度,題目\n"
                    "2026-06-02,1800,https://leetcode.com/problems/two-sum/\n"
                )

        class FakeSession:
            async def __aenter__(self):
                return self

            async def __aexit__(self, *exc):
                return False

        client._create_curl_session = lambda **kwargs: FakeSession()
        with patch.object(daily_source, "TencentDocsMcpClient", FakeMcp):
            self.assertTrue(asyncio.run(client.import_0x3f_daily("2026-06-02", None)))

        rows = self._daily_rows()
        self.assertEqual(rows, [("2026-06-02", "0x3f", '["leetcode:two-sum"]')])

    def test_daily_file_takes_precedence_over_online_fetch(self):
        client = self._codeforces_client_no_config()
        self._patched_token_config("token")
        daily_file = Path(self._tmpdir.name) / "0x3f.csv"
        daily_file.write_text(
            "日期,題目\n2026-06-02,https://leetcode.com/problems/two-sum/\n",
            encoding="utf-8",
        )

        class ExplodingMcp:
            def __init__(self, *args, **kwargs):
                raise AssertionError(
                    "online fetch must not run when --daily-file is set"
                )

        with patch.object(daily_source, "TencentDocsMcpClient", ExplodingMcp):
            self.assertTrue(
                asyncio.run(client.import_0x3f_daily("2026-06-02", str(daily_file)))
            )

        rows = self._daily_rows()
        self.assertEqual(rows, [("2026-06-02", "0x3f", '["leetcode:two-sum"]')])

    def test_0x3f_import_requires_parseable_input(self):
        client = self._codeforces_client_no_config()
        daily_file = Path(self._tmpdir.name) / "0x3f.csv"
        daily_file.write_text(
            "日期,題目\n2026-06-02,not a codeforces url\n", encoding="utf-8"
        )

        self.assertFalse(
            asyncio.run(client.import_0x3f_daily("2026-06-02", str(daily_file)))
        )
        self.assertFalse(
            asyncio.run(
                client.import_0x3f_daily(
                    "2026-06-02", str(daily_file.with_name("missing.csv"))
                )
            )
        )
        self.assertEqual(self._daily_rows(), [])
        original_get_config = daily_source.get_config

        class MissingTokenConfig:
            tencent_docs_token_env = "TENCENT_DOCS_TOKEN"

            def resolve_tencent_docs_token(self):
                return None

        try:
            daily_source.get_config = lambda: MissingTokenConfig()
            with self.assertRaisesRegex(ValueError, "missing or empty"):
                asyncio.run(client.import_0x3f_daily("2026-06-02", None))
        finally:
            daily_source.get_config = original_get_config

    def _create_legacy_daily_table(self):
        with sqlite3.connect(self.db_path) as conn:
            conn.execute("DROP TABLE IF EXISTS daily_challenge")
            conn.execute(
                """
                CREATE TABLE daily_challenge (
                    date TEXT NOT NULL,
                    domain TEXT NOT NULL,
                    id INTEGER,
                    slug TEXT NOT NULL,
                    PRIMARY KEY (date, domain)
                )
                """
            )

    def _create_full_legacy_daily_table(self):
        with sqlite3.connect(self.db_path) as conn:
            conn.execute("DROP TABLE IF EXISTS daily_challenge")
            conn.execute(
                """
                CREATE TABLE daily_challenge (
                    date TEXT NOT NULL,
                    domain TEXT NOT NULL,
                    id INTEGER,
                    slug TEXT NOT NULL,
                    title TEXT,
                    title_cn TEXT,
                    difficulty TEXT,
                    ac_rate REAL,
                    rating REAL,
                    contest TEXT,
                    problem_index TEXT,
                    tags TEXT,
                    link TEXT,
                    category TEXT,
                    paid_only INTEGER,
                    content TEXT,
                    content_cn TEXT,
                    similar_questions TEXT,
                    PRIMARY KEY (date, domain)
                )
                """
            )

    def _daily_rows(self):
        with sqlite3.connect(self.db_path) as conn:
            return conn.execute(
                "SELECT date, source, problems FROM daily_challenge ORDER BY date, source"
            ).fetchall()

    def _codeforces_client_no_config(self):
        client = DailySourceClient.__new__(DailySourceClient)
        client.problems_db = ProblemsDatabaseManager(self.db_path)
        client.daily_db = DailyChallengeDatabaseManager(self.db_path)
        return client

    @staticmethod
    def _problem(id, slug):
        return {
            "id": str(id),
            "source": "leetcode",
            "slug": slug,
            "title": slug.replace("-", " ").title(),
            "title_cn": "",
            "difficulty": "Easy",
            "ac_rate": 50.0,
            "rating": None,
            "contest": None,
            "problem_index": None,
            "tags": [],
            "link": f"https://leetcode.com/problems/{slug}/",
            "category": "Algorithms",
            "paid_only": 0,
            "content": None,
            "content_cn": None,
            "similar_questions": [],
        }


if __name__ == "__main__":
    unittest.main()
