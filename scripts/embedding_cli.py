"""CLI tool for building and querying problem embeddings."""

from __future__ import annotations

import argparse
import asyncio
import json
import math
import os
import sys
import tempfile
import time
import uuid
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple

from embeddings import (
    EmbeddingGenerator,
    EmbeddingRewriter,
    EmbeddingStorage,
    SimilaritySearcher,
)
from embeddings.providers import PermanentProviderError, TransientProviderError
from leetcode import html_to_text
from utils.config import get_config
from utils.database import EmbeddingDatabaseManager
from utils.logger import get_core_logger

logger = get_core_logger()

LOGS_DIR = os.path.join(os.path.dirname(__file__), "logs")


@dataclass
class BuildReport:
    total_pending: int = 0
    succeeded: int = 0
    skipped: Dict[str, int] = field(default_factory=dict)
    failed: Dict[str, int] = field(default_factory=dict)
    duration_secs: float = 0.0
    _skipped_ids: Dict[str, List[str]] = field(default_factory=dict)
    _failed_ids: Dict[str, List[str]] = field(default_factory=dict)

    def add_skipped(self, reason: str, problem_id: str) -> None:
        self.skipped[reason] = self.skipped.get(reason, 0) + 1
        self._skipped_ids.setdefault(reason, []).append(problem_id)

    def add_failed(self, reason: str, problem_id: str) -> None:
        self.failed[reason] = self.failed.get(reason, 0) + 1
        self._failed_ids.setdefault(reason, []).append(problem_id)

    def add_succeeded(self) -> None:
        self.succeeded += 1

    def to_dict(self) -> dict:
        return {
            "total_pending": self.total_pending,
            "succeeded": self.succeeded,
            "skipped": dict(self.skipped),
            "failed": dict(self.failed),
            "duration_secs": round(self.duration_secs, 1),
        }

    @property
    def total_failed(self) -> int:
        return sum(self.failed.values())


def _write_progress(job_id: str, data: dict) -> None:
    """Atomic write of progress file via temp + rename."""
    os.makedirs(LOGS_DIR, exist_ok=True)
    path = os.path.join(LOGS_DIR, f"{job_id}.progress.json")
    fd, tmp = tempfile.mkstemp(dir=LOGS_DIR, suffix=".tmp")
    try:
        with os.fdopen(fd, "w") as f:
            json.dump(data, f)
        os.replace(tmp, path)
    except Exception:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def _fetch_problems_with_content_sync(
    db: EmbeddingDatabaseManager,
    source: str,
    filter_pattern: str | None = None,
) -> List[Tuple[str, str]]:
    conditions = ["source = ?", "content IS NOT NULL", "content != ''"]
    params: list = [source]
    if filter_pattern:
        conditions.append("id LIKE '%' || ? || '%'")
        params.append(filter_pattern)
    where_clause = " AND ".join(conditions)
    rows = db.execute(
        f"""
        SELECT id, content
        FROM problems
        WHERE {where_clause}
        ORDER BY id ASC
        """,
        tuple(params),
        fetchall=True,
    )
    return [(str(row[0]), row[1]) for row in rows] if rows else []


def _count_problems_with_content_sync(
    db: EmbeddingDatabaseManager, source: str, filter_pattern: str | None = None
) -> int:
    conditions = ["source = ?", "content IS NOT NULL", "content != ''"]
    params: list = [source]
    if filter_pattern:
        conditions.append("id LIKE '%' || ? || '%'")
        params.append(filter_pattern)
    where_clause = " AND ".join(conditions)
    row = db.execute(
        f"""
        SELECT COUNT(*)
        FROM problems
        WHERE {where_clause}
        """,
        tuple(params),
        fetchone=True,
    )
    return int(row[0]) if row else 0


def _fetch_problem_ids_with_content_sync(
    db: EmbeddingDatabaseManager, source: str, filter_pattern: str | None = None
) -> List[str]:
    conditions = ["source = ?", "content IS NOT NULL", "content != ''"]
    params: list = [source]
    if filter_pattern:
        conditions.append("id LIKE '%' || ? || '%'")
        params.append(filter_pattern)
    where_clause = " AND ".join(conditions)
    rows = db.execute(
        f"SELECT id FROM problems WHERE {where_clause} ORDER BY id ASC",
        tuple(params),
        fetchall=True,
    )
    return [str(row[0]) for row in rows] if rows else []


