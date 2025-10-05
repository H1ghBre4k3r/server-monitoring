//! Shared API response types
//!
//! This module contains types that are shared between the API and the viewer (TUI).
//! By centralizing these types, we ensure consistency in serialization/deserialization
//! and avoid type drift between components.

use serde::{Deserialize, Serialize};

/// Server information with health status
///
/// Returned by GET /api/v1/servers and used by the viewer for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server identifier (format: "ip:port")
    pub server_id: String,

    /// Human-readable display name
    pub display_name: String,

    /// Monitoring status: "active", "paused", etc.
    pub monitoring_status: String,

    /// Health status: "up", "stale", "unknown"
    pub health_status: String,

    /// Last time metrics were received (RFC 3339 timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
}

/// Service information with health status
///
/// Returned by GET /api/v1/services and used by the viewer for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name from configuration
    pub name: String,

    /// URL being monitored
    pub url: String,

    /// Monitoring status: "active", "paused", etc.
    pub monitoring_status: String,

    /// Health status: "up", "down", "degraded", "stale", "unknown"
    pub health_status: String,

    /// Last check timestamp (RFC 3339 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_check: Option<String>,

    /// Last status result: "up", "down", or "degraded"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
}

// ============================================================================
// API Response Types - Proper typed responses instead of Json<Value>
// ============================================================================

/// Response for GET /api/v1/servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServersResponse {
    pub servers: Vec<ServerInfo>,
    pub count: usize,
}

/// Response for GET /api/v1/services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesResponse {
    pub services: Vec<ServiceInfo>,
    pub count: usize,
}

/// Response for GET /api/v1/health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
}

/// Response for GET /api/v1/servers/:id/metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub server_id: String,
    pub start: String,
    pub end: String,
    pub count: usize,
    pub metrics: Vec<crate::storage::schema::MetricRow>,
}

/// Response for GET /api/v1/servers/:id/metrics/latest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestMetricsResponse {
    pub server_id: String,
    pub count: usize,
    pub metrics: Vec<crate::storage::schema::MetricRow>,
}

/// Response for GET /api/v1/services/:name/checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceChecksResponse {
    pub service_name: String,
    pub start: String,
    pub end: String,
    pub count: usize,
    pub checks: Vec<crate::storage::schema::ServiceCheckRow>,
}

/// Response for GET /api/v1/services/:name/uptime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeResponse {
    pub service_name: String,
    pub since: String,
    pub start: String,
    pub end: String,
    pub uptime_percentage: f64,
    pub total_checks: usize,
    pub successful_checks: usize,
    pub avg_response_time_ms: Option<f64>,
}

/// Response for GET /api/v1/stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsResponse {
    pub timestamp: String,
    pub storage: StorageStats,
    pub collectors: usize,
    pub service_monitors: usize,
}

/// Storage statistics subset for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_metrics: usize,
    pub buffer_size: usize,
    pub flush_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_cleanup: Option<String>,
    pub total_metrics_deleted: u64,
    pub total_service_checks_deleted: u64,
}
