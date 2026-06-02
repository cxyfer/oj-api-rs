import json
import os
import sqlite3
import threading
import time
from pathlib import Path

from .logger import get_database_logger

# Module-level logger
logger = get_database_logger()


def _normalize_json_string_list(values):
    """Normalize list-like inputs from crawlers or stored JSON strings."""
    if not values:
        return []
    if isinstance(values, str):
        try:
            parsed_values = json.loads(values)
        except json.JSONDecodeError:
            values = [values]
        else:
            if isinstance(parsed_values, list):
                values = parsed_values
            else:
                values = [parsed_values]
    return [str(value).strip() for value in values if str(value).strip()]


def _normalize_similar_questions(values):
    if isinstance(values, str):
        try:
            values = json.loads(values)
        except json.JSONDecodeError:
            return []

    if not values:
        return []

    normalized = []
    for value in values:
        if isinstance(value, str):
            slug = value.strip()
        elif isinstance(value, dict):
            slug = (
                value.get("titleSlug")
                or value.get("title_slug")
                or value.get("slug")
                or ""
            ).strip()
        else:
            slug = ""
        if slug:
            normalized.append(slug)
    return normalized


def _prefer_non_empty(new_value, existing_value):
    if isinstance(new_value, list):
        return new_value if new_value else (existing_value or [])
    if new_value in (None, ""):
        return existing_value
    return new_value


