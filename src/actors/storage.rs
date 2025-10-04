//! StorageActor - Persists metrics to storage
//!
//! This is a stub implementation for Phase 1. It maintains an in-memory buffer
//! of recent metrics but doesn't persist to disk.
//!
//! ## Phase 2 Enhancement
//!
//! In Phase 2, this actor will be enhanced with:
//! - SQLite backend (default)
//! - PostgreSQL backend (production)
//! - Parquet file backend (archival)
//!
//! The actor interface remains the same - just swap the backend implementation.

use std::collections::VecDeque;

use tokio::sync::{broadcast, mpsc};
use tracing::{debug, instrument, trace, warn};

use super::messages::{MetricEvent, StorageCommand, StorageStats};

/// Maximum metrics to keep in memory (ring buffer)
///
/// At 1 metric/sec per server, 1000 metrics = ~16 minutes of history per server.
/// This is enough for testing and UI display, but Phase 2 will add real persistence.
const MAX_BUFFER_SIZE: usize = 1000;

/// In-memory storage actor (Phase 1 stub)
///
/// This actor subscribes to the metric broadcast channel and maintains a ring buffer
/// of recent metrics. It provides basic query capabilities for testing.
pub struct StorageActor {
    /// Metric buffer (ring buffer, oldest evicted when full)
    buffer: VecDeque<MetricEvent>,

    /// Command receiver
    command_rx: mpsc::Receiver<StorageCommand>,

    /// Metric event receiver (broadcast subscription)
    metric_rx: broadcast::Receiver<MetricEvent>,

    /// Flush counter (for stats)
    flush_count: u64,
}

impl StorageActor {
    /// Create a new storage actor
    pub fn new(
        command_rx: mpsc::Receiver<StorageCommand>,
        metric_rx: broadcast::Receiver<MetricEvent>,
    ) -> Self {
        Self {
            buffer: VecDeque::with_capacity(MAX_BUFFER_SIZE),
            command_rx,
            metric_rx,
            flush_count: 0,
        }
    }

    /// Run the actor's main loop
    #[instrument(skip(self))]
    pub async fn run(mut self) {
        debug!("starting storage actor (in-memory stub)");

        loop {
            tokio::select! {
                // Receive metric events
                result = self.metric_rx.recv() => {
                    match result {
                        Ok(event) => {
                            self.store_metric(event);
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
                    match cmd {
                        StorageCommand::Flush { respond_to } => {
                            debug!("manual flush requested");
                            self.flush();
                            let _ = respond_to.send(Ok(()));
                        }

                        StorageCommand::GetStats { respond_to } => {
                            let stats = self.get_stats();
                            let _ = respond_to.send(stats);
                        }

                        StorageCommand::Shutdown => {
                            debug!("received shutdown command");
                            break;
                        }
                    }
                }

                // Command channel closed
                else => {
                    warn!("command channel closed, shutting down");
                    break;
                }
            }
        }

        debug!("storage actor stopped");
    }

    /// Store a metric in the buffer
    fn store_metric(&mut self, event: MetricEvent) {
        trace!(
            "storing metric for {} at {}",
            event.server_id, event.timestamp
        );

        self.buffer.push_back(event);

        // Evict oldest if buffer is full
        if self.buffer.len() > MAX_BUFFER_SIZE {
            self.buffer.pop_front();
        }
    }

    /// Flush buffer to persistent storage
    ///
    /// In Phase 1, this is a no-op (already in memory).
    /// In Phase 2, this will write buffered metrics to SQLite/Postgres.
    fn flush(&mut self) {
        self.flush_count += 1;
        trace!("flush #{} (no-op for in-memory storage)", self.flush_count);
    }

    /// Get storage statistics
    fn get_stats(&self) -> StorageStats {
        StorageStats {
            total_metrics: self.buffer.len(),
            buffer_size: self.buffer.len(),
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
    /// Spawn a new storage actor
    pub fn spawn(metric_rx: broadcast::Receiver<MetricEvent>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

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
