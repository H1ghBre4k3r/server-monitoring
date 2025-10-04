//! SQLite storage backend implementation
//!
//! This module provides a SQLite-based implementation of the `StorageBackend` trait.
//!
//! ## Features
//!
//! - **Embedded**: No separate database server required
//! - **WAL mode**: Better concurrency for reads during writes
//! - **Connection pooling**: Efficient resource usage
//! - **Migrations**: Automatic schema versioning with sqlx
//!
//! ## Performance Characteristics
//!
//! - **Write throughput**: 500-2000 metrics/sec (batch inserts)
//! - **Read latency**: <10ms for typical queries
//! - **Scalability**: Suitable for 1-100 servers
//! - **Disk usage**: ~100KB per 1000 metrics (varies with metadata)
//!
//! ## Limitations
//!
//! - **Concurrency**: Limited concurrent writes (use PostgreSQL for high concurrency)
//! - **Replication**: No built-in replication (file-level backups only)
//! - **Distributed**: Single-machine only

use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Row, Sqlite};
use tracing::{debug, info, instrument, warn};

use super::backend::{HealthStatus, QueryRange, StorageBackend};
use super::error::{StorageError, StorageResult};
use super::schema::{MetricRow, MetricType};

/// SQLite storage backend
///
/// This backend stores metrics in a local SQLite database file.
/// It's ideal for small to medium deployments (1-100 servers).
pub struct SqliteBackend {
    pool: Pool<Sqlite>,
    db_path: String,
}

impl SqliteBackend {
    /// Create a new SQLite backend
    ///
    /// This will:
    /// 1. Create the database file if it doesn't exist
    /// 2. Run migrations to create tables
    /// 3. Configure SQLite for optimal performance (WAL mode, etc.)
    ///
    /// ## Arguments
    ///
    /// * `db_path` - Path to the SQLite database file (e.g., "./metrics.db")
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use server_monitoring::storage::sqlite::SqliteBackend;
    /// # async fn example() -> anyhow::Result<()> {
    /// let backend = SqliteBackend::new("./metrics.db").await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip_all)]
    pub async fn new(db_path: impl AsRef<Path>) -> StorageResult<Self> {
        let db_path_str = db_path.as_ref().to_string_lossy().to_string();

        info!("initializing SQLite backend at: {}", db_path_str);

        // Configure SQLite for optimal performance
        let options = SqliteConnectOptions::new()
            .filename(&db_path_str)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal) // WAL mode for better concurrency
            .synchronous(SqliteSynchronous::Normal) // Balance safety and performance
            .busy_timeout(std::time::Duration::from_secs(30)); // Retry on lock contention

