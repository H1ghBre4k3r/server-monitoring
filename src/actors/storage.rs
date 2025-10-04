//! StorageActor - Persists metrics to storage
//!
//! ## Architecture
//!
//! The StorageActor supports two modes:
//!
//! ### In-Memory Mode (default, backward compatible)
//! - Ring buffer with fixed capacity
//! - No persistence - data lost on restart
//! - Fast, no I/O overhead
//!
//! ### Persistent Mode (Phase 2)
//! - Pluggable backend via StorageBackend trait
//! - Batch writes for performance
//! - Dual flush triggers (size + time)
//! - Query support for historical data
//!
//! ## Batching Strategy
//!
//! When using a persistent backend, metrics are batched for efficiency:
//! - **Size trigger**: Flush after 100 metrics (configurable)
//! - **Time trigger**: Flush after 5 seconds (configurable)
//!
//! This balances write throughput with data freshness.

use std::collections::VecDeque;
use std::time::Duration;

use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tracing::{debug, error, info, instrument, trace, warn};

use super::messages::{MetricEvent, StorageCommand, StorageStats};

#[cfg(feature = "storage-sqlite")]
use crate::storage::{StorageBackend, backend::QueryRange, schema::MetricRow};

/// Maximum metrics to keep in in-memory buffer (ring buffer)
const MAX_BUFFER_SIZE: usize = 1000;

/// Batch size trigger - flush after this many metrics
const BATCH_SIZE_TRIGGER: usize = 100;

/// Batch time trigger - flush after this duration
const BATCH_TIME_TRIGGER: Duration = Duration::from_secs(5);

/// Cleanup interval - run retention cleanup daily
const CLEANUP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

/// Storage actor with optional persistent backend
///
/// Supports two modes:
/// 1. In-memory (backend = None): Ring buffer, no persistence
/// 2. Persistent (backend = Some): Batched writes to backend
pub struct StorageActor {
    /// Optional persistent backend (None = in-memory only)
    #[cfg(feature = "storage-sqlite")]
    backend: Option<Box<dyn StorageBackend>>,

    /// Batch buffer for persistent backend (metrics waiting to be flushed)
    #[cfg(feature = "storage-sqlite")]
    batch_buffer: Vec<MetricRow>,

    /// In-memory ring buffer (used when backend is None, or as cache)
    memory_buffer: VecDeque<MetricEvent>,

    /// Command receiver
    command_rx: mpsc::Receiver<StorageCommand>,

    /// Metric event receiver (broadcast subscription)
    metric_rx: broadcast::Receiver<MetricEvent>,

    /// Flush counter (for stats)
    flush_count: u64,

    /// Retention period in days (for automatic cleanup)
    #[cfg(feature = "storage-sqlite")]
    retention_days: Option<u32>,
}

impl StorageActor {
    /// Create a new storage actor with optional backend
    #[cfg(feature = "storage-sqlite")]
    pub fn new(
        command_rx: mpsc::Receiver<StorageCommand>,
        metric_rx: broadcast::Receiver<MetricEvent>,
        backend: Option<Box<dyn StorageBackend>>,
        retention_days: Option<u32>,
    ) -> Self {
        let mode = if backend.is_some() {
            "persistent"
        } else {
            "in-memory"
        };
        debug!("creating storage actor in {mode} mode");

        if let Some(days) = retention_days {
            debug!("retention cleanup enabled: {} days", days);
        }

        Self {
            backend,
            batch_buffer: Vec::with_capacity(BATCH_SIZE_TRIGGER),
            memory_buffer: VecDeque::with_capacity(MAX_BUFFER_SIZE),
            command_rx,
            metric_rx,
            flush_count: 0,
            retention_days,
        }
    }

    /// Create a new in-memory storage actor (backward compat)
    #[cfg(not(feature = "storage-sqlite"))]
    pub fn new(
        command_rx: mpsc::Receiver<StorageCommand>,
        metric_rx: broadcast::Receiver<MetricEvent>,
    ) -> Self {
        debug!("creating storage actor in in-memory mode");

        Self {
            memory_buffer: VecDeque::with_capacity(MAX_BUFFER_SIZE),
            command_rx,
            metric_rx,
            flush_count: 0,
        }
    }

