use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetrics {
    pub system: SystemInformation,
    pub memory: MemoryInformation,
    pub cpus: CpuOverview,
    pub components: Vec<ComponentInformation>,
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
    pub cpus: Vec<CpuInformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInformation {
    pub name: String,
    pub frequency: u64,
    pub usage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInformation {
    pub name: String,
    pub temperature: Option<f32>,
}
