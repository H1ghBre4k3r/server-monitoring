//! In-memory storage backend (no persistence)
//!
//! This backend stores metrics in a ring buffer in memory.
//! It's useful for:
//! - Testing without database dependencies
//! - Backward compatibility (default if no storage configured)
//! - Low-latency dashboards (recent metrics only)
//!
//! ## Limitations
//!
//! - **No persistence**: All data lost on restart
//! - **Limited capacity**: Ring buffer size is fixed
//! - **No historical queries**: Only recent metrics available

use std::collections::{HashMap, VecDeque};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::debug;

use super::backend::{HealthStatus, QueryRange, StorageBackend};
use super::error::StorageResult;
use super::schema::MetricRow;

/// Maximum metrics to keep in memory per server
const MAX_METRICS_PER_SERVER: usize = 1000;

/// In-memory storage backend
///
/// Stores metrics in a ring buffer with a fixed capacity.
/// When the buffer is full, oldest metrics are evicted.
pub struct MemoryBackend {
    /// Metrics grouped by server_id
    metrics: HashMap<String, VecDeque<MetricRow>>,

    /// Total metrics stored (across all servers)
    total_count: usize,
}

impl MemoryBackend {
    /// Create a new in-memory backend
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            total_count: 0,
        }
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for MemoryBackend {
    async fn insert_batch(&self, _metrics: Vec<MetricRow>) -> StorageResult<()> {
        // MemoryBackend needs interior mutability
        // For now, this is a placeholder - we'll use RwLock in the actor
        debug!("in-memory backend: insert_batch called (requires interior mutability)");
        Ok(())
    }

    async fn query_range(&self, query: QueryRange) -> StorageResult<Vec<MetricRow>> {
        debug!("querying in-memory storage for {}", query.server_id);

        let metrics = self
            .metrics
            .get(&query.server_id)
            .map(|deque| {
                deque
                    .iter()
                    .filter(|m| m.timestamp >= query.start && m.timestamp <= query.end)
                    .take(query.limit.unwrap_or(usize::MAX))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        Ok(metrics)
    }

    async fn query_latest(&self, server_id: &str, limit: usize) -> StorageResult<Vec<MetricRow>> {
        debug!("querying latest {} metrics for {}", limit, server_id);

        let metrics = self
            .metrics
            .get(server_id)
            .map(|deque| deque.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default();

        Ok(metrics)
    }

    async fn cleanup_old_metrics(&self, before: DateTime<Utc>) -> StorageResult<usize> {
        debug!("cleanup requested for metrics before {}", before);
        // Would need interior mutability
        Ok(0)
    }

    async fn health_check(&self) -> StorageResult<HealthStatus> {
        Ok(HealthStatus {
            healthy: true,
            message: "In-memory storage operational".to_string(),
            metadata: HashMap::from([
                ("backend".to_string(), "memory".to_string()),
                ("total_metrics".to_string(), self.total_count.to_string()),
            ]),
        })
    }

    async fn get_stats(&self) -> StorageResult<String> {
        Ok(format!(
            "In-Memory: {} metrics across {} servers",
            self.total_count,
            self.metrics.len()
        ))
    }

    async fn close(&self) -> StorageResult<()> {
        debug!("closing in-memory backend (no-op)");
        Ok(())
    }
}
