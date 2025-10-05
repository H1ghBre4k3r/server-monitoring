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

        /// Cleanup interval in hours (how often to run retention cleanup)
        #[serde(default = "default_cleanup_interval_hours")]
        cleanup_interval_hours: u32,
    },
    // Future: PostgreSQL, Parquet, etc.
}

impl StorageConfig {
    /// Validate storage configuration parameters
    pub fn validate(&self) -> Result<(), String> {
        match self {
            StorageConfig::None => Ok(()),
            StorageConfig::Sqlite {
                retention_days,
                cleanup_interval_hours,
                ..
            } => {
                // Validate retention_days: 1 day to 10 years
                if *retention_days < 1 {
                    return Err("retention_days must be at least 1".to_string());
                }
                if *retention_days > 3650 {
                    return Err("retention_days cannot exceed 3650 (10 years)".to_string());
                }

                // Validate cleanup_interval_hours: 1 hour to 30 days
                if *cleanup_interval_hours < 1 {
                    return Err("cleanup_interval_hours must be at least 1".to_string());
                }
                if *cleanup_interval_hours > 720 {
                    return Err("cleanup_interval_hours cannot exceed 720 (30 days)".to_string());
                }

                // Warn if cleanup interval is longer than retention period
                let retention_hours = *retention_days as u64 * 24;
                if (*cleanup_interval_hours as u64) > retention_hours {
                    tracing::warn!(
                        "cleanup_interval_hours ({}) is longer than retention period ({} hours). \
                         Old data may accumulate.",
                        cleanup_interval_hours,
                        retention_hours
                    );
                }

                Ok(())
            }
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig::Sqlite {
            path: default_sqlite_path(),
            retention_days: default_retention_days(),
            cleanup_interval_hours: default_cleanup_interval_hours(),
        }
    }
}

fn default_sqlite_path() -> PathBuf {
    PathBuf::from("./metrics.db")
}

fn default_retention_days() -> u32 {
    30
}

fn default_cleanup_interval_hours() -> u32 {
    24 // Run cleanup once per day by default
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub servers: Option<Vec<ServerConfig>>,

    /// Storage configuration (optional - defaults to in-memory)
    pub storage: Option<StorageConfig>,

    /// Service monitoring configuration (HTTP/HTTPS endpoints)
    pub services: Option<Vec<ServiceConfig>>,
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

/// HTTP method for service checks
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Head,
}

/// Service monitoring configuration (HTTP/HTTPS endpoints)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServiceConfig {
    /// Service name (for display and identification)
    pub name: String,

    /// URL to monitor (HTTP or HTTPS)
    pub url: String,

    /// Check interval in seconds
    #[serde(default = "default_service_interval")]
    pub interval: usize,

    /// Request timeout in seconds
    #[serde(default = "default_service_timeout")]
    pub timeout: usize,

    /// HTTP method to use
    #[serde(default)]
    pub method: HttpMethod,

    /// Expected HTTP status codes (e.g., [200, 201, 204])
    /// If not specified, any 2xx status is considered success
    pub expected_status: Option<Vec<u16>>,

    /// Optional regex pattern to match in response body
    pub body_pattern: Option<String>,

    /// Consecutive failures before alerting
    pub grace: Option<usize>,

    /// Alert configuration
    pub alert: Option<Alert>,
}

fn default_service_interval() -> usize {
    60 // Check every 60 seconds by default
}

fn default_service_timeout() -> usize {
    10 // 10 second timeout by default
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
