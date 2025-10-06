//! Message types for actor communication
//!
//! This module defines all message types used for communication between actors.
//!
//! ## Design Principles
//!
//! 1. **Commands**: Request/response messages sent to specific actors via mpsc
//! 2. **Events**: Broadcast notifications published to multiple subscribers
//! 3. **Immutability**: All messages are cloneable for multi-subscriber patterns

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::ServerMetrics;

#[cfg(feature = "storage-sqlite")]
use crate::storage::{
    backend::QueryRange,
    schema::{MetricRow, ServiceCheckRow, UptimeStats},
};

/// Event published when metrics are collected from a server
///
/// This event is broadcast to all interested actors (AlertActor, StorageActor, ApiActor).
/// The broadcast channel may lag or drop messages for slow subscribers - this is acceptable
/// as metrics are continuously generated and storage can handle gaps.
#[derive(Debug, Clone)]
pub struct MetricEvent {
    /// Unique identifier for the server (format: "ip:port")
    pub server_id: String,

    /// The collected metrics
    pub metrics: ServerMetrics,

    /// When the metrics were collected
    pub timestamp: DateTime<Utc>,

    /// Display name for the server (for logging/alerts)
    pub display_name: String,
}

/// Event published when polling status changes for a server
///
/// This event tracks whether the collector can successfully reach the server,
/// regardless of the metrics themselves. This helps distinguish between
/// "server is down" vs "metrics are old" scenarios.
#[derive(Debug, Clone)]
pub struct PollingStatusEvent {
    /// Unique identifier for the server (format: "ip:port")
    pub server_id: String,

    /// When the poll attempt occurred
    pub timestamp: DateTime<Utc>,

    /// Display name for the server (for logging/alerts)
    pub display_name: String,

    /// Whether the poll was successful
    pub success: bool,

    /// Error message if poll failed
    pub error_message: Option<String>,
}

/// Commands that can be sent to a MetricCollectorActor
#[derive(Debug)]
pub enum CollectorCommand {
    /// Trigger an immediate poll (bypassing the interval timer)
    ///
    /// Used for testing and manual refresh operations.
    PollNow {
        /// Channel to send the result back
        respond_to: oneshot::Sender<anyhow::Result<()>>,
    },

    /// Update the polling interval
    ///
    /// The new interval takes effect after the next poll completes.
    UpdateInterval {
        /// New interval in seconds
        interval_secs: u64,
    },

    /// Gracefully shut down the collector
    ///
    /// The actor will finish any in-flight poll and then exit.
    Shutdown,
}

/// Commands that can be sent to the AlertActor
#[derive(Debug)]
pub enum AlertCommand {
    /// Get the current alert state for a server
    GetState {
        server_id: String,
        respond_to: oneshot::Sender<Option<AlertState>>,
    },

    /// Mute alerts for a duration
    ///
    /// Useful for maintenance windows.
    MuteAlerts { duration_secs: u64 },

    /// Unmute alerts
    UnmuteAlerts,

    /// Gracefully shut down the alert actor
    Shutdown,
}

/// Current alert state for a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertState {
    /// Server identifier
    pub server_id: String,

    /// CPU usage grace period state
    pub cpu_consecutive_exceeds: usize,

    /// Temperature grace period state
    pub temp_consecutive_exceeds: usize,

    /// Last metric evaluation timestamp
    pub last_evaluation: DateTime<Utc>,
}

/// Commands that can be sent to the StorageActor
#[derive(Debug)]
pub enum StorageCommand {
    /// Manually flush write buffer to storage
    Flush {
        respond_to: oneshot::Sender<anyhow::Result<()>>,
    },

    /// Get storage statistics
    GetStats {
        respond_to: oneshot::Sender<StorageStats>,
    },

    /// Query metrics within a time range (Phase 2 - with persistent backend)
    #[cfg(feature = "storage-sqlite")]
    QueryRange {
        query: QueryRange,
        respond_to: oneshot::Sender<anyhow::Result<Vec<MetricRow>>>,
    },

