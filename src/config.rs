use std::net::IpAddr;
use std::path::PathBuf;

use tracing::trace;

/// Storage backend configuration
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "backend", rename_all = "lowercase")]
pub enum StorageConfig {
    /// In-memory storage (no persistence)
    #[serde(rename = "none")]
    None,

    /// SQLite database (default for most deployments)
    Sqlite {
        /// Path to the SQLite database file
        #[serde(default = "default_sqlite_path")]
        path: PathBuf,

        /// Retention period in days (metrics older than this are deleted)
        #[serde(default = "default_retention_days")]
        retention_days: u32,
    },
    // Future: PostgreSQL, Parquet, etc.
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig::Sqlite {
            path: default_sqlite_path(),
            retention_days: default_retention_days(),
        }
    }
}

fn default_sqlite_path() -> PathBuf {
    PathBuf::from("./metrics.db")
}

fn default_retention_days() -> u32 {
    30
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub servers: Option<Vec<ServerConfig>>,

    /// Storage configuration (optional - defaults to in-memory)
    pub storage: Option<StorageConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub ip: IpAddr,
    pub display: Option<String>,
    #[serde(default = "crate::util::get_default_port")]
    pub port: u16,
    #[serde(default = "default_interval")]
    pub interval: usize,
    pub token: Option<String>,
    pub limits: Option<Limits>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Limits {
    pub temperature: Option<Limit>,
    pub usage: Option<Limit>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Alert {
    Discord(Discord),
    Webhook(Webhook),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Webhook {
    pub url: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Discord {
    pub url: String,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Limit {
    pub limit: usize,
    pub grace: Option<usize>,
    pub alert: Option<Alert>,
}

fn default_interval() -> usize {
    15
}

pub fn read_config_file(path: &str) -> anyhow::Result<Config> {
    let file_content = std::fs::read_to_string(path)?;
    serde_json::from_str(&file_content)
        .map_err(|_| anyhow::anyhow!("Invalid configuration file provided!"))
        .inspect(|config| trace!("loaded config: {config:?}"))
}
