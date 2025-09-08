pub mod alerts;
pub mod config;
pub mod discord;
pub mod monitors;
pub mod util;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetrics {
    pub system: SystemInformation,
    pub memory: MemoryInformation,
    pub cpus: CpuOverview,
    pub components: ComponentOverview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInformation {
    pub name: Option<String>,
    pub kernel_version: Option<String>,
    pub os_version: Option<String>,
    pub host_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInformation {
    pub total: u64,
    pub used: u64,
    pub total_swap: u64,
    pub used_swap: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuOverview {
    pub total: usize,
    pub arch: String,
    pub average_usage: f32,
    pub cpus: Vec<CpuInformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInformation {
    pub name: String,
    pub frequency: u64,
    pub usage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentOverview {
    pub average_temperature: Option<f32>,
    pub components: Vec<ComponentInformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInformation {
    pub name: String,
    pub temperature: Option<f32>,
}
