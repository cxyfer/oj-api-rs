import logging
import os
import sys
import warnings
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Optional
from urllib.parse import urlparse

if sys.version_info >= (3, 11):
    import tomllib
else:
    try:
        import tomli as tomllib
    except ImportError:
        raise ImportError("tomli is required for Python < 3.11")

logger = logging.getLogger("config")


class ConfigManager:
    def __init__(self, config_path: Optional[str] = None):
        if config_path:
            self.config_path = Path(config_path)
        else:
            env_path = os.getenv("CONFIG_PATH")
            self.config_path = Path(env_path) if env_path else Path("../config.toml")
        self._config: Dict[str, Any] = {}
        self._load_config()

    def _load_config(self) -> None:
        if not self.config_path.exists():
            raise FileNotFoundError(
                f"Configuration file not found: {self.config_path}\n"
                "Please copy config.toml.example to config.toml and update it."
            )
        with open(self.config_path, "rb") as f:
            self._config = tomllib.load(f)
        logger.info(f"Configuration loaded from {self.config_path}")

    def _set_nested(self, d: Dict[str, Any], path: tuple, value: Any) -> None:
        for key in path[:-1]:
            d = d.setdefault(key, {})
        d[path[-1]] = value

    def _get_nested(self, d: Dict[str, Any], path: tuple, default: Any = None) -> Any:
        for key in path:
            if isinstance(d, dict) and key in d:
                d = d[key]
            else:
                return default
        return d

    def get(self, key: str, default: Any = None) -> Any:
        path = tuple(key.split("."))
        return self._get_nested(self._config, path, default)

    def get_section(self, section: str) -> Dict[str, Any]:
        return self._config.get(section, {})

    @property
    def gemini_api_key(self) -> Optional[str]:
        return self.get("gemini.api_key")

    @property
    def gemini_base_url(self) -> Optional[str]:
        return self.get("gemini.base_url")

    @property
    def database_path(self) -> str:
        raw = self.get("database.path", "data/data.db")
        p = Path(raw)
        if not p.is_absolute():
            p = self.config_path.parent / p
        return str(p)

    @property
    def log_level(self) -> str:
        return self.get("logging.level", "INFO")

    @property
    def llm_provider(self) -> str:
        return self.get("llm.provider", "gemini")

    @property
    def _using_legacy_config(self) -> bool:
        return "llm" not in self._config and "gemini" in self._config

    def _get_llm_model_section(self, capability: str) -> Dict[str, Any]:
        """Resolve model config section with [llm] -> [gemini] fallback."""
        if "llm" in self._config:
            return self.get(f"llm.models.{capability}", {})

        if "gemini" in self._config:
            warnings.warn(
                "[gemini] config section is deprecated; migrate to [llm]. "
                "See config.toml.example for the new structure.",
                DeprecationWarning,
                stacklevel=3,
            )
            return self.get(f"gemini.models.{capability}", {})

        raise ValueError(
            "No LLM configuration found. "
            "Please add an [llm] section to config.toml. "
            "See config.toml.example for the structure."
        )

    def _get_llm_global(self, key: str, default: Any = None) -> Any:
        """Get a global LLM setting with [llm] -> [gemini] fallback."""
        if "llm" in self._config:
            return self.get(f"llm.{key}", default)
        return self.get(f"gemini.{key}", default)

    def resolve_api_key(self, capability: str) -> Optional[str]:
        """Resolve API key per-capability with precedence chain.

        [llm.models.<cap>].api_key -> [llm].api_key -> env var.
        """
        section = self._get_llm_model_section(capability)
        cap_key = section.get("api_key")
        if cap_key:
            return cap_key

        global_key = self._get_llm_global("api_key")
        if global_key:
            return global_key

        provider = self.get(f"llm.models.{capability}.provider") or self.llm_provider
        env_names = {
            "openai": ["OPENAI_API_KEY"],
            "gemini": ["GOOGLE_API_KEY", "GEMINI_API_KEY", "GOOGLE_GEMINI_API_KEY"],
        }
        for env_name in env_names.get(provider, []):
            val = os.getenv(env_name)
            if val:
                return val

        return None

    def resolve_base_url(self, capability: str) -> Optional[str]:
        """Resolve base_url per-capability with inheritance."""
        section = self._get_llm_model_section(capability)
        cap_url = section.get("base_url")
        if cap_url:
            return cap_url
        return self._get_llm_global("base_url")

    def get_embedding_model_config(self) -> "EmbeddingModelConfig":
        section = self._get_llm_model_section("embedding")
        return EmbeddingModelConfig(
            name=section.get("name", "gemini-embedding-001"),
            dim=section.get("dim", 768),
            task_type=section.get("task_type", "SEMANTIC_SIMILARITY"),
            batch_size=section.get("batch_size", 32),
            api_key=section.get("api_key"),
            base_url=section.get("base_url"),
        )

    def get_rewrite_model_config(self) -> "RewriteModelConfig":
        section = self._get_llm_model_section("rewrite")
        return RewriteModelConfig(
            name=section.get("name", "gemini-2.0-flash"),
            temperature=section.get("temperature", 0.3),
            timeout=section.get("timeout", 30),
            max_retries=section.get("max_retries", 2),
            workers=section.get("workers", 4),
            api_key=section.get("api_key"),
            base_url=section.get("base_url"),
        )

    def get_similar_config(self) -> "SimilarConfig":
        section = self.get("similar", {})
        return SimilarConfig(
            top_k=section.get("top_k", 5),
            min_similarity=section.get("min_similarity", 0.70),
        )

    def get_crawler_config(self, crawler_name: str) -> "CrawlerHttpConfig":
        _FIELDS = ("user_agent", "proxy", "http_proxy", "https_proxy", "socks5_proxy")
        global_section = self._config.get("crawler", {})
        per_crawler = global_section.get(crawler_name, {}) if isinstance(global_section, dict) else {}

        def _norm(val: Any) -> Optional[str]:
            if val is None:
                return None
            s = str(val).strip()
            return s if s else None

        merged = {}
        for field in _FIELDS:
            value = _norm(per_crawler.get(field)) if isinstance(per_crawler, dict) else None
            if value is None:
                value = _norm(global_section.get(field))
            merged[field] = value

        proxy_fields = ("proxy", "http_proxy", "https_proxy", "socks5_proxy")
        for field in proxy_fields:
            if merged[field] is not None:
                _validate_proxy_url(merged[field])

        return CrawlerHttpConfig(**merged)


