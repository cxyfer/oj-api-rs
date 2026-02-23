import logging
import os
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Optional

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
        self._apply_env_overrides()

    def _load_config(self) -> None:
        if not self.config_path.exists():
            raise FileNotFoundError(
                f"Configuration file not found: {self.config_path}\n"
                "Please copy config.toml.example to config.toml and update it."
            )
        with open(self.config_path, "rb") as f:
            self._config = tomllib.load(f)
        logger.info(f"Configuration loaded from {self.config_path}")

    def _apply_env_overrides(self) -> None:
        env_mappings = {
            "GEMINI_API_KEY": ("gemini", "api_key"),
        }
        for env_var, path in env_mappings.items():
            val = os.getenv(env_var)
            if val:
                self._set_nested(self._config, path, val)

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

    def get_embedding_model_config(self) -> "EmbeddingModelConfig":
        section = self.get("gemini.models.embedding", {})
        return EmbeddingModelConfig(
            name=section.get("name", "gemini-embedding-001"),
            dim=section.get("dim", 768),
            task_type=section.get("task_type", "SEMANTIC_SIMILARITY"),
            batch_size=section.get("batch_size", 32),
            api_key=section.get("api_key"),
            base_url=section.get("base_url"),
        )

    def get_rewrite_model_config(self) -> "RewriteModelConfig":
        section = self.get("gemini.models.rewrite", {})
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


_config: Optional[ConfigManager] = None


def get_config() -> ConfigManager:
    global _config
    if _config is None:
        _config = ConfigManager()
    return _config
