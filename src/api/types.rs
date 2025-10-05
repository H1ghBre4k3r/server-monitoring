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