    /// Run the actor's main loop
    #[instrument(skip(self))]
    pub async fn run(mut self) {
        #[cfg(feature = "storage-sqlite")]
        let has_backend = self.backend.is_some();

        #[cfg(feature = "storage-sqlite")]
        let has_retention = self.retention_days.is_some();

        #[cfg(feature = "storage-sqlite")]
        debug!(
            "starting storage actor (mode: {})",
            if has_backend {
                "persistent"
            } else {
                "in-memory"
            }
        );

        #[cfg(not(feature = "storage-sqlite"))]
        debug!("starting storage actor (in-memory mode)");

        #[cfg(feature = "storage-sqlite")]
        let mut flush_interval = time::interval(BATCH_TIME_TRIGGER);

        // Cleanup interval for retention policy
        #[cfg(feature = "storage-sqlite")]
        let mut cleanup_interval = time::interval(CLEANUP_INTERVAL);

        // Run initial cleanup on startup if retention is configured
        #[cfg(feature = "storage-sqlite")]
        if has_backend && has_retention {
            debug!("running initial retention cleanup on startup");
            self.run_cleanup().await;
        }

        loop {
            #[cfg(feature = "storage-sqlite")]
            {
                tokio::select! {
                    // Receive metric events
                    result = self.metric_rx.recv() => {
                        match result {
                            Ok(event) => {
                                self.store_metric(event).await;
                            }
                            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                                warn!("storage actor lagged, skipped {skipped} metrics");
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                warn!("metric channel closed, shutting down");
                                break;
                            }
                        }
                    }

                    // Time-based flush trigger (only with persistent backend)
                    _ = flush_interval.tick(), if has_backend => {
                        if !self.batch_buffer.is_empty() {
                            trace!("time-based flush triggered ({} metrics)", self.batch_buffer.len());
                            self.flush_batch().await;
                        }
                    }

                    // Cleanup trigger for retention policy (daily)
                    _ = cleanup_interval.tick(), if has_backend && has_retention => {
                        debug!("daily retention cleanup triggered");
                        self.run_cleanup().await;
                    }

                    // Handle commands
                    Some(cmd) = self.command_rx.recv() => {
                        self.handle_command(cmd).await;
                    }

                    // Command channel closed
                    else => {
                        warn!("command channel closed, shutting down");
                        break;
                    }
                }
            }

            #[cfg(not(feature = "storage-sqlite"))]
            {
                tokio::select! {
                    // Receive metric events
                    result = self.metric_rx.recv() => {
                        match result {
                            Ok(event) => {
                                self.store_metric(event).await;
                            }
                            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                                warn!("storage actor lagged, skipped {skipped} metrics");
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                warn!("metric channel closed, shutting down");
                                break;
                            }
                        }
                    }

                    // Handle commands
                    Some(cmd) = self.command_rx.recv() => {
                        self.handle_command(cmd).await;
                    }

                    // Command channel closed
                    else => {
                        warn!("command channel closed, shutting down");
                        break;
                    }
                }
            }
        }

        // Final flush before shutdown
        #[cfg(feature = "storage-sqlite")]
        if has_backend && !self.batch_buffer.is_empty() {
            debug!(
                "final flush before shutdown ({} metrics)",
                self.batch_buffer.len()
            );
            self.flush_batch().await;
        }

        #[cfg(feature = "storage-sqlite")]
        if let Some(backend) = self.backend.as_ref() {
            debug!("closing backend");
            if let Err(e) = backend.close().await {
                error!("error closing backend: {}", e);
            }
        }

        debug!("storage actor stopped");
    }

    /// Store a metric (either in batch buffer or memory buffer)
    async fn store_metric(&mut self, event: MetricEvent) {
        trace!(
            "storing metric for {} at {}",
            event.server_id, event.timestamp
        );

        // Always add to memory buffer for recent queries
        self.memory_buffer.push_back(event.clone());
        if self.memory_buffer.len() > MAX_BUFFER_SIZE {
            self.memory_buffer.pop_front();
        }

        // If we have a persistent backend, add to batch buffer
        #[cfg(feature = "storage-sqlite")]
        if self.backend.is_some() {
            let row = MetricRow::from_server_metrics(
                event.server_id.clone(),
                event.display_name.clone(),
                event.timestamp,
                &event.metrics,
            );

            self.batch_buffer.push(row);

            // Size-based flush trigger
            if self.batch_buffer.len() >= BATCH_SIZE_TRIGGER {
                trace!(
                    "size-based flush triggered ({} metrics)",
                    self.batch_buffer.len()
                );
                self.flush_batch().await;
            }
        }
    }

    /// Flush the batch buffer to persistent backend
    #[cfg(feature = "storage-sqlite")]
    async fn flush_batch(&mut self) {
        if let Some(backend) = self.backend.as_ref() {
            if self.batch_buffer.is_empty() {
                return;
            }

            let batch_size = self.batch_buffer.len();
            debug!("flushing {} metrics to backend", batch_size);

            let batch: Vec<MetricRow> = self.batch_buffer.drain(..).collect();

            match backend.insert_batch(batch).await {
                Ok(()) => {
                    self.flush_count += 1;
                    trace!(
                        "flush #{} complete ({} metrics)",
                        self.flush_count, batch_size
                    );
                }
                Err(e) => {
                    error!("failed to flush batch: {}", e);
                    // Metrics are lost - could implement retry logic here
                }
            }
        }
    }

    /// Run retention cleanup - delete metrics older than retention_days
    #[cfg(feature = "storage-sqlite")]
    async fn run_cleanup(&self) {
        if let (Some(backend), Some(retention_days)) = (self.backend.as_ref(), self.retention_days)
        {
            // Calculate cutoff date
            let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);

            debug!("running retention cleanup (deleting metrics before {})", cutoff);

            match backend.cleanup_old_metrics(cutoff).await {
                Ok(deleted_count) => {
                    if deleted_count > 0 {
                        info!("retention cleanup complete: deleted {} old metrics", deleted_count);
                    } else {
                        trace!("retention cleanup complete: no old metrics to delete");
                    }
                }
                Err(e) => {
                    error!("failed to run retention cleanup: {}", e);
                    // Don't crash the actor - cleanup will be retried on next interval
                }
            }
        }
    }

    /// Handle a command
    async fn handle_command(&mut self, cmd: StorageCommand) {
        match cmd {
            StorageCommand::Flush { respond_to } => {
                debug!("manual flush requested");

                #[cfg(feature = "storage-sqlite")]
                if self.backend.is_some() {
                    self.flush_batch().await;
                } else {
                    self.flush_count += 1;
                    trace!("flush #{} (no-op for in-memory storage)", self.flush_count);
                }

                #[cfg(not(feature = "storage-sqlite"))]
                {
                    self.flush_count += 1;
                    trace!("flush #{} (no-op for in-memory storage)", self.flush_count);
                }

                let _ = respond_to.send(Ok(()));
            }

            StorageCommand::GetStats { respond_to } => {
                let stats = self.get_stats().await;
                let _ = respond_to.send(stats);
            }

            #[cfg(feature = "storage-sqlite")]
            StorageCommand::QueryRange { query, respond_to } => {
                let result = match self.backend.as_ref() {
                    Some(backend) => backend.query_range(query).await.map_err(Into::into),
                    None => Err(anyhow::anyhow!(
                        "Query operations not available in in-memory mode"
                    )),
                };
                let _ = respond_to.send(result);
            }

            #[cfg(feature = "storage-sqlite")]
            StorageCommand::QueryLatest {
                server_id,
                limit,
                respond_to,
            } => {
                let result = match self.backend.as_ref() {
                    Some(backend) => backend
                        .query_latest(&server_id, limit)
                        .await
                        .map_err(Into::into),
                    None => Err(anyhow::anyhow!(
                        "Query operations not available in in-memory mode"
                    )),
                };
                let _ = respond_to.send(result);
            }

            #[cfg(feature = "storage-sqlite")]
            StorageCommand::HealthCheck { respond_to } => {
                let result = match self.backend.as_ref() {
                    Some(backend) => backend
                        .health_check()
                        .await
                        .map(|h| h.message)
                        .map_err(Into::into),
                    None => Ok("In-memory storage: operational".to_string()),
                };
                let _ = respond_to.send(result);
            }

            StorageCommand::Shutdown => {
                debug!("received shutdown command");
                // The loop will break and handle cleanup
            }
        }
    }

    /// Get storage statistics
    async fn get_stats(&self) -> StorageStats {
        #[cfg(feature = "storage-sqlite")]
        let total_metrics = if let Some(backend) = self.backend.as_ref() {
            // Try to get accurate count from backend
            backend
                .get_stats()
                .await
                .ok()
                .and_then(|s| s.split_whitespace().nth(1)?.parse().ok())
                .unwrap_or(self.memory_buffer.len())
        } else {
            self.memory_buffer.len()
        };

        #[cfg(not(feature = "storage-sqlite"))]
        let total_metrics = self.memory_buffer.len();

        StorageStats {
            total_metrics,
            buffer_size: self.memory_buffer.len(),
            flush_count: self.flush_count,
        }
    }
}