@dataclass
class EmbeddingModelConfig:
    name: str = "gemini-embedding-001"
    dim: int = 768
    task_type: str = "SEMANTIC_SIMILARITY"
    batch_size: int = 32
    api_key: Optional[str] = None
    base_url: Optional[str] = None


@dataclass
class RewriteModelConfig:
    name: str = "gemini-2.0-flash"
    temperature: float = 0.3
    timeout: int = 30
    max_retries: int = 2
    workers: int = 4
    api_key: Optional[str] = None
    base_url: Optional[str] = None


@dataclass
class SimilarConfig:
    top_k: int = 5
    min_similarity: float = 0.70


_VALID_PROXY_SCHEMES = {"http", "https", "socks5", "socks5h"}


def _validate_proxy_url(url: str) -> None:
    parsed = urlparse(url)
    if parsed.scheme not in _VALID_PROXY_SCHEMES:
        raise ValueError(
            f"Invalid proxy scheme '{parsed.scheme}' in '{url}'. "
            f"Must be one of: {', '.join(sorted(_VALID_PROXY_SCHEMES))}"
        )
    if not parsed.hostname:
        raise ValueError(f"Proxy URL missing host: '{url}'")


@dataclass(frozen=True)
class CrawlerHttpConfig:
    user_agent: Optional[str] = None
    proxy: Optional[str] = None
    http_proxy: Optional[str] = None
    https_proxy: Optional[str] = None
    socks5_proxy: Optional[str] = None

    def resolve_proxy(self, scheme: str = "https") -> Optional[str]:
        if scheme not in ("http", "https"):
            raise ValueError(f"Invalid scheme '{scheme}', must be 'http' or 'https'")
        specific = self.http_proxy if scheme == "http" else self.https_proxy
        return specific or self.socks5_proxy or self.proxy or None


_config: Optional[ConfigManager] = None


def get_config() -> ConfigManager:
    global _config
    if _config is None:
        _config = ConfigManager()
    return _config
