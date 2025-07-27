use std::net::IpAddr;

use tracing::trace;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub servers: Option<Vec<ServerConfig>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub ip: IpAddr,
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

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct Limit {
    pub limit: usize,
    pub grace: Option<usize>,
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
