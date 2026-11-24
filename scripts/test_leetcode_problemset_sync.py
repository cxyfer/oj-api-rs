import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from leetcode import LeetCodeClient
from utils.database import ProblemsDatabaseManager


class _FakeResponse:
    def __init__(self, status_code, payload=None, text=None):
        self.status_code = status_code
        self.text = text if text is not None else json.dumps(payload)


class _FakeSession:
    def __init__(self, responses):
        self._responses = list(responses)
        self.requests = []

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc, traceback):
        return False

    async def get(self, url, **kwargs):
        self.requests.append((url, kwargs))
        response = self._responses.pop(0)
        if isinstance(response, Exception):
            raise response
        return response


class LeetCodeProblemsetSyncTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self._tmpdir = tempfile.TemporaryDirectory()
        self.db_path = str(Path(self._tmpdir.name) / "data.db")
        self.client = self._make_client()

    def tearDown(self):
        self._tmpdir.cleanup()

    def _make_client(self):
        client = LeetCodeClient.__new__(LeetCodeClient)
        client.problems_db = ProblemsDatabaseManager(self.db_path)
        client.ratings = {}
        client.ratings_last_update = 0
        client.max_retries = 3
        client.retry_delay = 0
        client.base_url = "https://leetcode.com"
        client._http_config = SimpleNamespace(user_agent="test-agent")
        return client

    async def test_fetch_category_problems_uses_curl_browser_impersonation(self):
        client = self._make_client()
        captured = {}

        def create_session(**kwargs):
            captured.update(kwargs)
            return _FakeSession(
                [_FakeResponse(200, payload=self._problemset_payload())]
            )

        client._create_curl_session = create_session

        problems = await client.fetch_category_problems("algorithms")

        self.assertEqual(captured["impersonate"], "chrome124")
        self.assertEqual(len(problems), 1)
        self.assertEqual(problems[0]["id"], 1)
        self.assertEqual(problems[0]["slug"], "two-sum")

    async def test_fetch_category_problems_raises_after_retries(self):
        client = self._make_client()
        client.max_retries = 2
        attempts = 0

        def create_session(**kwargs):
            nonlocal attempts
            attempts += 1
            return _FakeSession(
                [_FakeResponse(403, text="<title>Just a moment...</title>")]
            )

        client._create_curl_session = create_session

        with self.assertRaisesRegex(RuntimeError, "failed to fetch algorithms"):
            await client.fetch_category_problems("algorithms")
        self.assertEqual(attempts, 2)

    async def test_fetch_all_problems_propagates_category_failure(self):
        client = self._make_client()

        async def fetch_category(category):
            if category == "database":
                raise RuntimeError("HTTP 403")
            return [self._problem(id="1", slug="two-sum")]

        client.fetch_category_problems = fetch_category

        with self.assertRaisesRegex(RuntimeError, "HTTP 403"):
            await client.fetch_all_problems()

    def test_merge_problemset_ratings_normalizes_frontend_ids(self):
        problems = [
            {
                "id": "0001",
                "slug": "two-sum",
                "title": "Two Sum",
                "title_cn": "",
                "rating": 0,
                "contest": None,
                "problem_index": None,
            }
        ]
        ratings = {
            1: {
                "id": 1,
                "rating": 1200.5,
                "title_cn": "兩數之和",
                "contest": "weekly-contest-1",
                "problem_index": "A",
            }
        }

        merged = self.client.merge_problemset_ratings(problems, ratings)

        self.assertEqual(merged[0]["rating"], 1200.5)
        self.assertEqual(merged[0]["title_cn"], "兩數之和")
        self.assertEqual(merged[0]["contest"], "weekly-contest-1")
        self.assertEqual(merged[0]["problem_index"], "A")

    async def test_problemset_sync_refreshes_existing_zero_rating(self):
        self.client.problems_db.update_problemset_metadata(
            [self._problem(id="1", slug="two-sum", rating=0)]
        )

        async def fetch_all_problems():
            return [
                self._problem(id="1", slug="two-sum", rating=0, title="Two Sum Updated")
            ]

        async def fetch_ratings():
            return {
                1: {
                    "id": 1,
                    "rating": 1200.5,
                    "contest": "weekly-contest-1",
                    "problem_index": "A",
                }
            }

        self.client.fetch_all_problems = fetch_all_problems
        self.client.fetch_ratings = fetch_ratings

        await self.client.init_all_problems()

        stored = self.client.problems_db.get_problem(id="1", source="leetcode")
        self.assertEqual(stored["rating"], 1200.5)
        self.assertEqual(stored["title"], "Two Sum Updated")
        self.assertEqual(stored["contest"], "weekly-contest-1")
        self.assertEqual(stored["problem_index"], "A")

    async def test_rating_source_failure_preserves_positive_rating_and_persists_metadata(
        self,
    ):
        self.client.problems_db.update_problemset_metadata(
            [self._problem(id="1", slug="two-sum", rating=1500.0)]
        )

        async def fetch_all_problems():
            return [
                self._problem(
                    id="1", slug="two-sum", rating=0, title="Two Sum Updated"
                ),
                self._problem(id="2", slug="add-two-numbers", rating=0),
            ]

        self.client.fetch_all_problems = fetch_all_problems

        async def fail_fetch_ratings():
            raise RuntimeError("rating source unavailable")

        self.client.fetch_ratings = fail_fetch_ratings

        await self.client.init_all_problems()

        existing = self.client.problems_db.get_problem(id="1", source="leetcode")
        inserted = self.client.problems_db.get_problem(id="2", source="leetcode")
        self.assertEqual(existing["rating"], 1500.0)
        self.assertEqual(existing["title"], "Two Sum Updated")
        self.assertIsNotNone(inserted)
        self.assertEqual(inserted["rating"], 0.0)

    async def test_problemset_sync_preserves_existing_detail_fields(self):
        self.client.problems_db.update_problem(
            {
                **self._problem(id="1", slug="two-sum", rating=0),
                "content": "existing content",
                "content_cn": "既有內容",
                "tags": ["Array", "Hash Table"],
                "similar_questions": ["three-sum"],
            },
            force_update=True,
        )

        async def fetch_all_problems():
            return [self._problem(id="1", slug="two-sum", rating=1200.5)]

        async def fetch_ratings():
            return {}

        self.client.fetch_all_problems = fetch_all_problems
        self.client.fetch_ratings = fetch_ratings

        await self.client.init_all_problems()

        stored = self.client.problems_db.get_problem(id="1", source="leetcode")
        self.assertEqual(stored["content"], "existing content")
        self.assertEqual(stored["content_cn"], "既有內容")
        self.assertEqual(stored["tags"], ["Array", "Hash Table"])
        self.assertEqual(stored["similar_questions"], ["three-sum"])

    @staticmethod
    def _problem(id, slug, rating=0, title=None):
        return {
            "id": id,
            "slug": slug,
            "title": title or slug.replace("-", " ").title(),
            "title_cn": "",
            "difficulty": "Easy",
            "ac_rate": 50.0,
            "rating": rating,
            "contest": None,
            "problem_index": None,
            "tags": None,
            "link": f"https://leetcode.com/problems/{slug}/",
            "category": "Algorithms",
            "paid_only": 0,
            "content": None,
            "content_cn": None,
            "similar_questions": None,
        }

    @staticmethod
    def _problemset_payload():
        return {
            "num_total": 1,
            "stat_status_pairs": [
                {
                    "stat": {
                        "frontend_question_id": 1,
                        "question__title_slug": "two-sum",
                        "question__title": "Two Sum",
                        "question__hide": False,
                        "total_submitted": 100,
                        "total_acs": 50,
                    },
                    "difficulty": {"level": 1},
                    "paid_only": False,
                }
            ],
        }


if __name__ == "__main__":
    unittest.main()