def _fetch_sources_with_content_sync(db: EmbeddingDatabaseManager) -> List[str]:
    rows = db.execute(
        """
        SELECT DISTINCT source
        FROM problems
        WHERE content IS NOT NULL AND content != ''
        ORDER BY source ASC
        """,
        fetchall=True,
    )
    return [row[0] for row in rows] if rows else []


async def _prepare_db(db: EmbeddingDatabaseManager, dim: int, rebuild: bool) -> None:
    if rebuild:
        db.execute("DROP TABLE IF EXISTS vec_embeddings", commit=True)
    db.create_vec_table(dim)


async def build_embeddings(
    db: EmbeddingDatabaseManager,
    storage: EmbeddingStorage,
    rewriter: EmbeddingRewriter | None,
    generator: EmbeddingGenerator | None,
    source: str,
    batch_size: int,
    rebuild: bool,
    dry_run: bool,
    filter_pattern: str | None = None,
    job_id: str | None = None,
) -> BuildReport:
    config = get_config()
    embedding_config = config.get_embedding_model_config()
    report = BuildReport()
    start_time = time.monotonic()
    wall_start = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

    await _prepare_db(db, embedding_config.dim, rebuild)

    if rebuild:
        await storage.delete_all_embeddings(source)

    if not rebuild and not db.check_dimension_consistency(embedding_config.dim):
        raise ValueError(
            "Embedding dimension mismatch. Please run with --rebuild to reset the index."
        )

    total_problems = await asyncio.to_thread(
        _count_problems_with_content_sync, db, source, filter_pattern
    )
    existing_metadata = await storage.get_existing_ids(
        source, embedding_config.name, embedding_config.dim
    )
    existing_vectors = await storage.get_existing_vector_ids(source)
    existing_ids = existing_metadata.intersection(existing_vectors)

    if filter_pattern:
        filtered_ids = set(
            await asyncio.to_thread(
                _fetch_problem_ids_with_content_sync, db, source, filter_pattern
            )
        )
        pending_count = len(filtered_ids - existing_ids)
    else:
        pending_count = max(total_problems - len(existing_ids), 0)

    logger.info("Total problems with content: %s", total_problems)
    logger.info("Existing embeddings: %s", len(existing_ids))
    logger.info("Pending embeddings: %s", pending_count)

    if dry_run:
        batch_calls = math.ceil(pending_count / batch_size) if batch_size else 0
        logger.info("Estimated embedding API calls: %s", batch_calls)
        logger.info("Estimated rewrite API calls: %s", pending_count)
        report.total_pending = pending_count
        report.duration_secs = time.monotonic() - start_time
        return report

    if rewriter is None or generator is None:
        raise ValueError("Embedding generator not initialized")

    problems = await asyncio.to_thread(
        _fetch_problems_with_content_sync, db, source, filter_pattern
    )
    pending = [(pid, content) for pid, content in problems if pid not in existing_ids]

    if not pending:
        logger.info("No pending embeddings to process.")
        report.duration_secs = time.monotonic() - start_time
        return report

    total_pending = len(pending)
    report.total_pending = total_pending
    effective_batch_size = max(1, batch_size or 1)
    rewrite_workers = max(
        1, min(getattr(rewriter.model_config, "workers", 1), total_pending)
    )
    logger.info(
        "Starting rewrite pipeline: %s problems, workers=%s, batch_size=%s",
        total_pending,
        rewrite_workers,
        effective_batch_size,
    )
    executor = ThreadPoolExecutor(max_workers=rewrite_workers)

    try:
        rewrite_queue: asyncio.Queue[Tuple[str, str] | None] = asyncio.Queue()
        embed_queue: asyncio.Queue[Tuple[str, str] | None] = asyncio.Queue()
        for item in pending:
            rewrite_queue.put_nowait(item)
        for _ in range(rewrite_workers):
            rewrite_queue.put_nowait(None)

        progress_lock = asyncio.Lock()
        rewrite_done = 0
        rewrite_skipped = 0
        embed_done = 0

        def _update_progress(phase: str) -> None:
            if not job_id:
                return
            try:
                _write_progress(
                    job_id,
                    {
                        "phase": phase,
                        "rewrite_progress": {
                            "done": rewrite_done,
                            "total": total_pending,
                            "skipped": rewrite_skipped,
                        },
                        "embed_progress": {
                            "done": embed_done,
                            "total": total_pending - rewrite_skipped,
                        },
                        "started_at": wall_start,
                    },
                )
            except Exception:
                pass

        async def rewrite_worker(worker_id: int) -> None:
            nonlocal rewrite_done, rewrite_skipped
            while True:
                item = await rewrite_queue.get()
                if item is None:
                    rewrite_queue.task_done()
                    break
                problem_id, content = item
                text = html_to_text(content) if content else ""
                if not text.strip():
                    logger.warning("Problem %s skipped: empty_content", problem_id)
                    async with progress_lock:
                        rewrite_skipped += 1
                        rewrite_done += 1
                        report.add_skipped("empty_content", problem_id)
                        _update_progress("rewriting")
                    rewrite_queue.task_done()
                    continue
                try:
                    rewritten = await rewriter.rewrite_with_executor(text, executor)
                except asyncio.TimeoutError:
                    logger.error(
                        "Problem %s: rewrite_timeout after %ss",
                        problem_id,
                        rewriter.model_config.timeout,
                    )
                    async with progress_lock:
                        rewrite_skipped += 1
                        rewrite_done += 1
                        report.add_skipped("rewrite_timeout", problem_id)
                        _update_progress("rewriting")
                    rewrite_queue.task_done()
                    continue
                except Exception as exc:
                    logger.error("Problem %s: rewrite_error: %s", problem_id, exc)
                    async with progress_lock:
                        rewrite_skipped += 1
                        rewrite_done += 1
                        report.add_skipped("rewrite_error", problem_id)
                        _update_progress("rewriting")
                    rewrite_queue.task_done()
                    continue
                if not rewritten or not rewritten.strip():
                    logger.warning("Problem %s: rewrite_empty", problem_id)
                    async with progress_lock:
                        rewrite_skipped += 1
                        rewrite_done += 1
                        report.add_skipped("rewrite_empty", problem_id)
                        _update_progress("rewriting")
                    rewrite_queue.task_done()
                    continue
                await embed_queue.put((problem_id, rewritten))
                async with progress_lock:
                    rewrite_done += 1
                    if rewrite_done % 50 == 0 or rewrite_done == total_pending:
                        logger.info(
                            "Rewrite progress %s/%s (skipped %s)",
                            rewrite_done,
                            total_pending,
                            rewrite_skipped,
                        )
                    _update_progress("rewriting")
                rewrite_queue.task_done()

        async def embed_worker() -> None:
            nonlocal embed_done
            buffer: List[Tuple[str, str]] = []
            while True:
                item = await embed_queue.get()
                if item is None:
                    embed_queue.task_done()
                    break
                buffer.append(item)
                if len(buffer) >= effective_batch_size:
                    await _flush_with_bisect(
                        buffer,
                        storage,
                        generator,
                        embedding_config,
                        source,
                        report,
                        progress_lock,
                    )
                    async with progress_lock:
                        embed_done += len(buffer)
                        _update_progress("embedding")
                    buffer.clear()
                embed_queue.task_done()
            if buffer:
                await _flush_with_bisect(
                    buffer,
                    storage,
                    generator,
                    embedding_config,
                    source,
                    report,
                    progress_lock,
                )
                async with progress_lock:
                    embed_done += len(buffer)
                    _update_progress("embedding")
            logger.info("Embedding pipeline complete (%s succeeded)", report.succeeded)

        rewrite_tasks = [
            asyncio.create_task(rewrite_worker(i)) for i in range(rewrite_workers)
        ]
        embed_task = asyncio.create_task(embed_worker())

        await rewrite_queue.join()
        await embed_queue.put(None)
        await embed_queue.join()
        await asyncio.gather(*rewrite_tasks)
        await embed_task
    finally:
        executor.shutdown(wait=True)
        report.duration_secs = time.monotonic() - start_time

    return report


