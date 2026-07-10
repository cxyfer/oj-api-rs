use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub listen_addr: SocketAddr,
    pub admin_secret: String,
    pub graceful_shutdown_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:7856".parse().unwrap(),
            admin_secret: String::new(),
            graceful_shutdown_secs: 10,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub path: String,
    pub pool_max_size: u32,
    pub busy_timeout_ms: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "data/data.db".into(),
            pool_max_size: 8,
            busy_timeout_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CrawlerConfig {
    pub timeout_secs: u64,
    #[serde(default)]
    pub per_source_timeout: HashMap<String, u64>,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 300,
            per_source_timeout: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EmbeddingConfig {
    pub timeout_secs: u64,
    pub batch_timeout_secs: u64,
    pub over_fetch_factor: u32,
    pub concurrency: u32,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            batch_timeout_secs: 600,
            over_fetch_factor: 4,
            concurrency: 4,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub rust_log: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            rust_log: "info".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct McpConfig {
    pub allowed_hosts: Vec<String>,
}

pub const DEFAULT_TENCENT_DOCS_TOKEN_ENV: &str = "TENCENT_DOCS_TOKEN";

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct TencentDocsDailySourceConfig {
    pub token: String,
    pub token_env: String,
}

impl fmt::Debug for TencentDocsDailySourceConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TencentDocsDailySourceConfig")
            .field("token", &"[REDACTED]")
            .field("token_env", &self.token_env)
            .finish()
    }
}

impl TencentDocsDailySourceConfig {
    pub fn resolve_token(&self) -> Option<String> {
        let token = self.token.trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }

        let token_env = self.token_env.trim();
        if token_env.is_empty() {
            return None;
        }

        std::env::var(token_env)
            .ok()
            .and_then(|token| (!token.trim().is_empty()).then(|| token.trim().to_string()))
    }
}

impl Default for TencentDocsDailySourceConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            token_env: DEFAULT_TENCENT_DOCS_TOKEN_ENV.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct DailySourcesConfig {
    pub tencent_docs: TencentDocsDailySourceConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub crawler: CrawlerConfig,
    pub embedding: EmbeddingConfig,
    pub logging: LoggingConfig,
    pub mcp: McpConfig,
    pub daily_sources: DailySourcesConfig,
    #[serde(skip)]
    pub config_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            crawler: CrawlerConfig::default(),
            embedding: EmbeddingConfig::default(),
            logging: LoggingConfig::default(),
            mcp: McpConfig::default(),
            daily_sources: DailySourcesConfig::default(),
            config_path: PathBuf::from("config.toml"),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = std::env::var("CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("config.toml"));

        let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!(
                "FATAL: failed to read configuration file '{}': {}",
                path.display(),
                e
            );
            std::process::exit(1);
        });

        let mut config: Config = toml::from_str(&content).unwrap_or_else(|_| {
            eprintln!(
                "FATAL: failed to parse configuration file '{}'",
                path.display()
            );
            std::process::exit(1);
        });

        let config_dir = path.parent().unwrap_or(Path::new("."));

        // Resolve database.path relative to config file directory
        let db_path = Path::new(&config.database.path);
        if db_path.is_relative() {
            config.database.path = config_dir.join(db_path).to_string_lossy().into_owned();
        }

        config.config_path = std::fs::canonicalize(&path).unwrap_or(path);

        config.validate();
        config
    }

    fn validate(&self) {
        if self.server.admin_secret.is_empty() || self.server.admin_secret == "changeme" {
            eprintln!(
                "WARNING: admin_secret is '{}' — change it before deploying to production",
                if self.server.admin_secret.is_empty() {
                    "(empty)"
                } else {
                    &self.server.admin_secret
                }
            );
        }

        if !(1..=32).contains(&self.embedding.concurrency) {
            eprintln!(
                "FATAL: embedding.concurrency must be between 1 and 32, got {}",
                self.embedding.concurrency
            );
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_sources_token_env_defaults_to_tencent_docs_token() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(
            config.daily_sources.tencent_docs.token_env,
            DEFAULT_TENCENT_DOCS_TOKEN_ENV
        );
    }

    #[test]
    fn daily_sources_token_env_accepts_custom_name() {
        let config: Config =
            toml::from_str("[daily_sources.tencent_docs]\ntoken_env = \"MY_TENCENT_TOKEN\"\n")
                .unwrap();
        assert_eq!(
            config.daily_sources.tencent_docs.token_env,
            "MY_TENCENT_TOKEN"
        );
    }

    #[test]
    fn daily_sources_direct_token_takes_precedence_and_is_trimmed() {
        let config: Config = toml::from_str(
            "[daily_sources.tencent_docs]\ntoken = \" direct-token \"\ntoken_env = \"MISSING_TOKEN\"\n",
        )
        .unwrap();
        assert_eq!(
            config.daily_sources.tencent_docs.resolve_token().as_deref(),
            Some("direct-token")
        );
    }

    #[test]
    fn daily_sources_blank_direct_token_falls_back_to_environment() {
        let _env_lock = crate::utils::TEST_PATH_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        std::env::set_var("OJ_TEST_TENCENT_TOKEN", " env-token ");
        let config: Config = toml::from_str(
            "[daily_sources.tencent_docs]\ntoken = \"  \"\ntoken_env = \"OJ_TEST_TENCENT_TOKEN\"\n",
        )
        .unwrap();
        assert_eq!(
            config.daily_sources.tencent_docs.resolve_token().as_deref(),
            Some("env-token")
        );
        std::env::remove_var("OJ_TEST_TENCENT_TOKEN");
    }

    #[test]
    fn daily_sources_direct_token_does_not_require_environment_fallback() {
        let config: Config = toml::from_str(
            "[daily_sources.tencent_docs]\ntoken = \" direct-token \"\ntoken_env = \"  \"\n",
        )
        .unwrap();
        assert_eq!(
            config.daily_sources.tencent_docs.resolve_token().as_deref(),
            Some("direct-token")
        );
    }

    #[test]
    fn daily_sources_blank_token_env_disables_environment_fallback() {
        let config: Config =
            toml::from_str("[daily_sources.tencent_docs]\ntoken = \"  \"\ntoken_env = \"  \"\n")
                .unwrap();

        assert_eq!(config.daily_sources.tencent_docs.resolve_token(), None);
    }

    #[test]
    fn daily_sources_without_direct_token_or_environment_fallback_returns_none() {
        let config: Config =
            toml::from_str("[daily_sources.tencent_docs]\ntoken = \"  \"\ntoken_env = \"  \"\n")
                .unwrap();
        assert_eq!(config.daily_sources.tencent_docs.resolve_token(), None);
    }

    #[test]
    fn tencent_docs_debug_output_redacts_token() {
        let config = TencentDocsDailySourceConfig {
            token: "secret-token".into(),
            token_env: "TENCENT_DOCS_TOKEN".into(),
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("secret-token"));
    }

    #[test]
    fn config_example_documents_direct_token_placeholder_and_environment_fallback() {
        let example = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config.toml.example"),
        )
        .unwrap();
        assert!(example.contains("[daily_sources.tencent_docs]"));
        assert!(example.contains("token = \"\""));
        assert!(example.contains("token_env = \"TENCENT_DOCS_TOKEN\""));
        assert!(!example.contains("token_value"));
    }
}
