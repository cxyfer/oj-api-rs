"""Abstract LLM provider interface and provider-agnostic error types."""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import List, Sequence


class TransientProviderError(Exception):
    """Retryable provider error (rate limits, temporary unavailability)."""


class PermanentProviderError(Exception):
    """Non-retryable provider error (auth failure, invalid model, dimension mismatch)."""


class LLMProvider(ABC):
    """Abstract base class for LLM providers.

    Implementations must provide embed, embed_batch, and rewrite methods.
    """

    @abstractmethod
    def embed(self, text: str) -> List[float]:
        ...

    @abstractmethod
    def embed_batch(self, texts: Sequence[str]) -> List[List[float]]:
        ...

    @abstractmethod
    def rewrite(self, prompt: str) -> str:
        ...