async def _flush_with_bisect(
    batch: List[Tuple[str, str]],
    storage: EmbeddingStorage,
    generator: EmbeddingGenerator,
    embedding_config,
    source: str,
    report: BuildReport,
    progress_lock: asyncio.Lock,
    max_retries: int = 3,
) -> None:
    """Retry-then-bisect flush strategy."""
    rewritten_texts = [item[1] for item in batch]
    problem_ids = [item[0] for item in batch]

    for attempt in range(max_retries):
        try:
            embeddings = await generator.embed_batch(rewritten_texts)
            if len(embeddings) != len(problem_ids):
                raise PermanentProviderError(
                    f"Batch size mismatch: expected {len(problem_ids)} got {len(embeddings)}"
                )
            for pid, rewritten, emb in zip(problem_ids, rewritten_texts, embeddings):
                await storage.save_embedding(
                    source,
                    pid,
                    rewritten,
                    embedding_config.name,
                    embedding_config.dim,
                    emb,
                )
                async with progress_lock:
                    report.add_succeeded()
            return
        except TransientProviderError as exc:
            if attempt < max_retries - 1:
                wait = min(2 ** (attempt + 1), 60)
                logger.warning("Transient error, retrying in %ss: %s", wait, exc)
                await asyncio.sleep(wait)
                continue
            break
        except PermanentProviderError:
            break
        except Exception as exc:
            if attempt < max_retries - 1:
                wait = min(2 ** (attempt + 1), 60)
                logger.warning("Embed error, retrying in %ss: %s", wait, exc)
                await asyncio.sleep(wait)
                continue
            break

    if len(batch) == 1:
        pid = problem_ids[0]
        logger.error("Problem %s: embed_permanent failure", pid)
        async with progress_lock:
            report.add_failed("embed_permanent", pid)
        return

    mid = len(batch) // 2
    await _flush_with_bisect(
        batch[:mid],
        storage,
        generator,
        embedding_config,
        source,
        report,
        progress_lock,
        max_retries,
    )
    await _flush_with_bisect(
        batch[mid:],
        storage,
        generator,
        embedding_config,
        source,
        report,
        progress_lock,
        max_retries,
    )


