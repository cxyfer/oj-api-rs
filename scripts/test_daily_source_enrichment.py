import asyncio
import sqlite3
import tempfile
import unittest
from pathlib import Path
from unittest.mock import AsyncMock, Mock, call, patch

import daily_source
from daily_source import DailySourceClient
from leetcode import LeetCodeClient
from utils.database import DailyChallengeDatabaseManager, ProblemsDatabaseManager


class DailySourceEnrichmentTests(unittest.TestCase):
    def setUp(self):
        self._tmpdir = tempfile.TemporaryDirectory()
        self.db_path = str(Path(self._tmpdir.name) / "data.db")

    def tearDown(self):
        self._tmpdir.cleanup()

    def test_enrichment_candidates_use_pre_write_database_state(self):
        client = self._client()
        absent = self._problem("codeforces", "absent")
        blank = self._problem(
            "codeforces", "blank", title="Incoming title", content="Incoming content"
        )
        title_only = self._problem("codeforces", "title-only")
        content_only = self._problem("codeforces", "content-only")
        complete = self._problem("codeforces", "complete")
        complete_with_tags = self._problem(
            "codeforces",
            "complete-with-tags",
            title="Complete Title",
            content="Daily summary",
            tags=["implementation"],
        )
        unchanged_snapshot = self._problem(
            "codeforces",
            "unchanged-snapshot",
            title="Daily title",
            content="Daily summary",
        )
        gym_placeholder = self._problem(
            "codeforces",
            "GYM106054C",
            title="GYM106054C",
            content="Daily summary",
        )
        stored_gym_placeholder = gym_placeholder.copy()
        stored_gym_placeholder["content"] = "Official statement already stored"
        missing_tags = self._problem(
            "codeforces",
            "343C",
            title="Read Time",
            content="Daily summary",
        )
        stored_missing_tags = missing_tags.copy()
        stored_missing_tags["content"] = "Official statement already stored"

        self._store_existing(
            client, self._problem("codeforces", "blank", title=" \t", content="\n")
        )
        self._store_existing(
            client,
            self._problem(
                "codeforces", "title-only", title="Existing title", content=" "
            ),
        )
        stored_complete_with_tags = complete_with_tags.copy()
        stored_complete_with_tags["content"] = "Official statement"
        self._store_existing(client, stored_complete_with_tags)
        self._store_existing(
            client,
            self._problem(
                "codeforces", "content-only", title=" ", content="Existing content"
            ),
        )
        self._store_existing(
            client,
            self._problem(
                "codeforces",
                "complete",
                title="Existing title",
                content="Existing content",
            ),
        )
        self._store_existing(client, unchanged_snapshot.copy())
        self._store_existing(client, stored_gym_placeholder)
        self._store_existing(client, stored_missing_tags)

        candidates = client._enrichment_candidates(
            [
                absent,
                blank,
                title_only,
                content_only,
                complete,
                complete_with_tags,
                unchanged_snapshot,
                gym_placeholder,
                missing_tags,
            ]
        )

        self.assertEqual(
            candidates,
            [
                absent,
                blank,
                title_only,
                content_only,
                complete,
                unchanged_snapshot,
                gym_placeholder,
                missing_tags,
            ],
        )

    def test_enrich_problem_dispatches_codeforces_regular_and_gym(self):
        client = self._client()
        with patch.object(daily_source, "CodeforcesClient") as codeforces_client:
            fetch_single_problem = AsyncMock(return_value=True)
            codeforces_client.return_value.fetch_single_problem = fetch_single_problem

            self.assertTrue(
                asyncio.run(
                    client._enrich_problem(self._problem("codeforces", "1930A"))
                )
            )
            self.assertTrue(
                asyncio.run(
                    client._enrich_problem(self._problem("codeforces", "GYM106539D"))
                )
            )

        self.assertEqual(
            fetch_single_problem.await_args_list,
            [
                call("1930A", prefer_source_details=True),
                call(
                    "106539D",
                    stored_problem_id="GYM106539D",
                    prefer_source_details=True,
                ),
            ],
        )

    def test_enrich_problem_dispatches_atcoder_and_luogu(self):
        client = self._client()
        atcoder_problem = self._problem("atcoder", "abc001_1", contest="abc001")
        luogu_problem = self._problem("luogu", "P1001")

        with (
            patch.object(daily_source, "AtCoderClient") as atcoder_client,
            patch.object(daily_source, "LuoguClient") as luogu_client,
        ):
            atcoder_fetch = AsyncMock(return_value=True)
            luogu_fetch = AsyncMock(return_value=True)
            atcoder_client.return_value.fetch_single_problem = atcoder_fetch
            luogu_client.return_value.fetch_single_problem = luogu_fetch

            self.assertTrue(asyncio.run(client._enrich_problem(atcoder_problem)))
            self.assertTrue(asyncio.run(client._enrich_problem(luogu_problem)))

        atcoder_fetch.assert_awaited_once_with(
            "abc001/abc001_1", prefer_source_details=True
        )
        luogu_fetch.assert_awaited_once_with("P1001", prefer_source_details=True)

    def test_store_and_enrich_dispatches_leetcode_cn_after_snapshot_storage(self):
        client = self._client()
        problem = self._problem(
            "leetcode",
            "two-sum",
            slug="two-sum",
            link="https://leetcode.cn/problems/two-sum/",
        )
        snapshot_states = []

        async def get_problem(*, problem_id, domain):
            snapshot_states.append(
                (
                    client.problems_db.get_problem(id="two-sum", source="leetcode")
                    is not None,
                    self._daily_rows(),
                )
            )
            return {"id": problem_id, "domain": domain}

        with patch.object(daily_source, "LeetCodeClient") as leetcode_client:
            leetcode_client.return_value.get_problem = AsyncMock(
                side_effect=get_problem
            )

            self.assertTrue(
                asyncio.run(
                    client._store_and_enrich_daily_source(
                        "2026-07-10", "0x3f", [problem]
                    )
                )
            )

        leetcode_client.assert_called_once_with(
            domain="cn", data_dir=str(client.data_dir), db_path=self.db_path
        )
        leetcode_client.return_value.get_problem.assert_awaited_once_with(
            problem_id="two-sum", domain="cn"
        )
        self.assertEqual(
            snapshot_states,
            [(True, [("2026-07-10", "0x3f", '["leetcode:two-sum"]')])],
        )

    def test_store_and_enrich_uses_local_leetcode_numeric_id(self):
        client = self._client()
        numeric_problem = self._problem(
            "leetcode",
            "1",
            slug="two-sum",
            title=" ",
            content="\t",
            link="https://leetcode.com/problems/two-sum/",
        )
        numeric_problem["rating"] = 1700.0
        self._store_existing(client, numeric_problem)
        problem = self._problem(
            "leetcode",
            "two-sum",
            slug="two-sum",
            title="Daily title",
            content="Daily summary",
            link="https://leetcode.com/problems/two-sum/",
        )
        problem["rating"] = 1700.0

        async def get_problem(*, problem_id, domain):
            return {"id": problem_id, "domain": domain}

        with patch.object(daily_source, "LeetCodeClient") as leetcode_client:
            leetcode_client.return_value.get_problem = AsyncMock(
                side_effect=get_problem
            )

            self.assertTrue(
                asyncio.run(
                    client._store_and_enrich_daily_source(
                        "2026-07-10", "0x3f", [problem]
                    )
                )
            )

        leetcode_client.return_value.get_problem.assert_awaited_once_with(
            problem_id="1", domain="com"
        )
        self.assertEqual(
            self._daily_rows(),
            [("2026-07-10", "0x3f", '["leetcode:1"]')],
        )
        self.assertIsNone(
            client.problems_db.get_problem(id="two-sum", source="leetcode")
        )

    def test_numeric_leetcode_id_lookup_ignores_slug_snapshot(self):
        client = self._client()
        self._store_existing(
            client,
            self._problem("leetcode", "two-sum", slug="two-sum"),
        )
        self._store_existing(
            client,
            self._problem("leetcode", "1", slug="two-sum"),
        )

        self.assertEqual(
            client.problems_db.get_numeric_problem_id_by_slug("leetcode", "two-sum"),
            "1",
        )

    def test_store_and_enrich_selects_before_storage_and_continues_after_failure(self):
        client = self._client()
        preexisting_blank = self._problem(
            "codeforces", "1930A", title=" ", content="\t"
        )
        self._store_existing(client, preexisting_blank)
        problems = [
            self._problem(
                "codeforces",
                "1930A",
                title="Incoming title",
                content="Incoming content",
            ),
            self._problem("codeforces", "1930B"),
            self._problem("codeforces", "1930C"),
        ]
        events = []
        snapshot_states = []
        original_candidates = client._enrichment_candidates
        original_store = client._store_daily_source

        def enrichment_candidates(candidate_problems):
            events.append("candidates")
            return original_candidates(candidate_problems)

        def store_daily_source(date, daily_source_name, stored_problems):
            events.append("store")
            return original_store(date, daily_source_name, stored_problems)

        async def enrich_problem(problem):
            problem_id = problem["id"]
            events.append(f"enrich:{problem_id}")
            snapshot_states.append(
                (
                    problem_id,
                    client.problems_db.get_problem(id=problem_id, source="codeforces")
                    is not None,
                    self._daily_rows(),
                )
            )
            if problem_id == "1930B":
                raise RuntimeError("crawler failed")
            return True

        client._enrichment_candidates = enrichment_candidates
        client._store_daily_source = store_daily_source
        client._enrich_problem = enrich_problem

        self.assertTrue(
            asyncio.run(
                client._store_and_enrich_daily_source("2026-07-10", "sheep", problems)
            )
        )

        self.assertEqual(
            events,
            [
                "candidates",
                "store",
                "enrich:1930A",
                "enrich:1930B",
                "enrich:1930C",
            ],
        )
        self.assertEqual(
            snapshot_states,
            [
                (
                    "1930A",
                    True,
                    [
                        (
                            "2026-07-10",
                            "sheep",
                            '["codeforces:1930A", "codeforces:1930B", "codeforces:1930C"]',
                        )
                    ],
                ),
                (
                    "1930B",
                    True,
                    [
                        (
                            "2026-07-10",
                            "sheep",
                            '["codeforces:1930A", "codeforces:1930B", "codeforces:1930C"]',
                        )
                    ],
                ),
                (
                    "1930C",
                    True,
                    [
                        (
                            "2026-07-10",
                            "sheep",
                            '["codeforces:1930A", "codeforces:1930B", "codeforces:1930C"]',
                        )
                    ],
                ),
            ],
        )

    def test_store_failure_does_not_call_enrichment(self):
        client = self._client()
        client._store_daily_source = Mock(return_value=False)
        client._enrich_problem = AsyncMock(return_value=True)

        self.assertFalse(
            asyncio.run(
                client._store_and_enrich_daily_source(
                    "2026-07-10", "sheep", [self._problem("codeforces", "1930A")]
                )
            )
        )

        client._enrich_problem.assert_not_awaited()

    def test_leetcode_get_problem_refetches_whitespace_only_detail(self):
        problems_db = ProblemsDatabaseManager(self.db_path)
        self.assertTrue(
            problems_db.update_problem(
                {
                    "id": "two-sum",
                    "source": "leetcode",
                    "slug": "two-sum",
                    "title": " \t",
                    "title_cn": "",
                    "difficulty": "Easy",
                    "ac_rate": 50.0,
                    "rating": 1700.0,
                    "contest": None,
                    "problem_index": None,
                    "tags": ["Array"],
                    "link": "https://leetcode.com/problems/two-sum/",
                    "category": "Algorithms",
                    "paid_only": 0,
                    "content": "\n ",
                    "content_cn": None,
                    "similar_questions": [],
                },
                force_update=True,
            )
        )
        client = object.__new__(LeetCodeClient)
        client.problems_db = problems_db
        client.fetch_problem_detail = AsyncMock(
            return_value={
                "title": "Two Sum",
                "content": "<p>Find the pair.</p>",
                "tags": ["Array", "Hash Table"],
            }
        )
        client.get_problem_rating = AsyncMock()

        self.assertIsNotNone(
            asyncio.run(client.get_problem(slug="two-sum", domain="com"))
        )

        client.fetch_problem_detail.assert_awaited_once_with("two-sum", domain="com")
        client.get_problem_rating.assert_not_awaited()
        stored = problems_db.get_problem(id="two-sum", source="leetcode")
        self.assertIsNotNone(stored)
        self.assertTrue(stored["title"].strip())
        self.assertTrue(stored["content"].strip())

    def _client(self):
        client = DailySourceClient.__new__(DailySourceClient)
        client.data_dir = Path(self._tmpdir.name) / "data"
        client.data_dir.mkdir()
        client.db_path = self.db_path
        client.problems_db = ProblemsDatabaseManager(self.db_path)
        client.daily_db = DailyChallengeDatabaseManager(self.db_path)
        return client

    @staticmethod
    def _problem(
        source,
        problem_id,
        *,
        slug=None,
        title=None,
        content=None,
        contest=None,
        link=None,
        tags=None,
    ):
        slug = slug or problem_id
        return {
            "id": problem_id,
            "source": source,
            "slug": slug,
            "title": problem_id if title is None else title,
            "title_cn": "",
            "difficulty": None,
            "ac_rate": None,
            "rating": None,
            "contest": contest,
            "problem_index": None,
            "tags": tags or [],
            "link": link or f"https://example.test/{source}/{slug}",
            "category": "Algorithms",
            "paid_only": 0,
            "content": content,
            "content_cn": None,
            "similar_questions": [],
        }

    def _store_existing(self, client, problem):
        self.assertTrue(client.problems_db.update_problem(problem, force_update=True))

    def _daily_rows(self):
        with sqlite3.connect(self.db_path) as conn:
            return conn.execute(
                "SELECT date, source, problems FROM daily_challenge ORDER BY date, source"
            ).fetchall()


if __name__ == "__main__":
    unittest.main()
