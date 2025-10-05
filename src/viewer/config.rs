//! Configuration for the TUI viewer

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Viewer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// API server URL
    pub api_url: String,

    /// API authentication token (optional)
    pub api_token: Option<String>,

    /// Refresh interval in seconds (default: 5)
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,

    /// Maximum metrics to display per server (default: 100)
    #[serde(default = "default_max_metrics")]
    pub max_metrics: usize,

    /// Chart time window in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_time_window")]
    pub time_window_seconds: u64,

    /// Enable debug mode (default: false)
    #[serde(default)]
    pub debug: bool,
}

fn default_refresh_interval() -> u64 {
    5
}

fn default_max_metrics() -> usize {
    100
}

fn default_time_window() -> u64 {
    300 // 5 minutes
}

impl Config {
    /// Load configuration from file, or use defaults if file doesn't exist
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = path.map(|p| p.to_path_buf()).or_else(|| {
            // Try default locations
            let home = dirs::home_dir()?;
            let default_path = home.join(".config/guardia/viewer.toml");
            if default_path.exists() {
                Some(default_path)
            } else {
                None
            }
        });

        if let Some(path) = config_path {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;

            toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))
        } else {
            // Use defaults
            Ok(Self::default())
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:8080".to_string(),
            api_token: None,
            refresh_interval: default_refresh_interval(),
            max_metrics: default_max_metrics(),
            time_window_seconds: default_time_window(),
            debug: false,
        }
    }
}
