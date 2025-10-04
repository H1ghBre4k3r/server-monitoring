//! Database schema and metric row definitions
//!
//! ## Design Philosophy
//!
//! We use a **hybrid approach** to balance queryability with flexibility:
//!
//! ### Aggregate Metrics (Columns)
//! Store high-level aggregates as typed columns for efficient queries:
//! - `cpu_avg` - Average CPU usage across all cores
//! - `memory_used`, `memory_total` - Memory statistics
//! - `temp_avg` - Average temperature across components
//!
//! ### Detailed Metrics (JSON)
//! Store detailed breakdowns as JSON for flexibility:
//! - Per-core CPU usage: `{"cpu_0": 45.2, "cpu_1": 67.8, ...}`
//! - Per-component temperature: `{"CPU": 65.0, "GPU": 72.0, ...}`
//! - System information (rarely queried)
//!
//! This approach allows:
//! - Fast queries on aggregates (time series graphs, alerts)
//! - Complete data retention for detailed analysis
//! - Schema evolution without migrations (add new JSON fields)
//!
//! ## Schema Evolution
//!
//! When adding new metrics:
//! 1. If frequently queried → add column (requires migration)
//! 2. If rarely queried → add to metadata JSON (no migration)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ServerMetrics;

/// A single metric row stored in the database
///
/// This represents one metric reading from one server at one point in time.
/// For efficiency, we store both aggregate values (columns) and detailed
/// breakdowns (JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRow {
    /// When the metric was collected (always UTC)
    pub timestamp: DateTime<Utc>,

    /// Server identifier (format: "ip:port")
    pub server_id: String,

    /// Display name for the server (for UI/logging)
    pub display_name: String,

    /// Type of metric (for filtering/querying)
    pub metric_type: MetricType,

    // === Aggregate metrics (frequently queried) ===
    /// Average CPU usage across all cores (percentage 0-100)
    pub cpu_avg: Option<f32>,

    /// Memory used (bytes)
    pub memory_used: Option<u64>,

    /// Total memory (bytes)
    pub memory_total: Option<u64>,

    /// Average temperature across all components (Celsius)
    pub temp_avg: Option<f32>,

    // === Detailed metrics (full ServerMetrics struct) ===
    /// Complete ServerMetrics structure containing all detailed data
    ///
    /// Stored directly as the typed struct in memory, serialized to JSON
    /// only when writing to the database. This is more type-safe and
    /// avoids unnecessary serialization.
    pub metadata: ServerMetrics,
}

/// Type of metric stored
///
/// This enum allows filtering queries by metric type and
/// helps with retention policies (e.g., keep system metrics
/// longer than resource metrics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    /// Resource usage snapshot (CPU, memory, temperature)
    Resource,

    /// System information (kernel version, hostname, etc.)
    System,

    /// Custom/future metric types
    Custom,
}

impl MetricRow {
    /// Convert a ServerMetrics event into a MetricRow for storage
    ///
    /// This extracts aggregate values for indexed columns and stores
    /// the complete metrics for detailed queries.
    pub fn from_server_metrics(
        server_id: String,
        display_name: String,
        timestamp: DateTime<Utc>,
        metrics: &ServerMetrics,
    ) -> Self {
        // Extract aggregate values for indexed columns (fast queries)
        let cpu_avg = Some(metrics.cpus.average_usage);
        let memory_used = Some(metrics.memory.used);
        let memory_total = Some(metrics.memory.total);
        let temp_avg = metrics.components.average_temperature;

        Self {
            timestamp,
            server_id,
            display_name,
            metric_type: MetricType::Resource,
            cpu_avg,
            memory_used,
            memory_total,
            temp_avg,
            metadata: metrics.clone(), // Store the complete struct directly
        }
    }
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Resource => write!(f, "resource"),
            MetricType::System => write!(f, "system"),
            MetricType::Custom => write!(f, "custom"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentInformation, ComponentOverview, CpuInformation, CpuOverview, MemoryInformation,
        ServerMetrics, SystemInformation,
    };

    fn create_test_metrics() -> ServerMetrics {
        ServerMetrics {
            system: SystemInformation {
                name: Some("Ubuntu".to_string()),
                kernel_version: Some("5.15.0".to_string()),
                os_version: Some("22.04".to_string()),
                host_name: Some("test-server".to_string()),
            },
            memory: MemoryInformation {
                total: 16_000_000_000,
                used: 8_000_000_000,
                total_swap: 4_000_000_000,
                used_swap: 1_000_000_000,
            },
            cpus: CpuOverview {
                total: 2,
                arch: "x86_64".to_string(),
                average_usage: 55.5,
                cpus: vec![
                    CpuInformation {
                        name: "cpu0".to_string(),
                        frequency: 2400,
                        usage: 45.2,
                    },
                    CpuInformation {
                        name: "cpu1".to_string(),
                        frequency: 2400,
                        usage: 65.8,
                    },
                ],
            },
            components: ComponentOverview {
                average_temperature: Some(67.5),
                components: vec![
                    ComponentInformation {
                        name: "CPU".to_string(),
                        temperature: Some(65.0),
                    },
                    ComponentInformation {
                        name: "GPU".to_string(),
                        temperature: Some(70.0),
                    },
                ],
            },
        }
    }

    #[test]
    fn test_metric_row_from_server_metrics() {
        let metrics = create_test_metrics();
        let timestamp = Utc::now();

        let row = MetricRow::from_server_metrics(
            "192.168.1.100:3000".to_string(),
            "Test Server".to_string(),
            timestamp,
            &metrics,
        );

        // Check aggregate values extracted correctly
        assert_eq!(row.cpu_avg, Some(55.5));
        assert_eq!(row.memory_used, Some(8_000_000_000));
        assert_eq!(row.memory_total, Some(16_000_000_000));
        assert_eq!(row.temp_avg, Some(67.5));

        // Check metadata contains the complete ServerMetrics struct
        assert_eq!(row.metadata.cpus.cpus.len(), 2);
        assert_eq!(row.metadata.cpus.average_usage, 55.5);
        assert_eq!(row.metadata.cpus.cpus[0].name, "cpu0");
        assert_eq!(row.metadata.cpus.cpus[0].usage, 45.2);

        assert_eq!(row.metadata.components.components.len(), 2);
        assert_eq!(row.metadata.components.average_temperature, Some(67.5));

        assert_eq!(row.metadata.system.name, Some("Ubuntu".to_string()));
    }

    #[test]
    fn test_metric_type_display() {
        assert_eq!(MetricType::Resource.to_string(), "resource");
        assert_eq!(MetricType::System.to_string(), "system");
        assert_eq!(MetricType::Custom.to_string(), "custom");
    }
}