/// Handle for controlling the StorageActor
#[derive(Clone)]
pub struct StorageHandle {
    sender: mpsc::Sender<StorageCommand>,
}

impl StorageHandle {
    /// Spawn a new storage actor with optional backend
    #[cfg(feature = "storage-sqlite")]
    pub fn spawn_with_backend(
        metric_rx: broadcast::Receiver<MetricEvent>,
        backend: Option<Box<dyn StorageBackend>>,
        retention_days: Option<u32>,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        let actor = StorageActor::new(cmd_rx, metric_rx, backend, retention_days);

        tokio::spawn(actor.run());

        Self { sender: cmd_tx }
    }

    /// Spawn a new in-memory storage actor (backward compatible)
    pub fn spawn(metric_rx: broadcast::Receiver<MetricEvent>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        #[cfg(feature = "storage-sqlite")]
        let actor = StorageActor::new(cmd_rx, metric_rx, None, None);

        #[cfg(not(feature = "storage-sqlite"))]
        let actor = StorageActor::new(cmd_rx, metric_rx);

        tokio::spawn(actor.run());

        Self { sender: cmd_tx }
    }

    /// Manually flush the write buffer
    pub async fn flush(&self) -> anyhow::Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(StorageCommand::Flush { respond_to: tx })
            .await?;

