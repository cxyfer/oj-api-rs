import asyncio
import tempfile
import unittest
from pathlib import Path

from atcoder import AtCoderClient
from codeforces import CodeforcesClient
from luogu import LuoguClient
from utils.database import ProblemsDatabaseManager


class SingleProblemDerivationTests(unittest.TestCase):
    def test_codeforces_contest_url(self):
        self.assertEqual(
            CodeforcesClient.problem_url_for_id("1988A"),
            "https://codeforces.com/contest/1988/problem/A",
        )

    def test_codeforces_gym_url(self):
        self.assertEqual(
            CodeforcesClient.problem_url_for_id("102951A"),
            "https://codeforces.com/gym/102951/problem/A",
        )

    def test_codeforces_rejects_malformed_id(self):
        self.assertIsNone(CodeforcesClient.problem_url_for_id("1988"))
        self.assertIsNone(CodeforcesClient.problem_url_for_id("ABC"))

    def test_codeforces_fetch_single_problem_preserves_gym_key(self):
        client = object.__new__(CodeforcesClient)
        captured = {}

        class FakeSession:
            async def __aenter__(self):
                return object()

            async def __aexit__(self, exc_type, exc, traceback):
                return False

        class FakeDb:
            def update_problem(self, problem):
                captured.setdefault("problems", []).append(problem)
                return True

        async def fake_fetch_content_by_url(session, url):
            captured["url"] = url
            return "statement"

        client._create_curl_session = lambda **kwargs: FakeSession()
        client.fetch_content_by_url = fake_fetch_content_by_url
        client.problems_db = FakeDb()

        self.assertTrue(
            asyncio.run(
                client.fetch_single_problem("106539D", stored_problem_id="GYM106539D")
            )
        )
        self.assertEqual(
            captured["url"],
            "https://codeforces.com/gym/106539/problem/D",
        )
        self.assertEqual(
            [problem["id"] for problem in captured["problems"]], ["GYM106539D"]
        )
        self.assertEqual(captured["problems"][0]["slug"], "GYM106539D")
        self.assertEqual(captured["problems"][0]["contest"], "GYM106539")

    def test_codeforces_fetch_single_problem_updates_gym_snapshot_in_sqlite(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            problems_db = ProblemsDatabaseManager(str(Path(tmpdir) / "data.db"))
            self.assertTrue(
                problems_db.update_problem(
                    {
                        "id": "GYM106539D",
                        "source": "codeforces",
                        "slug": "GYM106539D",
                        "title": "GYM106539D",
                        "title_cn": "",
                        "difficulty": None,
                        "ac_rate": None,
                        "rating": None,
                        "contest": "GYM106539",
                        "problem_index": "D",
                        "tags": [],
                        "link": "https://codeforces.com/gym/106539/problem/D",
                        "category": "Algorithms",
                        "paid_only": 0,
                        "content": None,
                        "content_cn": None,
                        "similar_questions": [],
                    }
                )
            )
            client = object.__new__(CodeforcesClient)

            class FakeSession:
                async def __aenter__(self):
                    return object()

                async def __aexit__(self, exc_type, exc, traceback):
                    return False

            async def fake_fetch_content_by_url(session, url):
                self.assertEqual(url, "https://codeforces.com/gym/106539/problem/D")
                return "statement"

            client._create_curl_session = lambda **kwargs: FakeSession()
            client.fetch_content_by_url = fake_fetch_content_by_url
            client.problems_db = problems_db

            self.assertTrue(
                asyncio.run(
                    client.fetch_single_problem(
                        "106539D", stored_problem_id="GYM106539D"
                    )
                )
            )

            stored = problems_db.get_problem(id="GYM106539D", source="codeforces")
            self.assertIsNotNone(stored)
            self.assertEqual(stored["content"], "statement")
            self.assertIsNone(
                problems_db.get_problem(id="106539D", source="codeforces")
            )

    def test_codeforces_fetch_single_problem_rejects_prefixed_gym_id(self):
        client = object.__new__(CodeforcesClient)

        self.assertFalse(asyncio.run(client.fetch_single_problem("GYM106539D")))

    def test_atcoder_url(self):
        self.assertEqual(
            AtCoderClient.problem_url_for_id("abc042_a"),
            "https://atcoder.jp/contests/abc042/tasks/abc042_a",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("arc001_1"),
            "https://atcoder.jp/contests/arc001/tasks/arc001_1",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("past201912_a"),
            "https://atcoder.jp/contests/past201912-open/tasks/past201912_a",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("ndpc/ndpc2026_m"),
            "https://atcoder.jp/contests/ndpc/tasks/ndpc2026_m",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("ndpc/tasks/ndpc2026_m"),
            "https://atcoder.jp/contests/ndpc/tasks/ndpc2026_m",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("arc058_abc042_a"),
            "https://atcoder.jp/contests/arc058/tasks/arc058_abc042_a",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("abc042/arc058_abc042_a"),
            "https://atcoder.jp/contests/abc042/tasks/arc058_abc042_a",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("abc042/aaabbb_aaabbb_ccc"),
            "https://atcoder.jp/contests/abc042/tasks/aaabbb_aaabbb_ccc",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("abc042/tasks/aaabbb_aaabbb_ccc"),
            "https://atcoder.jp/contests/abc042/tasks/aaabbb_aaabbb_ccc",
        )

    def test_atcoder_fetch_single_problem_stores_normalized_id(self):
        client = object.__new__(AtCoderClient)
        captured = {}

        class FakeSession:
            async def __aenter__(self):
                return object()

            async def __aexit__(self, exc_type, exc, traceback):
                return False

        class FakeDb:
            def update_problem(self, problem):
                captured["problem"] = problem
                return True

        async def fake_fetch_content_by_url(session, url):
            captured["url"] = url
            return "statement"

        client._create_aiohttp_session = FakeSession
        client.fetch_content_by_url = fake_fetch_content_by_url
        client.problems_db = FakeDb()

        self.assertTrue(
            asyncio.run(client.fetch_single_problem("abc042/aaabbb_aaabbb_ccc"))
        )
        self.assertEqual(
            captured["url"],
            "https://atcoder.jp/contests/abc042/tasks/aaabbb_aaabbb_ccc",
        )
        self.assertEqual(captured["problem"]["id"], "aaabbb_aaabbb_ccc")
        self.assertEqual(captured["problem"]["slug"], "aaabbb_aaabbb_ccc")
        self.assertEqual(captured["problem"]["contest"], "abc042")
        self.assertEqual(
            captured["problem"]["link"],
            "https://atcoder.jp/contests/abc042/tasks/aaabbb_aaabbb_ccc",
        )

    def test_atcoder_rejects_malformed_id(self):
        self.assertIsNone(AtCoderClient.problem_url_for_id("abc321"))
        self.assertIsNone(AtCoderClient.problem_url_for_id("abc321/a"))
        self.assertIsNone(AtCoderClient.problem_url_for_id("../abc321_a"))
        self.assertIsNone(AtCoderClient.problem_url_for_id("ndpc/problems/ndpc2026_m"))
        self.assertIsNone(AtCoderClient.problem_url_for_id("ndpc/tasks/../ndpc2026_m"))

    def test_luogu_url(self):
        self.assertEqual(
            LuoguClient.problem_url_for_id("P1083"),
            "https://www.luogu.com.cn/problem/P1083",
        )
        self.assertEqual(
            LuoguClient.problem_url_for_id("p1083"),
            "https://www.luogu.com.cn/problem/P1083",
        )

    def test_luogu_rejects_malformed_id(self):
        self.assertIsNone(LuoguClient.problem_url_for_id("1083"))
        self.assertIsNone(LuoguClient.problem_url_for_id("P"))


if __name__ == "__main__":
    unittest.main()
