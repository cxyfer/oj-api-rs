import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

from utils.database import DailyChallengeDatabaseManager, ProblemsDatabaseManager


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

    def test_legacy_migration_converts_rows_and_slug_fallback(self):
        ProblemsDatabaseManager(self.db_path).update_problem(
            {
                "id": "42",
                "source": "leetcode",
                "slug": "two-sum",
                "title": "Two Sum",
                "title_cn": "兩數之和",
                "difficulty": "Easy",
                "ac_rate": 50.0,
                "rating": None,
                "contest": None,
                "problem_index": None,
                "tags": [],
                "link": "https://leetcode.com/problems/two-sum/",
                "category": "Algorithms",
                "paid_only": 0,
                "content": None,
                "content_cn": None,
                "similar_questions": [],
            },
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

    def _daily_rows(self):
        with sqlite3.connect(self.db_path) as conn:
            return conn.execute(
                "SELECT date, source, problems FROM daily_challenge ORDER BY date, source"
            ).fetchall()


if __name__ == "__main__":
    unittest.main()
