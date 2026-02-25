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
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self { timeout_secs: 300 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EmbeddingConfig {
    pub timeout_secs: u64,
    pub over_fetch_factor: u32,
    pub concurrency: u32,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub crawler: CrawlerConfig,
    pub embedding: EmbeddingConfig,
    pub logging: LoggingConfig,
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

        let mut config: Config = toml::from_str(&content).unwrap_or_else(|e| {
            eprintln!(
                "FATAL: failed to parse configuration file '{}': {}",
                path.display(),
                e
            );
            std::process::exit(1);
        });

        let config_dir = path.parent().unwrap_or(Path::new("."));

        // Resolve database.path relative to config file directory
        let db_path = Path::new(&config.database.path);
        if db_path.is_relative() {
            config.database.path = config_dir
                .join(db_path)
                .to_string_lossy()
                .into_owned();
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