class SettingsDatabaseManager:
    """
    This class manages server settings in the database.
    """

    def __init__(self, db_path="data/settings.db"):
        """
        Initialize the database manager

        Args:
            db_path (str): The path to the database file
        """

        self.db_path = db_path
        Path(os.path.dirname(db_path)).mkdir(parents=True, exist_ok=True)
        self._init_db()
        logger.info(f"Database manager initialized with database at {db_path}")

    def _init_db(self):
        """Initialize the database, create necessary tables"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        # Create server settings table
        cursor.execute("""
        CREATE TABLE IF NOT EXISTS server_settings (
            server_id INTEGER PRIMARY KEY,
            channel_id INTEGER NOT NULL,
            role_id INTEGER,
            post_time TEXT DEFAULT '00:00',
            timezone TEXT DEFAULT 'UTC',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        """)

        conn.commit()
        conn.close()
        logger.debug("Database tables initialized")

    def get_server_settings(self, server_id):
        """Get the settings for a specific server

        Args:
            server_id (int): Discord server ID

            Returns:
                dict: server settings, return None if not found
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute(
            "SELECT channel_id, role_id, post_time, timezone FROM server_settings WHERE server_id = ?",
            (server_id,),
        )
        result = cursor.fetchone()
        conn.close()

        if result:
            logger.debug(f"Server {server_id} settings: {result}")
            return {
                "server_id": server_id,
                "channel_id": result[0],
                "role_id": result[1],
                "post_time": result[2],
                "timezone": result[3],
            }
        return None

    def set_server_settings(
        self, server_id, channel_id, role_id=None, post_time="00:00", timezone="UTC"
    ):
        """Set or update server settings

        Args:
            server_id (int): Discord server ID
            channel_id (int): The channel ID to send the daily challenge
            role_id (int, optional): The role ID to mention
            post_time (str, optional): The time to send the daily challenge, format "HH:MM"
            timezone (str, optional): The timezone name

        Returns:
            bool: return True if updated successfully
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        try:
            cursor.execute(
                """
                INSERT INTO server_settings (server_id, channel_id, role_id, post_time, timezone)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(server_id) DO UPDATE SET
                    channel_id = excluded.channel_id,
                    role_id = excluded.role_id,
                    post_time = excluded.post_time,
                    timezone = excluded.timezone,
                    updated_at = CURRENT_TIMESTAMP
                """,
                (server_id, channel_id, role_id, post_time, timezone),
            )
            conn.commit()
            return True
        except Exception as e:
            logger.error(f"Error setting server settings: {e}")
            return False
        finally:
            logger.debug(
                f"Server {server_id} settings updated: ({channel_id}, {role_id}, {post_time}, {timezone})"
            )
            conn.close()

    def set_channel(self, server_id, channel_id):
        """Update the server notification channel

        Args:
            server_id (int): Discord server ID
            channel_id (int): The channel ID

        Returns:
            bool: return True if updated successfully
        """
        settings = self.get_server_settings(server_id)
        if settings:
            return self.set_server_settings(
                server_id,
                channel_id,
                settings.get("role_id"),
                settings.get("post_time", "00:00"),
                settings.get("timezone", "UTC"),
            )
        else:
            return self.set_server_settings(server_id, channel_id)

    def set_role(self, server_id, role_id):
        """Update the server notification role

        Args:
            server_id (int): Discord server ID
            role_id (int): The role ID

        Returns:
            bool: return True if updated successfully
        """
        settings = self.get_server_settings(server_id)
        if settings:
            return self.set_server_settings(
                server_id,
                settings.get("channel_id"),
                role_id,
                settings.get("post_time", "00:00"),
                settings.get("timezone", "UTC"),
            )
        return False  # return False if server settings not found

    def set_post_time(self, server_id, post_time):
        """Update the server notification time

        Args:
            server_id (int): Discord server ID
            post_time (str): The time to send the daily challenge, format "HH:MM"

        Returns:
            bool: return True if updated successfully
        """
        settings = self.get_server_settings(server_id)
        if settings:
            return self.set_server_settings(
                server_id,
                settings.get("channel_id"),
                settings.get("role_id"),
                post_time,
                settings.get("timezone", "UTC"),
            )
        return False  # return False if server settings not found

    def set_timezone(self, server_id, timezone):
        """Update the server notification timezone

        Args:
            server_id (int): Discord server ID
            timezone (str): The timezone name

        Returns:
            bool: return True if updated successfully
        """
        settings = self.get_server_settings(server_id)
        if settings:
            return self.set_server_settings(
                server_id,
                settings.get("channel_id"),
                settings.get("role_id"),
                settings.get("post_time", "00:00"),
                timezone,
            )
        return False  # return False if server settings not found

    def get_all_servers(self):
        """Get all servers with settings

        Returns:
            list: A list of dictionaries containing all server settings
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute(
            "SELECT server_id, channel_id, role_id, post_time, timezone FROM server_settings"
        )
        results = cursor.fetchall()
        conn.close()

        servers = []
        for row in results:
            servers.append(
                {
                    "server_id": row[0],
                    "channel_id": row[1],
                    "role_id": row[2],
                    "post_time": row[3],
                    "timezone": row[4],
                }
            )

        return servers

    def delete_server_settings(self, server_id):
        """Delete server settings

        Args:
            server_id (int): Discord server ID

        Returns:
            bool: return True if deleted successfully
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        try:
            cursor.execute(
                "DELETE FROM server_settings WHERE server_id = ?", (server_id,)
            )
            conn.commit()
            return cursor.rowcount > 0
        except Exception as e:
            logger.error(f"Error deleting server settings: {e}")
            return False
        finally:
            conn.close()


class ProblemsDatabaseManager:
    """
    Manage LeetCode problem data database operations
    """

    def __init__(self, db_path="data/data.db"):
        self.db_path = db_path
        Path(os.path.dirname(db_path)).mkdir(parents=True, exist_ok=True)
        self._init_db()
        logger.info(f"Problems DB manager initialized with database at {db_path}")

    def _init_db(self):
        """Create problems table"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute("""
        CREATE TABLE IF NOT EXISTS problems (
            id TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'leetcode',
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
            PRIMARY KEY (source, id)
        )
        """)
        cursor.execute(
            "CREATE INDEX IF NOT EXISTS idx_problems_source_slug ON problems(source, slug)"
        )
        conn.commit()
        conn.close()
        logger.debug("Problems table initialized")

    def update_problems(self, problems, force_update=False):
        """
        Insert or update problem data in batch.

        When force_update is False (default), existing problems are skipped
        (INSERT OR IGNORE). When force_update is True, existing problems are
        overwritten (upsert via ON CONFLICT DO UPDATE).

        Args:
            problems (list[dict]): problem data list
            force_update (bool): if True, overwrite existing problems

        Returns:
            int: actual inserted/updated data count
        """
        total_count = len(problems)
        if total_count == 0:
            return 0

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        # Prepare data to insert
        values = []
        for problem in problems:
            problem_id = problem.get("id")
            problem_source = problem.get("source") or "leetcode"
            values.append(
                (
                    str(problem_id) if problem_id is not None else None,
                    problem_source,
                    problem.get("slug"),
                    problem.get("title"),
                    problem.get("title_cn"),
                    problem.get("difficulty"),
                    problem.get("ac_rate"),
                    problem.get("rating"),
                    problem.get("contest"),
                    problem.get("problem_index"),
                    json.dumps(_normalize_json_string_list(problem.get("tags"))),
                    problem.get("link"),
                    problem.get("category"),
                    problem.get("paid_only"),
                    problem.get("content"),
                    problem.get("content_cn"),
                    json.dumps(
                        _normalize_similar_questions(problem.get("similar_questions"))
                    ),
                )
            )

        if force_update:
            sql = """
            INSERT INTO problems (
                id, source, slug, title, title_cn, difficulty, ac_rate,
                rating, contest, problem_index, tags, link,
                category, paid_only, content, content_cn, similar_questions
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(source, id) DO UPDATE SET
                slug=excluded.slug,
                title=excluded.title,
                title_cn=excluded.title_cn,
                difficulty=excluded.difficulty,
                ac_rate=excluded.ac_rate,
                rating=excluded.rating,
                contest=excluded.contest,
                problem_index=excluded.problem_index,
                tags=excluded.tags,
                link=excluded.link,
                category=excluded.category,
                paid_only=excluded.paid_only,
                content=COALESCE(NULLIF(excluded.content, ''), problems.content),
                content_cn=COALESCE(NULLIF(excluded.content_cn, ''), problems.content_cn),
                similar_questions=CASE
                    WHEN excluded.similar_questions IS NULL OR excluded.similar_questions = '[]'
                        THEN problems.similar_questions
                    ELSE excluded.similar_questions
                END
            """
        else:
            sql = """
            INSERT OR IGNORE INTO problems (
                id, source, slug, title, title_cn, difficulty, ac_rate,
                rating, contest, problem_index, tags, link,
                category, paid_only, content, content_cn, similar_questions
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """
        try:
            cursor.executemany(sql, values)

            conn.commit()

            affected_count = cursor.rowcount
            verb = "upserted" if force_update else "inserted"
            skip_verb = "updated" if force_update else "ignored"

            logger.info(
                f"Batch {verb} {affected_count}/{total_count} problems "
                f"({skip_verb} {total_count - affected_count} existing problems)"
            )
            return affected_count

        except Exception as e:
            logger.error(f"Error inserting problems: {e}")
            return 0
        finally:
            conn.close()

    def update_problemset_metadata(self, problems, source="leetcode"):
        total_count = len(problems)
        if total_count == 0:
            return 0

        values = []
        for problem in problems:
            problem_id = problem.get("id")
            if problem_id is None or str(problem_id).strip() == "":
                continue
            values.append(
                (
                    str(problem_id),
                    source,
                    problem.get("slug"),
                    problem.get("title"),
                    problem.get("title_cn"),
                    problem.get("difficulty"),
                    problem.get("ac_rate"),
                    problem.get("rating"),
                    problem.get("contest"),
                    problem.get("problem_index"),
                    json.dumps(_normalize_json_string_list(problem.get("tags"))),
                    problem.get("link"),
                    problem.get("category"),
                    problem.get("paid_only"),
                    problem.get("content"),
                    problem.get("content_cn"),
                    json.dumps(
                        _normalize_similar_questions(problem.get("similar_questions"))
                    ),
                )
            )

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        try:
            cursor.executemany(
                """
                INSERT INTO problems (
                    id, source, slug, title, title_cn, difficulty, ac_rate,
                    rating, contest, problem_index, tags, link,
                    category, paid_only, content, content_cn, similar_questions
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(source, id) DO UPDATE SET
                    slug=COALESCE(NULLIF(excluded.slug, ''), problems.slug),
                    title=COALESCE(NULLIF(excluded.title, ''), problems.title),
                    title_cn=COALESCE(NULLIF(excluded.title_cn, ''), problems.title_cn),
                    difficulty=COALESCE(NULLIF(excluded.difficulty, ''), problems.difficulty),
                    ac_rate=COALESCE(excluded.ac_rate, problems.ac_rate),
                    rating=CASE
                        WHEN excluded.rating IS NOT NULL AND excluded.rating > 0 THEN excluded.rating
                        WHEN problems.rating IS NOT NULL AND problems.rating > 0 THEN problems.rating
                        ELSE COALESCE(excluded.rating, problems.rating)
                    END,
                    contest=COALESCE(NULLIF(excluded.contest, ''), problems.contest),
                    problem_index=COALESCE(NULLIF(excluded.problem_index, ''), problems.problem_index),
                    tags=CASE
                        WHEN excluded.tags IS NULL OR excluded.tags = '[]' THEN problems.tags
                        ELSE excluded.tags
                    END,
                    link=COALESCE(NULLIF(excluded.link, ''), problems.link),
                    category=COALESCE(NULLIF(excluded.category, ''), problems.category),
                    paid_only=COALESCE(excluded.paid_only, problems.paid_only),
                    content=COALESCE(NULLIF(excluded.content, ''), problems.content),
                    content_cn=COALESCE(NULLIF(excluded.content_cn, ''), problems.content_cn),
                    similar_questions=CASE
                        WHEN excluded.similar_questions IS NULL OR excluded.similar_questions = '[]'
                            THEN problems.similar_questions
                        ELSE excluded.similar_questions
                    END
                """,
                values,
            )
            conn.commit()
            affected_count = cursor.rowcount
            logger.info(
                f"Batch refreshed metadata for {affected_count}/{total_count} {source} problems"
            )
            return affected_count
        except Exception as e:
            logger.error(f"Error refreshing problem metadata: {e}")
            return 0
        finally:
            conn.close()

    def update_problem(self, problem, force_update=False):
        """
        Insert or update single problem data.

        Args:
            problem (dict): problem data, must contain id or slug field for identification
            force_update (bool, optional): force update all fields. If False, empty values will not overwrite
                                       existing data. Default is False.

        Returns:
            bool: True if update succeeded, False otherwise

        Raises:
            ValueError: when problem parameter doesn't contain id field
        """
        # Check if id exists to identify the problem
        problem_id = problem.get("id")
        problem_source = problem.get("source") or "leetcode"

        if not problem_id:
            raise ValueError("Problem must have 'id' field for identification")
        problem_id = str(problem_id)
        problem["source"] = problem_source

        problem["tags"] = _normalize_json_string_list(problem.get("tags"))
        problem["similar_questions"] = _normalize_similar_questions(
            problem.get("similar_questions")
        )

        # If not force update, get existing data and merge
        if not force_update:
            existing_problem = self.get_problem(id=problem_id, source=problem_source)
            if existing_problem:
                for key in existing_problem:
                    if key == "id":
                        continue
                    problem[key] = _prefer_non_empty(
                        problem.get(key), existing_problem.get(key)
                    )

                problem["tags"] = _normalize_json_string_list(problem.get("tags"))
                problem["similar_questions"] = _normalize_similar_questions(
                    problem.get("similar_questions")
                )

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        try:
            cursor.execute(
                """
            INSERT OR REPLACE INTO problems (
                id, source, slug, title, title_cn, difficulty, ac_rate,
                rating, contest, problem_index, tags, link,
                category, paid_only, content, content_cn, similar_questions
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
                (
                    problem_id,
                    problem_source,
                    problem.get("slug"),
                    problem.get("title"),
                    problem.get("title_cn"),
                    problem.get("difficulty"),
                    problem.get("ac_rate"),
                    problem.get("rating"),
                    problem.get("contest"),
                    problem.get("problem_index"),
                    json.dumps(problem.get("tags", [])),
                    problem.get("link"),
                    problem.get("category"),
                    problem.get("paid_only"),
                    problem.get("content"),
                    problem.get("content_cn"),
                    json.dumps(problem.get("similar_questions", [])),
                ),
            )

            conn.commit()
            logger.debug(
                "Updated problem with source=%s, id=%s, force_update=%s",
                problem_source,
                problem_id,
                force_update,
            )
            return True

        except Exception as e:
            logger.error(f"Error updating problem: {e}")
            return False
        finally:
            conn.close()

    def get_problem(self, id=None, slug=None, source="leetcode"):
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        if id:
            cursor.execute(
                "SELECT * FROM problems WHERE source = ? AND id = ?",
                (source, str(id)),
            )
        elif slug:
            cursor.execute(
                "SELECT * FROM problems WHERE source = ? AND slug = ?",
                (source, slug),
            )
        row = cursor.fetchone()
        conn.close()
        if row:
            problem = self._row_to_dict(row)
            problem["tags"] = _normalize_json_string_list(
                json.loads(problem["tags"]) if problem["tags"] else []
            )
            problem["similar_questions"] = _normalize_similar_questions(
                problem["similar_questions"]
            )
            return problem
        return None

    def get_problem_contents(self, source: str) -> list[tuple[str, str]]:
        """Get (id, content) pairs for problems with content."""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute(
            """
            SELECT id, content
            FROM problems
            WHERE source = ?
              AND content IS NOT NULL
              AND content != ''
            ORDER BY id ASC
            """,
            (source,),
        )
        rows = cursor.fetchall()
        conn.close()
        return [(str(row[0]), row[1]) for row in rows] if rows else []

    def batch_update_content(
        self, updates: list[tuple[str, str, str]], batch_size: int = 100
    ) -> tuple[int, bool]:
        """
        Batch update problem content.

        Args:
            updates: List of (content, source, id) tuples
            batch_size: Number of updates per transaction

        Returns:
            Tuple of (rows_updated, success). If success is False, some
            updates may have failed and the caller should consider retrying.
        """
        if not updates:
            return 0, True

        if batch_size < 1:
            batch_size = 100

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        total_updated = 0

        try:
            for i in range(0, len(updates), batch_size):
                batch = updates[i : i + batch_size]
                cursor.executemany(
                    "UPDATE problems SET content = ? WHERE source = ? AND id = ?",
                    batch,
                )
                total_updated += cursor.rowcount
                conn.commit()
            return total_updated, True
        except Exception as e:
            logger.error("Error batch updating content: %s", e)
            conn.rollback()
            return total_updated, False
        finally:
            conn.close()

    def get_problem_ids_missing_content(self, source="leetcode"):
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute(
            """
            SELECT id
            FROM problems
            WHERE source = ?
              AND (content IS NULL OR content = '')
              AND category = 'Algorithms'
              AND paid_only = 0
            -- id 為 TEXT，排序為字典序；若需數值排序請另行轉型
            ORDER BY id ASC
            """,
            (source,),
        )
        rows = cursor.fetchall()
        conn.close()
        return [str(row[0]) for row in rows] if rows else []

    def count_missing_content(self, source="leetcode") -> int:
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute(
            """
            SELECT COUNT(*)
            FROM problems
            WHERE source = ?
              AND (content IS NULL OR content = '')
              AND category = 'Algorithms'
              AND paid_only = 0
            """,
            (source,),
        )
        row = cursor.fetchone()
        conn.close()
        return int(row[0]) if row else 0

    def get_problems_missing_content(self, source: str) -> list[tuple[str, str]]:
        """Get (id, link) pairs for problems missing content."""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute(
            """
            SELECT id, link
            FROM problems
            WHERE source = ?
              AND (content IS NULL OR content = '')
              AND link IS NOT NULL AND link != ''
            ORDER BY id ASC
            """,
            (source,),
        )
        rows = cursor.fetchall()
        conn.close()
        return [(str(row[0]), row[1]) for row in rows] if rows else []

    def _row_to_dict(self, row):
        keys = [
            "id",
            "source",
            "slug",
            "title",
            "title_cn",
            "difficulty",
            "ac_rate",
            "rating",
            "contest",
            "problem_index",
            "tags",
            "link",
            "category",
            "paid_only",
            "content",
            "content_cn",
            "similar_questions",
        ]
        return dict(zip(keys, row))


class EmbeddingDatabaseManager:
    """
    管理 embeddings 相關資料表與 sqlite-vec 連線
    """

    def __init__(self, db_path="data/data.db"):
        self.db_path = db_path
        Path(os.path.dirname(db_path)).mkdir(parents=True, exist_ok=True)
        self._lock = threading.Lock()
        self._conn = self._create_connection()
        self._ensure_metadata_table()
        logger.info(f"Embedding DB manager initialized with database at {db_path}")

    def _create_connection(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self.db_path, check_same_thread=False)
        conn.enable_load_extension(True)
        try:
            import sqlite_vec
        except ImportError as exc:
            raise RuntimeError("sqlite-vec is required for embeddings") from exc
        sqlite_vec.load(conn)
        conn.enable_load_extension(False)
        return conn

    def _ensure_metadata_table(self) -> None:
        self.execute(
            """
            CREATE TABLE IF NOT EXISTS problem_embeddings (
                source TEXT NOT NULL,
                problem_id TEXT NOT NULL,
                rewritten_content TEXT,
                model TEXT NOT NULL,
                dim INTEGER NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (source, problem_id)
            )
            """,
            commit=True,
        )

    def create_vec_table(self, dim: int) -> None:
        if not isinstance(dim, int) or isinstance(dim, bool) or dim <= 0:
            raise ValueError("dim must be a positive integer")
        self.execute(
            f"""
            CREATE VIRTUAL TABLE IF NOT EXISTS vec_embeddings USING vec0(
                source TEXT,
                problem_id TEXT,
                embedding float[{dim}]
            )
            """,
            commit=True,
        )

    def check_dimension_consistency(self, dim: int) -> bool:
        try:
            row = self.execute(
                "SELECT vec_length(embedding) FROM vec_embeddings LIMIT 1",
                fetchone=True,
            )
            if row and row[0] != dim:
                return False
        except sqlite3.OperationalError:
            return False
        return True

    def execute(
        self,
        query: str,
        params: tuple = (),
        commit: bool = False,
        fetchone: bool = False,
        fetchall: bool = False,
    ):
        with self._lock:
            cursor = self._conn.execute(query, params)
            if commit:
                self._conn.commit()
            if fetchone:
                return cursor.fetchone()
            if fetchall:
                return cursor.fetchall()
            return None

    def executemany(self, query: str, seq, commit: bool = False) -> int:
        with self._lock:
            cursor = self._conn.executemany(query, seq)
            if commit:
                self._conn.commit()
            return cursor.rowcount

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        self.close()
        return False

    def close(self) -> None:
        with self._lock:
            self._conn.close()


class LLMTranslateDatabaseManager:
    """
    管理 LLM 翻譯結果的資料庫操作
    """

    def __init__(self, db_path="data/data.db", expire_seconds=604800):
        self.db_path = db_path
        self.expire_seconds = expire_seconds
        Path(os.path.dirname(db_path)).mkdir(parents=True, exist_ok=True)
        self._init_db()
        logger.info(
            f"LLMTranslate DB manager initialized with database at {db_path}, expire_seconds={expire_seconds}"
        )

    def _init_db(self):
        """建立 llm_translate_results 資料表"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute("""
        CREATE TABLE IF NOT EXISTS llm_translate_results (
            problem_id INTEGER NOT NULL,
            domain TEXT NOT NULL,
            translation TEXT,
            created_at INTEGER NOT NULL,
            model_name TEXT,
            PRIMARY KEY (problem_id, domain)
        )
        """)
        conn.commit()
        conn.close()
        logger.debug("llm_translate_results table initialized")

    def get_translation(self, problem_id, domain, expire_seconds=None):
        """
        查詢翻譯結果，若超過 expire_seconds 則回傳 None
        """
        if expire_seconds is None:
            expire_seconds = self.expire_seconds

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute(
            "SELECT translation, created_at, model_name FROM llm_translate_results WHERE problem_id = ? AND domain = ?",
            (problem_id, domain),
        )
        row = cursor.fetchone()
        conn.close()

        if row:
            translation, created_at, model_name = row
            now = int(time.time())
            if now - created_at <= expire_seconds:
                return {"translation": translation, "model_name": model_name}
        return None

    def save_translation(self, problem_id, domain, translation, model_name=None):
        """
        寫入或覆蓋翻譯結果，使用 UNIX timestamp 儲存時間

        Args:
            problem_id (int): 題目ID
            domain (str): 網域 (com/cn)
            translation (str): 翻譯內容
            model_name (str, optional): 使用的模型名稱
        """
        now = int(time.time())

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        if translation is None:
            translation = ""
        elif isinstance(translation, (dict, list)):
            translation = json.dumps(translation, ensure_ascii=False)
        else:
            translation = str(translation)
        cursor.execute(
            """
        INSERT OR REPLACE INTO llm_translate_results (problem_id, domain, translation, created_at, model_name)
        VALUES (?, ?, ?, ?, ?)
        """,
            (problem_id, domain, translation, now, model_name),
        )
        conn.commit()
        conn.close()
        logger.info(
            f"Saved LLM translation for problem_id={problem_id}, domain={domain}, model={model_name}"
        )


class LLMInspireDatabaseManager:
    """
    管理 LLM 靈感啟發結果的資料庫操作
    """

    def __init__(self, db_path="data/data.db", expire_seconds=604800):
        self.db_path = db_path
        self.expire_seconds = expire_seconds
        Path(os.path.dirname(db_path)).mkdir(parents=True, exist_ok=True)
        self._init_db()
        logger.info(
            f"LLMInspire DB manager initialized with database at {db_path}, expire_seconds={expire_seconds}"
        )

    def _init_db(self):
        """建立 llm_inspire_results 資料表"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute("""
        CREATE TABLE IF NOT EXISTS llm_inspire_results (
            problem_id INTEGER NOT NULL,
            domain TEXT NOT NULL,
            thinking TEXT,
            traps TEXT,
            algorithms TEXT,
            inspiration TEXT,
            created_at INTEGER NOT NULL,
            model_name TEXT,
            PRIMARY KEY (problem_id, domain)
        )
        """)
        conn.commit()
        conn.close()
        logger.debug("llm_inspire_results table initialized")

    def get_inspire(self, problem_id, domain, expire_seconds=None):
        """
        查詢靈感啟發結果，若超過 expire_seconds（預設使用初始化時設定的值）則回傳 None
        """
        if expire_seconds is None:
            expire_seconds = self.expire_seconds

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute(
            "SELECT thinking, traps, algorithms, inspiration, created_at, model_name "
            "FROM llm_inspire_results WHERE problem_id = ? AND domain = ?",
            (problem_id, domain),
        )
        row = cursor.fetchone()
        conn.close()

        if row:
            thinking, traps, algorithms, inspiration, created_at, model_name = row
            now = int(time.time())
            if now - created_at <= expire_seconds:
                return {
                    "thinking": thinking,
                    "traps": traps,
                    "algorithms": algorithms,
                    "inspiration": inspiration,
                    "model_name": model_name,
                }
        return None

    def save_inspire(
        self,
        problem_id,
        domain,
        thinking,
        traps,
        algorithms,
        inspiration,
        model_name=None,
    ):
        """
        寫入或覆蓋靈感啟發結果，使用 UNIX timestamp 儲存時間

        Args:
            problem_id (int): 題目ID
            domain (str): 網域 (com/cn)
            thinking (str): 思路內容
            traps (str): 陷阱內容
            algorithms (str): 演算法內容
            inspiration (str): 靈感內容
            model_name (str, optional): 使用的模型名稱
        """
        now = int(time.time())

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        # 確保所有欄位都是 str
        def safe_str(val):
            if val is None:
                return ""
            if isinstance(val, (dict, list)):
                return json.dumps(val, ensure_ascii=False)
            return str(val)

        cursor.execute(
            """
        INSERT OR REPLACE INTO llm_inspire_results
        (problem_id, domain, thinking, traps, algorithms, inspiration, created_at, model_name)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """,
            (
                problem_id,
                domain,
                safe_str(thinking),
                safe_str(traps),
                safe_str(algorithms),
                safe_str(inspiration),
                now,
                model_name,
            ),
        )
        conn.commit()
        conn.close()
        logger.info(
            f"Saved LLM inspire for problem_id={problem_id}, domain={domain}, model={model_name}"
        )

    def _row_to_dict(self, row):
        keys = [
            "id",
            "slug",
            "title",
            "title_cn",
            "difficulty",
            "ac_rate",
            "rating",
            "contest",
            "problem_index",
            "tags",
            "link",
            "category",
            "paid_only",
            "content",
            "content_cn",
            "similar_questions",
        ]
        return dict(zip(keys, row))


class DailyChallengeDatabaseManager:
    """
    Manage LeetCode daily challenge data database operations
    """

    def __init__(self, db_path="data/data.db"):
        self.db_path = db_path
        Path(os.path.dirname(db_path)).mkdir(parents=True, exist_ok=True)
        self._init_db()
        logger.info(f"DailyChallenge DB manager initialized with database at {db_path}")

    def _init_db(self):
        """Create or migrate daily_challenge table."""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        try:
            schema = self._detect_schema(cursor)
            if schema == "missing":
                self._create_compact_table(cursor)
            elif schema == "legacy":
                self._migrate_legacy_table(conn)
            elif schema != "compact":
                raise RuntimeError("unsupported daily_challenge schema")
            conn.commit()
        finally:
            conn.close()
        logger.debug("DailyChallenge table initialized")

    @staticmethod
    def _detect_schema(cursor):
        cursor.execute("PRAGMA table_info(daily_challenge)")
        columns = {row[1] for row in cursor.fetchall()}
        if not columns:
            return "missing"
        if {"date", "source", "problems"}.issubset(columns):
            return "compact"
        if {"date", "domain", "id", "slug"}.issubset(columns):
            return "legacy"
        return "unsupported"

    @staticmethod
    def _create_compact_table(cursor):
        cursor.execute("""
        CREATE TABLE IF NOT EXISTS daily_challenge (
            date TEXT NOT NULL,
            source TEXT NOT NULL,
            problems TEXT NOT NULL,
            PRIMARY KEY (date, source)
        )
        """)

    @staticmethod
    def _create_problems_table(cursor):
        cursor.execute("""
        CREATE TABLE IF NOT EXISTS problems (
            id TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'leetcode',
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
            PRIMARY KEY (source, id)
        )
        """)
        cursor.execute(
            "CREATE INDEX IF NOT EXISTS idx_problems_source_slug ON problems(source, slug)"
        )

    def _migrate_legacy_table(self, conn):
        cursor = conn.cursor()
        self._create_problems_table(cursor)
        cursor.execute("DROP TABLE IF EXISTS daily_challenge_legacy_migration")
        cursor.execute(
            "ALTER TABLE daily_challenge RENAME TO daily_challenge_legacy_migration"
        )
        self._create_compact_table(cursor)
        rows = self._legacy_daily_rows(cursor)
        for row in rows:
            date = row["date"]
            domain = row["domain"]
            resolved_id = self._legacy_problem_id(
                cursor, row.get("id"), row.get("slug")
            )
            if resolved_id is None:
                logger.warning(
                    "Skipping unconvertible legacy daily challenge row for %s %s",
                    date,
                    domain,
                )
                continue
            self._seed_legacy_problem_if_missing(cursor, resolved_id, row)
            cursor.execute(
                """
                INSERT OR REPLACE INTO daily_challenge (date, source, problems)
                VALUES (?, ?, ?)
                """,
                (
                    date,
                    self._daily_source_from_domain(domain),
                    json.dumps([f"leetcode:{resolved_id}"]),
                ),
            )
        cursor.execute("DROP TABLE daily_challenge_legacy_migration")

    @staticmethod
    def _legacy_daily_rows(cursor):
        cursor.execute("PRAGMA table_info(daily_challenge_legacy_migration)")
        columns = {row[1] for row in cursor.fetchall()}
        optional_columns = [
            "title",
            "title_cn",
            "difficulty",
            "ac_rate",
            "rating",
            "contest",
            "problem_index",
            "tags",
            "link",
            "category",
            "paid_only",
            "content",
            "content_cn",
            "similar_questions",
        ]
        selected = ["date", "domain", "id", "slug"]
        selected.extend(
            column if column in columns else f"NULL AS {column}"
            for column in optional_columns
        )
        cursor.execute(
            f"SELECT {', '.join(selected)} FROM daily_challenge_legacy_migration "
            "ORDER BY date, domain"
        )
        names = [description[0] for description in cursor.description]
        return [dict(zip(names, row)) for row in cursor.fetchall()]

    @staticmethod
    def _seed_legacy_problem_if_missing(cursor, problem_id, row):
        slug = row.get("slug")
        if not slug or not str(slug).strip():
            return
        cursor.execute(
            """
            INSERT OR IGNORE INTO problems (
                id, source, slug, title, title_cn, difficulty, ac_rate, rating,
                contest, problem_index, tags, link, category, paid_only, content,
                content_cn, similar_questions
            ) VALUES (?, 'leetcode', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                str(problem_id),
                str(slug).strip(),
                row.get("title"),
                row.get("title_cn"),
                row.get("difficulty"),
                row.get("ac_rate"),
                row.get("rating"),
                row.get("contest"),
                row.get("problem_index"),
                row.get("tags") or "[]",
                row.get("link"),
                row.get("category"),
                row.get("paid_only"),
                row.get("content"),
                row.get("content_cn"),
                row.get("similar_questions") or "[]",
            ),
        )

    @staticmethod
    def _legacy_problem_id(cursor, problem_id, slug):
        if problem_id is not None and str(problem_id).strip():
            return str(problem_id).strip()
        if not slug or not str(slug).strip():
            return None
        cursor.execute(
            "SELECT id FROM problems WHERE source = 'leetcode' AND slug = ? LIMIT 1",
            (str(slug).strip(),),
        )
        row = cursor.fetchone()
        return str(row[0]).strip() if row and str(row[0]).strip() else None

    @staticmethod
    def _daily_source_from_domain(value):
        if value == "com":
            return "leetcode.com"
        if value == "cn":
            return "leetcode.cn"
        return value

    @classmethod
    def _normalize_problem_refs(cls, daily):
        raw_refs = daily.get("problems")
        if isinstance(raw_refs, str):
            try:
                raw_refs = json.loads(raw_refs)
            except json.JSONDecodeError:
                raw_refs = [raw_refs]
            if isinstance(raw_refs, str):
                raw_refs = [raw_refs]
        if raw_refs is None:
            problem_id = daily.get("id") or daily.get("qid")
            raw_refs = [f"leetcode:{problem_id}"] if problem_id is not None else []
        elif not isinstance(raw_refs, list):
            raw_refs = [raw_refs]

        refs = []
        for ref in raw_refs:
            if not isinstance(ref, str) or ":" not in ref:
                continue
            source, problem_id = ref.split(":", 1)
            if source.strip() and problem_id.strip():
                refs.append(f"{source.strip()}:{problem_id.strip()}")
        return refs

    def update_daily(self, daily):
        """
        Insert or update daily challenge data
        Args:
            daily (dict): daily challenge data
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        try:
            date = daily.get("date")
            source = daily.get("source") or self._daily_source_from_domain(
                daily.get("domain")
            )
            problem_refs = self._normalize_problem_refs(daily)
            if not date or not source or not problem_refs:
                raise ValueError(
                    "daily challenge requires date, source, and problem refs"
                )
            cursor.execute(
                """
                INSERT INTO daily_challenge (date, source, problems)
                VALUES (?, ?, ?)
                ON CONFLICT(date, source) DO UPDATE SET
                    problems = excluded.problems
                """,
                (date, source, json.dumps(problem_refs)),
            )
            conn.commit()
            logger.info("Inserted/updated daily challenge for %s %s", date, source)
            return True
        except Exception as e:
            logger.error(f"Error inserting/updating daily challenge: {e}")
            return False
        finally:
            conn.close()

    def get_daily_by_date(self, date, domain):
        source = self._daily_source_from_domain(domain)
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute(
            "SELECT date, source, problems FROM daily_challenge WHERE date = ? AND source = ?",
            (date, source),
        )
        row = cursor.fetchone()
        conn.close()
        if row:
            return {
                "date": row[0],
                "source": row[1],
                "domain": domain,
                "problems": json.loads(row[2]),
            }
        return None


if __name__ == "__main__":
    # Example usage
    db_manager = SettingsDatabaseManager()
    db_manager.set_server_settings(
        123456789, 987654321, role_id=111222333, post_time="12:00", timezone="UTC"
    )
    settings = db_manager.get_server_settings(123456789)
    logger.debug(settings)
    db_manager.delete_server_settings(
        123456789
    )  # Delete settings for server ID 123456789
