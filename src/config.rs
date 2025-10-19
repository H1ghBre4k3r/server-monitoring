use std::collections::HashMap;
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
    /// Named alert configurations (registry of reusable alerts)
    pub alerts: Option<HashMap<String, Alert>>,

    /// Default configurations for servers and services
    pub defaults: Option<DefaultConfig>,

    pub servers: Option<Vec<ServerConfig>>,

    /// Storage configuration (optional - defaults to in-memory)
    pub storage: Option<StorageConfig>,

    /// Service monitoring configuration (HTTP/HTTPS endpoints)
    pub services: Option<Vec<ServiceConfig>>,

    /// API server configuration (optional - API disabled if not specified)
    #[cfg(feature = "api")]
    pub api: Option<ApiConfig>,
}

/// Default configuration values for servers and services
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DefaultConfig {
    /// Default server configuration
    pub server: Option<DefaultServerConfig>,

    /// Default service configuration
    pub service: Option<DefaultServiceConfig>,
}

/// Default configuration for servers
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DefaultServerConfig {
    /// Default polling interval in seconds
    pub interval: Option<usize>,

    /// Default limits configuration
    pub limits: Option<Limits>,
}

/// Default configuration for services
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DefaultServiceConfig {
    /// Default check interval in seconds
    pub interval: Option<usize>,

    /// Default request timeout in seconds
    pub timeout: Option<usize>,

    /// Default grace period (consecutive failures before alerting)
    pub grace: Option<usize>,

    /// Default alert configuration (alert name reference)
    pub alert: Option<String>,
}

/// API server configuration
#[cfg(feature = "api")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiConfig {
    /// Bind address (e.g., "127.0.0.1" or "0.0.0.0")
    #[serde(default = "default_api_bind")]
    pub bind: String,

    /// Port to listen on
    #[serde(default = "default_api_port")]
    pub port: u16,

    /// Optional Bearer token for authentication
    pub auth_token: Option<String>,

    /// Enable CORS (for web dashboards)
    #[serde(default = "default_api_cors")]
    pub enable_cors: bool,
}

#[cfg(feature = "api")]
fn default_api_bind() -> String {
    "127.0.0.1".to_string()
}

#[cfg(feature = "api")]
fn default_api_port() -> u16 {
    8080
}

#[cfg(feature = "api")]
fn default_api_cors() -> bool {
    true
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
    /// Alert name reference (looks up in Config.alerts registry)
    pub alert: Option<String>,
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

    /// Alert name reference (looks up in Config.alerts registry)
    pub alert: Option<String>,
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

/// Resolved configuration with alert references replaced by actual Alert objects
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub servers: Vec<ResolvedServerConfig>,
    pub services: Vec<ResolvedServiceConfig>,
    pub storage: Option<StorageConfig>,
    #[cfg(feature = "api")]
    pub api: Option<ApiConfig>,
}

/// Resolved server configuration with actual Alert objects
#[derive(Debug, Clone)]
pub struct ResolvedServerConfig {
    pub ip: IpAddr,
    pub display: Option<String>,
    pub port: u16,
    pub interval: usize,
    pub token: Option<String>,
    pub limits: Option<ResolvedLimits>,
}

/// Resolved limits configuration
#[derive(Debug, Clone)]
pub struct ResolvedLimits {
    pub temperature: Option<ResolvedLimit>,
    pub usage: Option<ResolvedLimit>,
}

/// Resolved limit with actual Alert object
#[derive(Debug, Clone)]
pub struct ResolvedLimit {
    pub limit: usize,
    pub grace: Option<usize>,
    pub alert: Option<Alert>,
}

/// Resolved service configuration with actual Alert object
#[derive(Debug, Clone)]
pub struct ResolvedServiceConfig {
    pub name: String,
    pub url: String,
    pub interval: usize,
    pub timeout: usize,
    pub method: HttpMethod,
    pub expected_status: Option<Vec<u16>>,
    pub body_pattern: Option<String>,
    pub grace: Option<usize>,
    pub alert: Option<Alert>,
}

