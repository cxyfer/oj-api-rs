import unittest

from codeforces import CodeforcesClient


class CodeforcesContestStandingsTests(unittest.IsolatedAsyncioTestCase):
    async def test_fetch_contest_problems_uses_contest_id_only(self):
        client = CodeforcesClient.__new__(CodeforcesClient)
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

        problems = await client.fetch_contest_problems(2214, object())

        self.assertEqual(
            requested_urls,
            ["https://codeforces.com/api/contest.standings?contestId=2214"],
        )
        self.assertEqual(len(problems), 1)
        self.assertEqual(problems[0]["id"], "2214A")
        self.assertEqual(problems[0]["contest"], "Codeforces Round 2214")


if __name__ == "__main__":
    unittest.main()
