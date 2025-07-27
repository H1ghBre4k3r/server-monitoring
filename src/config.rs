use std::net::IpAddr;

use tracing::trace;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub servers: Option<Vec<ServerConfig>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub ip: IpAddr,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_interval")]
    pub interval: usize,
    pub token: Option<String>,
}

fn default_port() -> u16 {
    51243
}

fn default_interval() -> usize {
    15
}

fn default_retries() -> i64 {
    0
}

pub fn read_config_file(path: &str) -> anyhow::Result<Config> {
    let file_content = std::fs::read_to_string(path)?;
    serde_json::from_str(&file_content)
        .map_err(|_| anyhow::anyhow!("Invalid configuration file provided!"))
        .inspect(|config| trace!("loaded config: {config:?}"))
}
