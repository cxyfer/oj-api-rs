"""Embedding generator for similarity search (delegates to LLM provider)."""

from __future__ import annotations

import asyncio
from typing import List, Sequence

from embeddings.providers import create_provider
from utils.config import ConfigManager, EmbeddingModelConfig, get_config
from utils.logger import get_llm_logger

logger = get_llm_logger()


class EmbeddingGenerator:
    def __init__(self, config: ConfigManager | None = None):
        self.config = config or get_config()
        self.model_config: EmbeddingModelConfig = self.config.get_embedding_model_config()
        self._provider = create_provider(self.config, "embedding")

    async def embed(self, content: str) -> List[float]:
        vectors = await self.embed_batch([content])
        return vectors[0] if vectors else []

    async def embed_batch(self, contents: Sequence[str]) -> List[List[float]]:
        if not contents:
            return []
        return await asyncio.to_thread(self._provider.embed_batch, contents)
