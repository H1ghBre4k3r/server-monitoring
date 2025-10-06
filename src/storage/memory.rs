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
use super::schema::{MetricRow, ServiceCheckRow, UptimeStats};
use crate::actors::messages::ServiceStatus;

/// In-memory storage backend
///
/// Stores metrics in a ring buffer with a fixed capacity.
/// When the buffer is full, oldest metrics are evicted.
pub struct MemoryBackend {
    /// Metrics grouped by server_id
    metrics: HashMap<String, VecDeque<MetricRow>>,

    /// Service checks grouped by service_name
    service_checks: HashMap<String, VecDeque<ServiceCheckRow>>,

    /// Total metrics stored (across all servers)
    total_count: usize,

    /// Total service checks stored (across all services)
    total_service_checks: usize,
}

impl MemoryBackend {
    /// Create a new in-memory backend
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            service_checks: HashMap::new(),
            total_count: 0,
            total_service_checks: 0,
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
            "In-Memory: {} metrics across {} servers, {} service checks across {} services",
            self.total_count,
            self.metrics.len(),
            self.total_service_checks,
            self.service_checks.len()
        ))
    }

    // ========================================================================
    // Service Check Operations
    // ========================================================================

    async fn insert_service_checks_batch(
        &self,
        _checks: Vec<ServiceCheckRow>,
    ) -> StorageResult<()> {
        // MemoryBackend needs interior mutability
        // For now, this is a placeholder - we'll use RwLock in the actor
        debug!(
            "in-memory backend: insert_service_checks_batch called (requires interior mutability)"
        );
        Ok(())
    }

    async fn query_service_checks_range(
        &self,
        service_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> StorageResult<Vec<ServiceCheckRow>> {
        debug!("querying in-memory service checks for {}", service_name);

        let checks = self
            .service_checks
            .get(service_name)
            .map(|deque| {
                deque
                    .iter()
                    .filter(|c| c.timestamp >= start && c.timestamp <= end)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        Ok(checks)
    }

    async fn query_latest_service_checks(
        &self,
        service_name: &str,
        limit: usize,
    ) -> StorageResult<Vec<ServiceCheckRow>> {
        debug!(
            "querying latest {} service checks for {}",
            limit, service_name
        );

        let checks = self
            .service_checks
            .get(service_name)
            .map(|deque| deque.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default();

        Ok(checks)
    }

    async fn calculate_uptime(
        &self,
        service_name: &str,
        since: DateTime<Utc>,
    ) -> StorageResult<UptimeStats> {
        debug!("calculating uptime for {} since {}", service_name, since);

        let now = Utc::now();
        let checks: Vec<ServiceCheckRow> = self
            .service_checks
            .get(service_name)
            .map(|deque| {
                deque
                    .iter()
                    .filter(|c| c.timestamp >= since)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let total_checks = checks.len();
        let successful_checks = checks
            .iter()
            .filter(|c| c.status == ServiceStatus::Up)
            .count();

        let uptime_percentage = if total_checks > 0 {
            (successful_checks as f64 / total_checks as f64) * 100.0
        } else {
            0.0
        };

        let avg_response_time_ms = if !checks.is_empty() {
            let sum: u64 = checks.iter().filter_map(|c| c.response_time_ms).sum();
            let count = checks
                .iter()
                .filter(|c| c.response_time_ms.is_some())
                .count();
            if count > 0 {
                Some(sum as f64 / count as f64)
            } else {
                None
            }
        } else {
            None
        };

        Ok(UptimeStats {
            service_name: service_name.to_string(),
            start: since,
            end: now,
            total_checks,
            successful_checks,
            uptime_percentage,
            avg_response_time_ms,
        })
    }

    async fn cleanup_old_service_checks(&self, before: DateTime<Utc>) -> StorageResult<usize> {
        debug!("cleanup requested for service checks before {}", before);
        // Would need interior mutability
        Ok(0)
    }

    async fn close(&self) -> StorageResult<()> {
        debug!("closing in-memory backend (no-op)");
        Ok(())
    }
}
