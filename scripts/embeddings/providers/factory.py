"""Factory for creating LLM provider instances from config."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from utils.config import ConfigManager

from .base import LLMProvider

_VALID_PROVIDERS = ("gemini", "openai")


def create_provider(config: ConfigManager, capability: str) -> LLMProvider:
    """Create an LLM provider for the given capability.

    Args:
        config: Application config manager with resolved LLM settings.
        capability: Either "embedding" or "rewrite".

    Returns:
        An LLMProvider instance for the requested capability.

    Raises:
        ValueError: If the provider name is not recognized.
    """
    provider_name = _resolve_provider_name(config, capability)

    if provider_name == "gemini":
        from .gemini import GeminiProvider

        return GeminiProvider(config, capability)

    if provider_name == "openai":
        from .openai_compat import OpenAICompatProvider

        return OpenAICompatProvider(config, capability)

    raise ValueError(
        f"Unknown LLM provider '{provider_name}'. Valid options: {', '.join(_VALID_PROVIDERS)}"
    )


def _resolve_provider_name(config: ConfigManager, capability: str) -> str:
    """Resolve provider name with per-capability override support."""
    cap_provider = config.get(f"llm.models.{capability}.provider")
    if cap_provider:
        return cap_provider

    global_provider = config.get("llm.provider")
    if global_provider:
        return global_provider

    return "gemini"
