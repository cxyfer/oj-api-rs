"""Problem statement rewriter (delegates to LLM provider)."""

from __future__ import annotations

import asyncio
from concurrent.futures import Executor
from typing import Optional

from embeddings.providers import create_provider
from utils.config import ConfigManager, RewriteModelConfig, get_config
from utils.logger import get_llm_logger

logger = get_llm_logger()

REWRITE_PROMPT = """Role: Competitive Programming Problem Simplifier

Task: Rewrite the given problem statement into its core algorithmic essence. The output must be concise,
self-contained, and immediately understandable without referencing the original.

Instructions:

1.  **Content to REMOVE**:
    *   All storytelling, legends, character names, and background context.
    *   All examples and their explanations.
    *   Redundant phrasing.

2.  **Content to KEEP**:
    *   The core problem definition and objective.

3.  **HTML Processing**:
    *   Extract text content from semantic tags like `<span data-keyword="...">`
        (e.g., `<span data-keyword="binary-array">binary array</span>` → "binary array").
    *   Convert mathematical HTML to MathJax:
        *   `<sup>` → `^{{}}` (e.g., `10<sup>9</sup>` → $10^9$)
        *   `<sub>` → `_{{}}` (e.g., `a<sub>i</sub>` → $a_i$)
        *   `<code>` variables → inline math (e.g., `<code>n</code>` → $n$)

4.  **Language**: If the original is not in English, translate to English.

5.  **Output Format**:
    *   Use MathJax ($...$) for all math.
    *   Be as succinct as possible while remaining unambiguous.
    *   **Output ONLY the simplified statement. No preamble, no labels, no markdown fences.**

Input Statement:
{ORIGINAL}
"""


class EmbeddingRewriter:
    def __init__(self, config: ConfigManager | None = None):
        self.config = config or get_config()
        self.model_config: RewriteModelConfig = self.config.get_rewrite_model_config()
        self._provider = create_provider(self.config, "rewrite")

    def _build_prompt(self, original: str) -> str:
        return REWRITE_PROMPT.format(ORIGINAL=original)

    async def rewrite(self, content: str) -> str:
        return await self.rewrite_with_executor(content, None)

    async def rewrite_with_executor(
        self, content: str, executor: Optional[Executor]
    ) -> str:
        if not content or not content.strip():
            return ""
        prompt = self._build_prompt(content)
        try:
            return await asyncio.wait_for(
                asyncio.get_running_loop().run_in_executor(
                    executor, self._provider.rewrite, prompt
                ),
                timeout=self.model_config.timeout,
            )
        except asyncio.TimeoutError:
            logger.error(
                "Rewrite timed out after %s seconds",
                self.model_config.timeout,
            )
            raise