impl Config {
    /// Resolve configuration by merging defaults and replacing alert name references
    /// with actual Alert objects from the registry
    pub fn resolve(self) -> anyhow::Result<ResolvedConfig> {
        let alert_registry = self.alerts.as_ref();

        // Helper to resolve alert name to Alert object
        let resolve_alert = |alert_name: &Option<String>| -> anyhow::Result<Option<Alert>> {
            match alert_name {
                None => Ok(None),
                Some(name) => {
                    let alert_registry = alert_registry.ok_or_else(|| {
                        anyhow::anyhow!(
                            "Alert '{}' referenced but no alerts registry defined",
                            name
                        )
                    })?;
                    alert_registry
                        .get(name)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Alert '{}' not found in registry", name))
                        .map(Some)
                }
            }
        };

        // Get default configurations
        let default_server = self.defaults.as_ref().and_then(|d| d.server.as_ref());
        let default_service = self.defaults.as_ref().and_then(|d| d.service.as_ref());

        // Resolve servers
        let servers = self
            .servers
            .unwrap_or_default()
            .into_iter()
            .map(|server| {
                // Merge limits with defaults
                let limits = match (server.limits, default_server.and_then(|d| d.limits.clone())) {
                    (Some(server_limits), Some(default_limits)) => {
                        // Server has limits - merge with defaults
                        Some(ResolvedLimits {
                            temperature: match (
                                server_limits.temperature,
                                default_limits.temperature,
                            ) {
                                (Some(server_temp), Some(default_temp)) => Some(ResolvedLimit {
                                    limit: server_temp.limit,
                                    grace: server_temp.grace.or(default_temp.grace),
                                    alert: resolve_alert(
                                        &server_temp.alert.or(default_temp.alert),
                                    )?,
                                }),
                                (Some(server_temp), None) => Some(ResolvedLimit {
                                    limit: server_temp.limit,
                                    grace: server_temp.grace,
                                    alert: resolve_alert(&server_temp.alert)?,
                                }),
                                (None, Some(default_temp)) => Some(ResolvedLimit {
                                    limit: default_temp.limit,
                                    grace: default_temp.grace,
                                    alert: resolve_alert(&default_temp.alert)?,
                                }),
                                (None, None) => None,
                            },
                            usage: match (server_limits.usage, default_limits.usage) {
                                (Some(server_usage), Some(default_usage)) => Some(ResolvedLimit {
                                    limit: server_usage.limit,
                                    grace: server_usage.grace.or(default_usage.grace),
                                    alert: resolve_alert(
                                        &server_usage.alert.or(default_usage.alert),
                                    )?,
                                }),
                                (Some(server_usage), None) => Some(ResolvedLimit {
                                    limit: server_usage.limit,
                                    grace: server_usage.grace,
                                    alert: resolve_alert(&server_usage.alert)?,
                                }),
                                (None, Some(default_usage)) => Some(ResolvedLimit {
                                    limit: default_usage.limit,
                                    grace: default_usage.grace,
                                    alert: resolve_alert(&default_usage.alert)?,
                                }),
                                (None, None) => None,
                            },
                        })
                    }
                    (Some(server_limits), None) => {
                        // Server has limits but no defaults
                        Some(ResolvedLimits {
                            temperature: server_limits
                                .temperature
                                .map(|temp| {
                                    Ok::<_, anyhow::Error>(ResolvedLimit {
                                        limit: temp.limit,
                                        grace: temp.grace,
                                        alert: resolve_alert(&temp.alert)?,
                                    })
                                })
                                .transpose()?,
                            usage: server_limits
                                .usage
                                .map(|usage| {
                                    Ok::<_, anyhow::Error>(ResolvedLimit {
                                        limit: usage.limit,
                                        grace: usage.grace,
                                        alert: resolve_alert(&usage.alert)?,
                                    })
                                })
                                .transpose()?,
                        })
                    }
                    (None, Some(default_limits)) => {
                        // No server limits - use defaults
                        Some(ResolvedLimits {
                            temperature: default_limits
                                .temperature
                                .map(|temp| {
                                    Ok::<_, anyhow::Error>(ResolvedLimit {
                                        limit: temp.limit,
                                        grace: temp.grace,
                                        alert: resolve_alert(&temp.alert)?,
                                    })
                                })
                                .transpose()?,
                            usage: default_limits
                                .usage
                                .map(|usage| {
                                    Ok::<_, anyhow::Error>(ResolvedLimit {
                                        limit: usage.limit,
                                        grace: usage.grace,
                                        alert: resolve_alert(&usage.alert)?,
                                    })
                                })
                                .transpose()?,
                        })
                    }
                    (None, None) => None,
                };

                Ok(ResolvedServerConfig {
                    ip: server.ip,
                    display: server.display,
                    port: server.port,
                    interval: server.interval,
                    token: server.token,
                    limits,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Resolve services
        let services = self
            .services
            .unwrap_or_default()
            .into_iter()
            .map(|service| {
                let resolved_alert = resolve_alert(
                    &service
                        .alert
                        .or_else(|| default_service.and_then(|d| d.alert.clone())),
                )?;

                Ok(ResolvedServiceConfig {
                    name: service.name,
                    url: service.url,
                    interval: service.interval,
                    timeout: service.timeout,
                    method: service.method,
                    expected_status: service.expected_status,
                    body_pattern: service.body_pattern,
                    grace: service
                        .grace
                        .or_else(|| default_service.and_then(|d| d.grace)),
                    alert: resolved_alert,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(ResolvedConfig {
            servers,
            services,
            storage: self.storage,
            #[cfg(feature = "api")]
            api: self.api,
        })
    }
}
