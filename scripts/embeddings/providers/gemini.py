"""Gemini LLM provider using google-genai SDK."""

from __future__ import annotations

import logging
from typing import Any, List, Sequence

from .base import LLMProvider, PermanentProviderError, TransientProviderError

logger = logging.getLogger("llm.gemini")


def _is_retryable(exc: Exception) -> bool:
    try:
        from google.genai import errors
    except ImportError:
        return False
    if isinstance(exc, errors.APIError):
        return exc.code in {429, 503}
    return False


class GeminiProvider(LLMProvider):
    """Provider wrapping the google-genai SDK for Gemini models."""

    def __init__(self, config: Any, capability: str) -> None:
        try:
            from google import genai
            from google.genai import types
        except ImportError as e:
            raise PermanentProviderError(
                "google-genai package is required for Gemini provider. "
                "Install with: pip install google-genai"
            ) from e

        self._types = types
        self._capability = capability

        api_key = config.resolve_api_key(capability)
        if not api_key:
            raise ValueError(
                "Gemini API key not configured. Set [llm].api_key, "
                "[llm.models.<cap>].api_key, or GEMINI_API_KEY env var."
            )

        base_url = config.resolve_base_url(capability)
        http_options = types.HttpOptions(base_url=base_url) if base_url else None
        self._client = genai.Client(api_key=api_key, http_options=http_options)

        if capability == "embedding":
            mc = config.get_embedding_model_config()
            self._model = mc.name
            self._dim = mc.dim
            self._embed_config = self._build_embed_config(mc)
        else:
            mc = config.get_rewrite_model_config()
            self._model = mc.name
            self._temperature = mc.temperature
            self._timeout = mc.timeout
            self._max_retries = mc.max_retries

        logger.debug(
            "GeminiProvider(%s): model=%s, base_url=%s",
            capability, self._model, base_url,
        )

    def _build_embed_config(self, mc: Any) -> Any:
        try:
            return self._types.EmbedContentConfig(
                task_type=mc.task_type,
                output_dimensionality=mc.dim,
            )
        except Exception:
            return {
                "task_type": mc.task_type,
                "output_dimensionality": mc.dim,
            }

    def _build_generation_config(self) -> Any:
        try:
            return self._types.GenerateContentConfig(temperature=self._temperature)
        except Exception:
            return {"temperature": self._temperature}

    # --- embed ---

    def embed(self, text: str) -> List[float]:
        vectors = self.embed_batch([text])
        return vectors[0] if vectors else []

    def embed_batch(self, texts: Sequence[str]) -> List[List[float]]:
        if not texts:
            return []
        result = self._embed_with_retry(list(texts))
        vectors = self._extract_vectors(result)
        self._validate_dims(vectors)
        return vectors

    def _embed_with_retry(self, contents: list[str]) -> Any:
        from tenacity import (
            Retrying,
            retry_if_exception,
            stop_after_attempt,
            wait_exponential,
        )

        retryer = Retrying(
            stop=stop_after_attempt(5),
            wait=wait_exponential(multiplier=1, min=2, max=60),
            retry=retry_if_exception(_is_retryable),
            reraise=True,
        )
        try:
            for attempt in retryer:
                with attempt:
                    return self._client.models.embed_content(
                        model=self._model,
                        contents=contents,
                        config=self._embed_config,
                    )
        except Exception as exc:
            if _is_retryable(exc):
                raise TransientProviderError(str(exc)) from exc
            raise PermanentProviderError(str(exc)) from exc

    @staticmethod
    def _extract_vectors(result: Any) -> List[List[float]]:
        raw = getattr(result, "embeddings", None)
        if raw is None and isinstance(result, dict):
            raw = result.get("embeddings")
        if raw is None:
            return []
        vectors: List[List[float]] = []
        for emb in raw:
            if hasattr(emb, "values"):
                vals = emb.values
            elif isinstance(emb, dict) and "values" in emb:
                vals = emb["values"]
            elif hasattr(emb, "embedding"):
                vals = emb.embedding
            else:
                vals = emb
            vectors.append(list(vals))
        return vectors

    def _validate_dims(self, vectors: List[List[float]]) -> None:
        for v in vectors:
            if len(v) != self._dim:
                raise PermanentProviderError(
                    f"Dimension mismatch: expected dim={self._dim}, got {len(v)}"
                )

    # --- rewrite ---

    def rewrite(self, prompt: str) -> str:
        from tenacity import (
            Retrying,
            retry_if_exception,
            stop_after_attempt,
            wait_exponential,
        )

        retryer = Retrying(
            stop=stop_after_attempt(self._max_retries),
            wait=wait_exponential(multiplier=1, min=2, max=30),
            retry=retry_if_exception(_is_retryable),
            reraise=True,
        )
        try:
            for attempt in retryer:
                with attempt:
                    try:
                        response = self._client.models.generate_content(
                            model=self._model,
                            contents=prompt,
                            config=self._build_generation_config(),
                            timeout=self._timeout,
                        )
                    except TypeError:
                        response = self._client.models.generate_content(
                            model=self._model,
                            contents=prompt,
                            config=self._build_generation_config(),
                        )
                    return self._extract_text(response)
        except Exception as exc:
            if _is_retryable(exc):
                raise TransientProviderError(str(exc)) from exc
            raise PermanentProviderError(str(exc)) from exc
        return ""

    @staticmethod
    def _extract_text(response: Any) -> str:
        if hasattr(response, "text"):
            return response.text or ""
        if isinstance(response, dict) and "text" in response:
            return response.get("text", "") or ""
        if hasattr(response, "candidates") and response.candidates:
            candidate = response.candidates[0]
            content = getattr(candidate, "content", None)
            if isinstance(content, str):
                return content
        return str(response) if response is not None else ""