async def query_similar(
    db: EmbeddingDatabaseManager,
    storage: EmbeddingStorage,
    rewriter: EmbeddingRewriter,
    generator: EmbeddingGenerator,
    source: Optional[str],
    query: str,
    top_k: int,
    min_similarity: float,
) -> None:
    if not query.strip():
        print("Please provide a problem description or keywords.")
        return

    config = get_config()
    embedding_config = config.get_embedding_model_config()

    await _prepare_db(db, embedding_config.dim, rebuild=False)

    if not db.check_dimension_consistency(embedding_config.dim):
        raise ValueError(
            "Embedding dimension mismatch. Please run with --rebuild to reset the index."
        )

    total_vectors = await storage.count_embeddings(source)
    if total_vectors == 0:
        print("Embedding index is empty. Run embedding_cli.py --build first.")
        return

    rewritten = await rewriter.rewrite(query)
    embedding = await generator.embed(rewritten)
    searcher = SimilaritySearcher(db, storage)
    results = await searcher.search(embedding, source, top_k, min_similarity)

    if not results:
        print("No similar problems found. Try a more detailed description.")
        return

    print("Top similar problems:")
    for idx, result in enumerate(results, start=1):
        title = result.get("title") or result.get("problem_id")
        difficulty = result.get("difficulty") or "N/A"
        similarity = result.get("similarity")
        link = result.get("link") or ""
        print(f"{idx}. {title} ({difficulty}) similarity={similarity:.2f}")
        if link:
            print(f"   {link}")


async def show_stats(
    db: EmbeddingDatabaseManager,
    storage: EmbeddingStorage,
    source: str,
    filter_pattern: str | None = None,
) -> None:
    config = get_config()
    embedding_config = config.get_embedding_model_config()

    await _prepare_db(db, embedding_config.dim, rebuild=False)

    total_problems = await asyncio.to_thread(
        _count_problems_with_content_sync, db, source, filter_pattern
    )
    total_vectors = await storage.count_embeddings(source, filter_pattern)
    total_metadata = await storage.count_metadata(source, filter_pattern)

    pending = max(total_problems - total_metadata, 0)
    print("Embedding stats:")
    print(f"  Total problems (with content): {total_problems}")
    print(f"  Metadata rows: {total_metadata}")
    print(f"  Vector rows: {total_vectors}")
    print(f"  Pending: {pending}")