        rx.await??;
        Ok(())
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> Option<StorageStats> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(StorageCommand::GetStats { respond_to: tx })
            .await
            .ok()?;

        rx.await.ok()
    }

    /// Query metrics within a time range (requires persistent backend)
    #[cfg(feature = "storage-sqlite")]
    pub async fn query_range(&self, query: QueryRange) -> anyhow::Result<Vec<MetricRow>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(StorageCommand::QueryRange {
                query,
                respond_to: tx,
            })
            .await?;

        rx.await?
    }

    /// Query the latest N metrics for a server (requires persistent backend)
    #[cfg(feature = "storage-sqlite")]
    pub async fn query_latest(
        &self,
        server_id: String,
        limit: usize,
    ) -> anyhow::Result<Vec<MetricRow>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(StorageCommand::QueryLatest {
                server_id,
                limit,
                respond_to: tx,
            })
            .await?;

        rx.await?
    }

    /// Check backend health
    #[cfg(feature = "storage-sqlite")]
    pub async fn health_check(&self) -> anyhow::Result<String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(StorageCommand::HealthCheck { respond_to: tx })
            .await?;

        rx.await?
    }

    /// Shutdown the storage actor
    pub async fn shutdown(&self) {
        let _ = self.sender.send(StorageCommand::Shutdown).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServerMetrics;
    use chrono::Utc;

    #[tokio::test]
    async fn test_storage_actor_basic() {
        let (metric_tx, metric_rx) = broadcast::channel(16);
        let handle = StorageHandle::spawn(metric_rx);

        // Send a metric
        let event = MetricEvent {
            server_id: "test:3000".to_string(),
            metrics: ServerMetrics::default(),
            timestamp: Utc::now(),
            display_name: "Test Server".to_string(),
        };

        metric_tx.send(event).unwrap();

        // Give it time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Check stats
        let stats = handle.get_stats().await.unwrap();
        assert_eq!(stats.total_metrics, 1);
        assert_eq!(stats.buffer_size, 1);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_storage_flush() {
        let (_metric_tx, metric_rx) = broadcast::channel(16);
        let handle = StorageHandle::spawn(metric_rx);

        // Flush should succeed even with empty buffer
        handle.flush().await.unwrap();

        let stats = handle.get_stats().await.unwrap();
        assert_eq!(stats.flush_count, 1);

        handle.shutdown().await;
    }
}
