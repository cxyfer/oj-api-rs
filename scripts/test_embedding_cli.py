import contextlib
import io
import json
import unittest

import embedding_cli
from embeddings.providers import PermanentProviderError


class _SuccessfulRewriter:
    def __init__(self, _config):
        pass

    async def rewrite(self, _text):
        return "rewritten query"


class _SuccessfulGenerator:
    def __init__(self, _config):
        pass

    async def embed(self, _text):
        return [0.1, 0.2]


class EmbedTextTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self._rewriter = embedding_cli.EmbeddingRewriter
        self._generator = embedding_cli.EmbeddingGenerator

    def tearDown(self):
        embedding_cli.EmbeddingRewriter = self._rewriter
        embedding_cli.EmbeddingGenerator = self._generator

    async def test_embed_text_success_shape(self):
        embedding_cli.EmbeddingRewriter = _SuccessfulRewriter
        embedding_cli.EmbeddingGenerator = _SuccessfulGenerator

        result = await embedding_cli._embed_text("binary search", object())

        self.assertEqual(
            result, {"embedding": [0.1, 0.2], "rewritten": "rewritten query"}
        )

    async def test_embed_text_config_failure_reports_config_stage(self):
        class FailingRewriter:
            def __init__(self, _config):
                raise RuntimeError("secret provider url")

        embedding_cli.EmbeddingRewriter = FailingRewriter
        stdout = io.StringIO()

        with (
            contextlib.redirect_stdout(stdout),
            self.assertRaises(SystemExit) as raised,
        ):
            await embedding_cli._embed_text("binary search", object())

        self.assertEqual(raised.exception.code, 1)
        self.assertEqual(
            json.loads(stdout.getvalue()),
            {
                "error": {
                    "stage": "config",
                    "kind": "provider_error",
                    "message": "embedding service configuration failed",
                }
            },
        )

    async def test_embed_text_rewrite_failure_reports_rewrite_stage(self):
        class FailingRewriter(_SuccessfulRewriter):
            async def rewrite(self, _text):
                raise PermanentProviderError("raw provider failure")

        embedding_cli.EmbeddingRewriter = FailingRewriter
        embedding_cli.EmbeddingGenerator = _SuccessfulGenerator
        stdout = io.StringIO()

        with (
            contextlib.redirect_stdout(stdout),
            self.assertRaises(SystemExit) as raised,
        ):
            await embedding_cli._embed_text("binary search", object())

        self.assertEqual(raised.exception.code, 1)
        self.assertEqual(json.loads(stdout.getvalue())["error"]["stage"], "rewrite")
        self.assertEqual(
            json.loads(stdout.getvalue())["error"]["message"],
            "query rewrite service failed",
        )

    async def test_embed_text_embedding_failure_reports_embedding_stage(self):
        class FailingGenerator(_SuccessfulGenerator):
            async def embed(self, _text):
                raise PermanentProviderError("raw embedding failure")

        embedding_cli.EmbeddingRewriter = _SuccessfulRewriter
        embedding_cli.EmbeddingGenerator = FailingGenerator
        stdout = io.StringIO()

        with (
            contextlib.redirect_stdout(stdout),
            self.assertRaises(SystemExit) as raised,
        ):
            await embedding_cli._embed_text("binary search", object())

        self.assertEqual(raised.exception.code, 1)
        self.assertEqual(json.loads(stdout.getvalue())["error"]["stage"], "embedding")
        self.assertEqual(
            json.loads(stdout.getvalue())["error"]["message"],
            "embedding service failed",
        )

    async def test_embed_text_config_failure_reports_config_stage_from_main(self):
        original_get_config = embedding_cli.get_config
        original_argv = embedding_cli.sys.argv
        embedding_cli.get_config = lambda: (_ for _ in ()).throw(
            RuntimeError("secret config path")
        )
        embedding_cli.sys.argv = ["embedding_cli.py", "--embed-text", "binary search"]
        stdout = io.StringIO()

        try:
            with (
                contextlib.redirect_stdout(stdout),
                self.assertRaises(SystemExit) as raised,
            ):
                await embedding_cli.main()
        finally:
            embedding_cli.get_config = original_get_config
            embedding_cli.sys.argv = original_argv

        self.assertEqual(raised.exception.code, 1)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["error"]["stage"], "config")
        self.assertEqual(payload["error"]["kind"], "configuration_error")
        self.assertEqual(
            payload["error"]["message"], "embedding service configuration failed"
        )


if __name__ == "__main__":
    unittest.main()