async def main() -> None:
    parser = argparse.ArgumentParser(description="Embedding CLI tool")
    parser.add_argument("--build", action="store_true", help="Build embeddings")
    parser.add_argument("--rebuild", action="store_true", help="Rebuild embeddings")
    parser.add_argument("--query", type=str, help="Query similar problems")
    parser.add_argument("--stats", action="store_true", help="Show embedding stats")
    parser.add_argument(
        "--dry-run", action="store_true", help="Estimate embedding cost"
    )
    parser.add_argument(
        "--embed-text", type=str, help="Generate embedding for given text", default=None
    )
    parser.add_argument("--source", type=str, help="Problem source", default="all")
    parser.add_argument("--top-k", type=int, help="Top-k results", default=None)
    parser.add_argument(
        "--min-similarity",
        type=float,
        help="Minimum similarity threshold",
        default=None,
    )
    parser.add_argument(
        "--batch-size", type=int, help="Embedding batch size", default=None
    )
    parser.add_argument(
        "--filter", type=str, help="Filter problems by ID substring", default=None
    )
    parser.add_argument(
        "--job-id", type=str, help="Job ID for progress tracking", default=None
    )

    args = parser.parse_args()
    config = get_config()
    similar_config = config.get_similar_config()
    embedding_config = config.get_embedding_model_config()

    source = args.source.strip().lower()
    top_k = args.top_k or similar_config.top_k
    min_similarity = (
        args.min_similarity
        if args.min_similarity is not None
        else similar_config.min_similarity
    )
    batch_size = args.batch_size or embedding_config.batch_size
    filter_pattern = args.filter
    job_id = args.job_id or str(uuid.uuid4())

    if not (args.build or args.rebuild or args.query or args.stats or args.embed_text):
        parser.print_help()
        return

    if args.embed_text:
        import json as _json

        rewriter = EmbeddingRewriter(config)
        generator = EmbeddingGenerator(config)
        rewritten = await rewriter.rewrite(args.embed_text)
        embedding = await generator.embed(rewritten)
        print(_json.dumps({"embedding": embedding, "rewritten": rewritten}))
        return

    db = EmbeddingDatabaseManager(db_path=config.database_path)
    storage = EmbeddingStorage(db)
    rewriter = None
    generator = None
    needs_llm = args.query or ((args.build or args.rebuild) and not args.dry_run)
    if needs_llm:
        rewriter = EmbeddingRewriter(config)
        generator = EmbeddingGenerator(config)

    sources: List[str] = []
    if source == "all":
        sources = await asyncio.to_thread(_fetch_sources_with_content_sync, db)

    if args.stats:
        if source == "all":
            if not sources:
                print("No problems with content found.")
            for src in sources:
                print(f"Source: {src}")
                await show_stats(db, storage, src, filter_pattern)
        else:
            await show_stats(db, storage, source, filter_pattern)

    if args.query:
        query_source = None if source == "all" else source
        await query_similar(
            db,
            storage,
            rewriter,
            generator,
            query_source,
            args.query,
            top_k,
            min_similarity,
        )

    if args.build or args.rebuild:
        combined_report = BuildReport()
        start_time = time.monotonic()
        try:
            if source == "all":
                if not sources:
                    print("No problems with content found.")
                    return
                if args.rebuild:
                    await _prepare_db(db, embedding_config.dim, rebuild=True)
                    await storage.delete_all_embeddings(None)
                for index, src in enumerate(sources, start=1):
                    logger.info(
                        "Building embeddings for source '%s' (%d/%d)",
                        src,
                        index,
                        len(sources),
                    )
                    try:
                        r = await build_embeddings(
                            db,
                            storage,
                            rewriter,
                            generator,
                            src,
                            batch_size,
                            rebuild=False,
                            dry_run=args.dry_run,
                            filter_pattern=filter_pattern,
                            job_id=job_id,
                        )
                        combined_report.total_pending += r.total_pending
                        combined_report.succeeded += r.succeeded
                        for k, v in r.skipped.items():
                            combined_report.skipped[k] = (
                                combined_report.skipped.get(k, 0) + v
                            )
                        for k, v in r.failed.items():
                            combined_report.failed[k] = (
                                combined_report.failed.get(k, 0) + v
                            )
                    except Exception as exc:
                        logger.error(
                            "Failed to build embeddings for source '%s': %s",
                            src,
                            exc,
                            exc_info=True,
                        )
                        combined_report.add_failed(f"source_fatal:{src}", src)
            else:
                combined_report = await build_embeddings(
                    db,
                    storage,
                    rewriter,
                    generator,
                    source,
                    batch_size,
                    args.rebuild,
                    args.dry_run,
                    filter_pattern,
                    job_id,
                )
        finally:
            combined_report.duration_secs = time.monotonic() - start_time
            print(f"EMBEDDING_SUMMARY:{json.dumps(combined_report.to_dict())}")
            if job_id:
                phase = "failed" if combined_report.total_failed > 0 else "completed"
                try:
                    _write_progress(job_id, {"phase": phase})
                except Exception:
                    pass

        if combined_report.total_failed > 0:
            sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