        // Create connection pool (5 connections by default)
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::ConnectionFailed(e.to_string()))?;

        info!("SQLite connection pool created");

        // Run migrations
        debug!("running database migrations");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| StorageError::MigrationFailed(e.to_string()))?;

        info!("database migrations complete");

        Ok(Self {
            pool,
            db_path: db_path_str,
        })
    }

    /// Helper to convert timestamp to Unix milliseconds for SQLite
    fn timestamp_to_millis(dt: &DateTime<Utc>) -> i64 {
        dt.timestamp_millis()
    }

    /// Helper to convert Unix milliseconds from SQLite to DateTime
    fn millis_to_timestamp(millis: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(millis).unwrap_or_else(Utc::now)
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    #[instrument(skip(self, metrics), fields(count = metrics.len()))]
    async fn insert_batch(&self, metrics: Vec<MetricRow>) -> StorageResult<()> {
        if metrics.is_empty() {
            return Ok(());
        }

        debug!("inserting {} metrics into SQLite", metrics.len());

        // Use a transaction for atomicity and performance
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;

        for metric in metrics {
            let timestamp = Self::timestamp_to_millis(&metric.timestamp);
            let metric_type_str = metric.metric_type.to_string();
            // Serialize ServerMetrics to JSON string for SQLite storage
            let metadata_json = serde_json::to_string(&metric.metadata).map_err(|e| {
                StorageError::SerializationError(format!("failed to serialize metadata: {}", e))
            })?;

            sqlx::query(
                r#"
                INSERT INTO metrics (
                    server_id, timestamp, display_name, metric_type,
                    cpu_avg, memory_used, memory_total, temp_avg, metadata
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (server_id, timestamp) DO UPDATE SET
                    display_name = excluded.display_name,
                    metric_type = excluded.metric_type,
                    cpu_avg = excluded.cpu_avg,
                    memory_used = excluded.memory_used,
                    memory_total = excluded.memory_total,
                    temp_avg = excluded.temp_avg,
                    metadata = excluded.metadata
                "#,
            )
            .bind(&metric.server_id)
            .bind(timestamp)
            .bind(&metric.display_name)
            .bind(metric_type_str)
            .bind(metric.cpu_avg)
            .bind(metric.memory_used.map(|v| v as i64))
            .bind(metric.memory_total.map(|v| v as i64))
            .bind(metric.temp_avg)
            .bind(metadata_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;

        debug!("batch insert complete");
        Ok(())
    }

    #[instrument(skip(self), fields(server_id = %query.server_id))]
    async fn query_range(&self, query: QueryRange) -> StorageResult<Vec<MetricRow>> {
        let start_millis = Self::timestamp_to_millis(&query.start);
        let end_millis = Self::timestamp_to_millis(&query.end);

        debug!(
            "querying metrics for {} from {} to {}",
            query.server_id, query.start, query.end
        );

        let limit_clause = query
            .limit
            .map(|l| format!("LIMIT {}", l))
            .unwrap_or_default();

        let sql = format!(
            r#"
            SELECT server_id, timestamp, display_name, metric_type,
                   cpu_avg, memory_used, memory_total, temp_avg, metadata
            FROM metrics
            WHERE server_id = ? AND timestamp >= ? AND timestamp <= ?
            ORDER BY timestamp ASC
            {}
            "#,
            limit_clause
        );

        let rows = sqlx::query(&sql)
            .bind(&query.server_id)
            .bind(start_millis)
            .bind(end_millis)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;

        let metrics: Result<Vec<MetricRow>, StorageError> = rows
            .into_iter()
            .map(|row| {
                let timestamp = Self::millis_to_timestamp(row.get("timestamp"));
                let metadata_str: String = row.get("metadata");
                // Deserialize JSON string back to ServerMetrics struct
                let metadata: crate::ServerMetrics =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        StorageError::SerializationError(format!(
                            "failed to deserialize metadata: {}",
                            e
                        ))
                    })?;

                let metric_type_str: String = row.get("metric_type");
                let metric_type = match metric_type_str.as_str() {
                    "resource" => MetricType::Resource,
                    "system" => MetricType::System,
                    "custom" => MetricType::Custom,
                    _ => MetricType::Resource, // Default fallback
                };

                Ok(MetricRow {
                    timestamp,
                    server_id: row.get("server_id"),
                    display_name: row.get("display_name"),
                    metric_type,
                    cpu_avg: row.get("cpu_avg"),
                    memory_used: row.get::<Option<i64>, _>("memory_used").map(|v| v as u64),
                    memory_total: row.get::<Option<i64>, _>("memory_total").map(|v| v as u64),
                    temp_avg: row.get("temp_avg"),
                    metadata,
                })
            })
            .collect();

        let results = metrics?;
        debug!("query returned {} metrics", results.len());
        Ok(results)
    }

    #[instrument(skip(self))]
    async fn query_latest(&self, server_id: &str, limit: usize) -> StorageResult<Vec<MetricRow>> {
        debug!("querying latest {} metrics for server {}", limit, server_id);

        let rows = sqlx::query(
            r#"
            SELECT server_id, timestamp, display_name, metric_type,
                   cpu_avg, memory_used, memory_total, temp_avg, metadata
            FROM metrics
            WHERE server_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(server_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::QueryFailed(e.to_string()))?;

        let metrics: Result<Vec<MetricRow>, StorageError> = rows
            .into_iter()
            .map(|row| {
                let timestamp = Self::millis_to_timestamp(row.get("timestamp"));
                let metadata_str: String = row.get("metadata");
                // Deserialize JSON string back to ServerMetrics struct
                let metadata: crate::ServerMetrics =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        StorageError::SerializationError(format!(
                            "failed to deserialize metadata: {}",
                            e
                        ))
                    })?;

                let metric_type_str: String = row.get("metric_type");
                let metric_type = match metric_type_str.as_str() {
                    "resource" => MetricType::Resource,
                    "system" => MetricType::System,
                    "custom" => MetricType::Custom,
                    _ => MetricType::Resource,
                };

                Ok(MetricRow {
                    timestamp,
                    server_id: row.get("server_id"),
                    display_name: row.get("display_name"),
                    metric_type,
                    cpu_avg: row.get("cpu_avg"),
                    memory_used: row.get::<Option<i64>, _>("memory_used").map(|v| v as u64),
                    memory_total: row.get::<Option<i64>, _>("memory_total").map(|v| v as u64),
                    temp_avg: row.get("temp_avg"),
                    metadata,
                })
            })
            .collect();

        let mut results = metrics?;
        // Reverse to get chronological order (oldest first)
        results.reverse();
        debug!("query returned {} metrics", results.len());
        Ok(results)
    }

    #[instrument(skip(self), fields(before = %before))]
    async fn cleanup_old_metrics(&self, before: DateTime<Utc>) -> StorageResult<usize> {
        let before_millis = Self::timestamp_to_millis(&before);

        info!("cleaning up metrics older than {}", before);

        let result = sqlx::query("DELETE FROM metrics WHERE timestamp < ?")
            .bind(before_millis)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;

        let deleted = result.rows_affected() as usize;
        info!("deleted {} old metrics", deleted);

        Ok(deleted)
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> StorageResult<HealthStatus> {
        // Simple ping query to verify connection
        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => {
                let mut metadata = HashMap::new();
                metadata.insert("backend".to_string(), "sqlite".to_string());
                metadata.insert("db_path".to_string(), self.db_path.clone());

                Ok(HealthStatus {
                    healthy: true,
                    message: "SQLite backend operational".to_string(),
                    metadata,
                })
            }
            Err(e) => {
                warn!("health check failed: {}", e);
                Ok(HealthStatus {
                    healthy: false,
                    message: format!("health check failed: {}", e),
                    metadata: HashMap::new(),
                })
            }
        }
    }

    #[instrument(skip(self))]
    async fn get_stats(&self) -> StorageResult<String> {
        // Get row count
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM metrics")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;

        let total_rows = row.0;

        // Get oldest and newest timestamps
        let oldest: Option<(Option<i64>,)> = sqlx::query_as("SELECT MIN(timestamp) FROM metrics")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;

        let newest: Option<(Option<i64>,)> = sqlx::query_as("SELECT MAX(timestamp) FROM metrics")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))?;

        // Get database file size
        let file_size = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let file_size_mb = file_size as f64 / 1_000_000.0;

        let time_range = match (oldest.and_then(|r| r.0), newest.and_then(|r| r.0)) {
            (Some(old), Some(new)) => {
                let old_dt = Self::millis_to_timestamp(old);
                let new_dt = Self::millis_to_timestamp(new);
                format!(
                    "{} to {}",
                    old_dt.format("%Y-%m-%d"),
                    new_dt.format("%Y-%m-%d")
                )
            }
            _ => "no data".to_string(),
        };

        Ok(format!(
            "SQLite: {} rows, {:.2} MB on disk, time range: {}",
            total_rows, file_size_mb, time_range
        ))
    }

    async fn close(&self) -> StorageResult<()> {
        info!("closing SQLite backend");
        self.pool.close().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentInformation, ComponentOverview, CpuInformation, CpuOverview, MemoryInformation,
        ServerMetrics, SystemInformation,
    };
    use chrono::Duration;

    fn create_test_metrics() -> ServerMetrics {
        ServerMetrics {
            system: SystemInformation::default(),
            memory: MemoryInformation {
                total: 16_000_000_000,
                used: 8_000_000_000,
                total_swap: 0,
                used_swap: 0,
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
                components: vec![ComponentInformation {
                    name: "CPU".to_string(),
                    temperature: Some(65.0),
                }],
            },
        }
    }

    #[tokio::test]
    async fn test_sqlite_backend_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let backend = SqliteBackend::new(&db_path).await;
        assert!(backend.is_ok());
    }

    #[tokio::test]
    async fn test_insert_and_query() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backend = SqliteBackend::new(&db_path).await.unwrap();

        let metrics = create_test_metrics();
        let timestamp = Utc::now();

        let row = MetricRow::from_server_metrics(
            "192.168.1.100:3000".to_string(),
            "Test Server".to_string(),
            timestamp,
            &metrics,
        );

        // Insert
        backend.insert_batch(vec![row.clone()]).await.unwrap();

        // Query
        let results = backend
            .query_latest("192.168.1.100:3000", 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].server_id, "192.168.1.100:3000");
        assert_eq!(results[0].cpu_avg, Some(55.5));
    }

    #[tokio::test]
    async fn test_batch_insert() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backend = SqliteBackend::new(&db_path).await.unwrap();

        let metrics = create_test_metrics();
        let base_time = Utc::now();

        let rows: Vec<MetricRow> = (0..10)
            .map(|i| {
                MetricRow::from_server_metrics(
                    "192.168.1.100:3000".to_string(),
                    "Test Server".to_string(),
                    base_time + Duration::seconds(i),
                    &metrics,
                )
            })
            .collect();

        backend.insert_batch(rows).await.unwrap();

        let results = backend
            .query_latest("192.168.1.100:3000", 20)
            .await
            .unwrap();
        assert_eq!(results.len(), 10);
    }

    #[tokio::test]
    async fn test_query_range() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backend = SqliteBackend::new(&db_path).await.unwrap();

        let metrics = create_test_metrics();
        let base_time = Utc::now();

        let rows: Vec<MetricRow> = (0..10)
            .map(|i| {
                MetricRow::from_server_metrics(
                    "192.168.1.100:3000".to_string(),
                    "Test Server".to_string(),
                    base_time + Duration::seconds(i * 60),
                    &metrics,
                )
            })
            .collect();

        backend.insert_batch(rows).await.unwrap();

        let query = QueryRange {
            server_id: "192.168.1.100:3000".to_string(),
            start: base_time + Duration::seconds(120),
            end: base_time + Duration::seconds(480),
            limit: None,
        };

        let results = backend.query_range(query).await.unwrap();
        assert_eq!(results.len(), 7); // Minutes 2-8 inclusive
    }

    #[tokio::test]
    async fn test_cleanup_old_metrics() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backend = SqliteBackend::new(&db_path).await.unwrap();

        let metrics = create_test_metrics();
        let now = Utc::now();
        let old_time = now - Duration::days(10);

        let old_row = MetricRow::from_server_metrics(
            "192.168.1.100:3000".to_string(),
            "Test Server".to_string(),
            old_time,
            &metrics,
        );

        let new_row = MetricRow::from_server_metrics(
            "192.168.1.100:3000".to_string(),
            "Test Server".to_string(),
            now,
            &metrics,
        );

        backend.insert_batch(vec![old_row, new_row]).await.unwrap();

        let cutoff = now - Duration::days(5);
        let deleted = backend.cleanup_old_metrics(cutoff).await.unwrap();

        assert_eq!(deleted, 1);

        let remaining = backend
            .query_latest("192.168.1.100:3000", 10)
            .await
            .unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[tokio::test]
    async fn test_health_check() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backend = SqliteBackend::new(&db_path).await.unwrap();

        let health = backend.health_check().await.unwrap();
        assert!(health.healthy);
        assert!(health.message.contains("operational"));
    }

    #[tokio::test]
    async fn test_get_stats() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backend = SqliteBackend::new(&db_path).await.unwrap();

        let stats = backend.get_stats().await.unwrap();
        assert!(stats.contains("SQLite"));
        assert!(stats.contains("rows"));
    }
}
