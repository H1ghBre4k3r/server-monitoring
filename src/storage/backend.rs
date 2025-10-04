//! Storage backend trait definition
//!
//! This module defines the core `StorageBackend` trait that all
//! storage implementations must implement.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::StorageResult;
use super::schema::MetricRow;

/// Query parameters for fetching metrics within a time range
#[derive(Debug, Clone)]
pub struct QueryRange {
    /// Server to query (format: "ip:port")
    pub server_id: String,

    /// Start of time range (inclusive)
    pub start: DateTime<Utc>,

    /// End of time range (inclusive)
    pub end: DateTime<Utc>,

    /// Maximum number of results to return (for pagination)
    pub limit: Option<usize>,
}

/// Health status of the storage backend
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Is the backend operational?
    pub healthy: bool,

    /// Human-readable status message
    pub message: String,

    /// Additional backend-specific metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Trait for persistent storage backends
///
/// All storage backends (SQLite, PostgreSQL, Parquet, etc.) must
/// implement this trait. The trait is designed to be:
///
/// - **Async**: All methods are async for compatibility with Tokio
/// - **Batch-oriented**: `insert_batch` is the primary write method
/// - **Queryable**: Support time-range queries for dashboards
/// - **Maintainable**: Health checks and cleanup operations
///
/// ## Thread Safety
///
/// Implementations must be `Send + Sync` as they will be used
/// across async tasks.
///
/// ## Error Handling
///
/// Methods return `StorageResult<T>` which wraps `StorageError`.
/// Implementations should convert backend-specific errors to
/// `StorageError` variants.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Insert a batch of metrics
    ///
    /// This is the primary write method and must be optimized for throughput.
    /// Implementations should:
    /// - Use transactions for atomicity
    /// - Batch SQL statements for efficiency
    /// - Handle partial failures gracefully
    ///
    /// ## Performance Target
    ///
    /// Should handle 1000+ metrics in <100ms on modern hardware.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use server_monitoring::storage::{StorageBackend, schema::MetricRow};
    /// # async fn example(backend: &dyn StorageBackend, metrics: Vec<MetricRow>) {
    /// backend.insert_batch(metrics).await.expect("insert failed");
    /// # }
    /// ```
    async fn insert_batch(&self, metrics: Vec<MetricRow>) -> StorageResult<()>;

    /// Query metrics within a time range
    ///
    /// Returns metrics for a specific server between start and end times.
    /// Results are ordered by timestamp (oldest first).
    ///
    /// ## Performance
    ///
    /// Implementations should use indexes on (server_id, timestamp)
    /// for efficient range scans.
    async fn query_range(&self, query: QueryRange) -> StorageResult<Vec<MetricRow>>;

    /// Get the N most recent metrics for a server
    ///
    /// This is optimized for dashboard displays that show
    /// "last 10 minutes" of data.
    async fn query_latest(&self, server_id: &str, limit: usize) -> StorageResult<Vec<MetricRow>>;

    /// Delete metrics older than the specified timestamp
    ///
    /// Used for retention policy enforcement. Should be called
    /// periodically (e.g., daily) to prevent unbounded growth.
    ///
    /// Returns the number of metrics deleted.
    async fn cleanup_old_metrics(&self, before: DateTime<Utc>) -> StorageResult<usize>;

    /// Check backend health
    ///
    /// Performs a lightweight operation to verify the backend
    /// is operational (e.g., ping database, check file access).
    ///
    /// ## Usage
    ///
    /// Called periodically by the StorageActor to monitor backend health.
    /// If unhealthy, the actor can log warnings or attempt reconnection.
    async fn health_check(&self) -> StorageResult<HealthStatus>;

    /// Get backend-specific statistics
    ///
    /// Returns human-readable stats about the backend
    /// (e.g., "SQLite: 1.2M rows, 450MB on disk").
    async fn get_stats(&self) -> StorageResult<String>;

    /// Close the backend and release resources
    ///
    /// Gracefully shuts down the backend, closing connections
    /// and flushing any pending writes.
    async fn close(&self) -> StorageResult<()>;
}
