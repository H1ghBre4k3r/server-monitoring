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
}
