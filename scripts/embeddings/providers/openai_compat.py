"""OpenAI-compatible LLM provider using the openai SDK."""

from __future__ import annotations

import logging
from typing import Any, List, Optional, Sequence

from .base import LLMProvider, PermanentProviderError, TransientProviderError

logger = logging.getLogger("llm.openai")


class OpenAICompatProvider(LLMProvider):
    """Provider wrapping the OpenAI SDK, compatible with any OpenAI-compatible endpoint."""

    def __init__(self, config: Any, capability: str) -> None:
        try:
            import openai as _openai
        except ImportError as e:
            raise PermanentProviderError(
                "openai package is required for OpenAI-compatible provider. "
                "Install with: pip install openai"
            ) from e

        self._openai = _openai
        self._capability = capability

        api_key = config.resolve_api_key(capability)
        if not api_key:
            raise ValueError(
                "OpenAI API key not configured. Set [llm].api_key, "
                "[llm.models.<cap>].api_key, or OPENAI_API_KEY env var."
            )

        base_url: Optional[str] = config.resolve_base_url(capability)

        self._client = _openai.OpenAI(api_key=api_key, base_url=base_url)

        if capability == "embedding":
            mc = config.get_embedding_model_config()
            self._model = mc.name
            self._dim = mc.dim
        else:
            mc = config.get_rewrite_model_config()
            self._model = mc.name
            self._temperature = mc.temperature
            self._max_retries = mc.max_retries

        logger.debug(
            "OpenAICompatProvider(%s): model=%s, base_url=%s",
            capability, self._model, base_url,
        )

    def _map_error(self, exc: Exception) -> Exception:
        """Map OpenAI SDK errors to provider-agnostic error types."""
        openai = self._openai
        if isinstance(exc, openai.RateLimitError):
            return TransientProviderError(str(exc))
        if isinstance(exc, openai.APIConnectionError):
            return TransientProviderError(str(exc))
        if isinstance(exc, openai.AuthenticationError):
            return PermanentProviderError(str(exc))
        if isinstance(exc, openai.BadRequestError):
            return PermanentProviderError(str(exc))
        if isinstance(exc, openai.APIStatusError):
            if exc.status_code in {429, 502, 503}:
                return TransientProviderError(str(exc))
            return PermanentProviderError(str(exc))
        return PermanentProviderError(str(exc))

    # --- embed ---

    def embed(self, text: str) -> List[float]:
        vectors = self.embed_batch([text])
        return vectors[0] if vectors else []

    def embed_batch(self, texts: Sequence[str]) -> List[List[float]]:
        if not texts:
            return []
        try:
            response = self._client.embeddings.create(
                input=list(texts),
                model=self._model,
            )
        except Exception as exc:
            raise self._map_error(exc) from exc

        vectors = [item.embedding for item in response.data]
        self._validate_dims(vectors)
        return vectors

    def _validate_dims(self, vectors: List[List[float]]) -> None:
        for v in vectors:
            if len(v) != self._dim:
                raise PermanentProviderError(
                    f"Dimension mismatch: expected dim={self._dim}, got {len(v)}"
                )

    # --- rewrite ---

    def rewrite(self, prompt: str) -> str:
        try:
            response = self._client.chat.completions.create(
                model=self._model,
                messages=[{"role": "user", "content": prompt}],
                temperature=self._temperature,
            )
        except Exception as exc:
            raise self._map_error(exc) from exc

        choice = response.choices[0] if response.choices else None
        if choice and choice.message and choice.message.content:
            return choice.message.content
        return ""
