import tempfile
import unittest
from unittest.mock import MagicMock, patch

from codeforces import CodeforcesClient
from utils.config import CrawlerHttpConfig


class CodeforcesContestStandingsTests(unittest.IsolatedAsyncioTestCase):
    @patch("codeforces.ProblemsDatabaseManager")
    @patch("utils.base_crawler.get_config")
    async def test_fetch_contest_problems_uses_contest_id_only(
        self, mock_get_config, _mock_db
    ):
        mock_get_config.return_value.get_crawler_config.return_value = (
            CrawlerHttpConfig()
        )
        with tempfile.TemporaryDirectory() as tmpdir:
            client = CodeforcesClient(data_dir=tmpdir, db_path=":memory:")
        requested_urls = []

        async def fetch_json(_session, url):
            requested_urls.append(url)
            return {
                "status": "OK",
                "result": {
                    "contest": {"name": "Codeforces Round 2214"},
                    "problems": [
                        {
                            "contestId": 2214,
                            "index": "A",
                            "name": "Example Problem",
                            "tags": ["implementation"],
                        }
                    ],
                },
            }

        client._fetch_json = fetch_json

        problems = await client.fetch_contest_problems(2214, MagicMock())

        self.assertEqual(
            requested_urls,
            ["https://codeforces.com/api/contest.standings?contestId=2214"],
        )
        self.assertEqual(len(problems), 1)
        self.assertEqual(problems[0]["id"], "2214A")
        self.assertEqual(problems[0]["contest"], "Codeforces Round 2214")


if __name__ == "__main__":
    unittest.main()