    /// Query the latest N metrics for a server (Phase 2 - with persistent backend)
    #[cfg(feature = "storage-sqlite")]
    QueryLatest {
        server_id: String,
        limit: usize,
        respond_to: oneshot::Sender<anyhow::Result<Vec<MetricRow>>>,
    },

    /// Check backend health (Phase 2 - with persistent backend)
    #[cfg(feature = "storage-sqlite")]
    HealthCheck {
        respond_to: oneshot::Sender<anyhow::Result<String>>,
    },

    // ========================================================================
    // Service Check Commands (Phase 3)
    // ========================================================================
    /// Query service checks within a time range (Phase 3 - with persistent backend)
    #[cfg(feature = "storage-sqlite")]
    QueryServiceChecksRange {
        service_name: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        respond_to: oneshot::Sender<anyhow::Result<Vec<ServiceCheckRow>>>,
    },

    /// Query the latest N service checks for a service (Phase 3 - with persistent backend)
    #[cfg(feature = "storage-sqlite")]
    QueryLatestServiceChecks {
        service_name: String,
        limit: usize,
        respond_to: oneshot::Sender<anyhow::Result<Vec<ServiceCheckRow>>>,
    },

    /// Calculate uptime statistics for a service (Phase 3 - with persistent backend)
    #[cfg(feature = "storage-sqlite")]
    CalculateUptime {
        service_name: String,
        since: DateTime<Utc>,
        respond_to: oneshot::Sender<anyhow::Result<UptimeStats>>,
    },

    /// Cleanup old service checks (Phase 3 - with persistent backend)
    #[cfg(feature = "storage-sqlite")]
    CleanupOldServiceChecks {
        before: DateTime<Utc>,
        respond_to: oneshot::Sender<anyhow::Result<usize>>,
    },

    /// Gracefully shut down the storage actor
    Shutdown,
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total metrics stored (in-memory count for Phase 1)
    pub total_metrics: usize,

    /// Number of metrics in write buffer
    pub buffer_size: usize,

    /// Number of flush operations performed
    pub flush_count: u64,

    /// Last cleanup timestamp (Phase 4 - retention)
    pub last_cleanup_time: Option<DateTime<Utc>>,

    /// Total metrics deleted by cleanup operations (Phase 4 - retention)
    pub total_metrics_deleted: u64,

    /// Total service checks deleted by cleanup operations (Phase 4 - retention)
    pub total_service_checks_deleted: u64,
}

// ============================================================================
// Service Monitoring Messages (Phase 3)
// ============================================================================

// Re-export it for backward compatibility
pub use crate::api::types::ServiceStatus;

/// Event published when a service health check is performed
///
/// This event is broadcast to all interested actors (AlertActor, StorageActor).
#[derive(Debug, Clone)]
pub struct ServiceCheckEvent {
    /// Service name (from configuration)
    pub service_name: String,

    /// URL that was checked
    pub url: String,

    /// When the check was performed
    pub timestamp: DateTime<Utc>,

    /// Overall status result
    pub status: ServiceStatus,

    /// Response time in milliseconds (if request succeeded)
    pub response_time_ms: Option<u64>,

    /// HTTP status code (if received)
    pub http_status_code: Option<u16>,

    /// SSL certificate expiry in days (if HTTPS and available)
    pub ssl_expiry_days: Option<i64>,

    /// Error message (if check failed)
    pub error_message: Option<String>,
}

/// Commands that can be sent to a ServiceMonitorActor
#[derive(Debug)]
pub enum ServiceCommand {
    /// Trigger an immediate health check (bypassing the interval timer)
    CheckNow {
        /// Channel to send the result back
        respond_to: oneshot::Sender<anyhow::Result<()>>,
    },

    /// Update the check interval
    ///
    /// The new interval takes effect after the next check completes.
    UpdateInterval {
        /// New interval in seconds
        interval_secs: u64,
    },

    /// Gracefully shut down the service monitor
    Shutdown,
}
