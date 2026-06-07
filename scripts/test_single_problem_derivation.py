import unittest

from atcoder import AtCoderClient
from codeforces import CodeforcesClient
from luogu import LuoguClient


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

    def test_atcoder_url(self):
        self.assertEqual(
            AtCoderClient.problem_url_for_id("abc321_a"),
            "https://atcoder.jp/contests/abc321/tasks/abc321_a",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("abc100_arc100_a"),
            "https://atcoder.jp/contests/abc100/tasks/abc100_arc100_a",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("past201912_a"),
            "https://atcoder.jp/contests/past201912-open/tasks/past201912_a",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("arc058_abc042_a"),
            "https://atcoder.jp/contests/abc042/tasks/arc058_abc042_a",
        )
        self.assertEqual(
            AtCoderClient.problem_url_for_id("arc001_1"),
            "https://atcoder.jp/contests/arc001/tasks/arc001_1",
        )

    def test_atcoder_rejects_malformed_id(self):
        self.assertIsNone(AtCoderClient.problem_url_for_id("abc321"))
        self.assertIsNone(AtCoderClient.problem_url_for_id("abc321/a"))

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
