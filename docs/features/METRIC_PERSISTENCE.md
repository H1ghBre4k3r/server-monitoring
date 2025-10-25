# Metric Persistence

## Overview

This document describes the design and implementation of metric persistence for the guardia system. The goal is to store time-series metrics in a durable, queryable storage backend to enable historical analysis, dashboards, and trend detection.

---

## Requirements

### Functional Requirements

1. **Store all collected metrics** with timestamps
2. **Query metrics** by time range, server ID, and metric type
3. **Aggregate metrics** (min/max/avg) over time windows
4. **Retain metrics** according to configurable policies
5. **Handle high write throughput** (target: 10,000 metrics/sec)
6. **Support multiple backends** (SQLite, PostgreSQL, file-based)
7. **Graceful degradation** if storage fails (don't break monitoring)

### Non-Functional Requirements

1. **Performance:** <10ms write latency for batches
2. **Reliability:** No data loss during normal operations
3. **Scalability:** Support 1M+ metrics without degradation
4. **Maintainability:** Simple schema and clear query patterns
5. **Resource efficiency:** Minimal memory footprint, good compression

---

## Storage Backend Options

### Comparison Matrix

| Backend | Write Speed | Query Speed | Complexity | Ops Overhead | Compression | Use Case |
|---------|-------------|-------------|------------|--------------|-------------|----------|
| **SQLite** | Good | Good | Low | None | Moderate | Single-node, small-medium scale |
| **PostgreSQL** | Excellent | Excellent | Medium | Medium | Good | Production, multi-node |
| **Parquet** | Excellent | Variable | Low | Low | Excellent | Archival, cold storage |
| **InfluxDB** | Excellent | Excellent | High | High | Excellent | Time-series specialist |

**Recommendation:** Start with SQLite, provide PostgreSQL for production, add Parquet for archival.

---

## Backend Implementations

### 1. SQLite Backend (Default)

**Pros:**
- Zero configuration, embedded
- ACID transactions
- Good for 1-100 servers
- Easy backups (single file)
- Built-in aggregation via SQL

**Cons:**
- Single writer limitation
- Limited to single machine
- Max ~100k writes/sec

**Implementation:**

```rust
// Cargo.toml
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "chrono"] }
```

**Schema:**

```sql
CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,  -- Unix timestamp in milliseconds
    metric_type TEXT NOT NULL,   -- 'cpu_usage', 'temperature', 'memory_used', etc.
    value REAL NOT NULL,
    metadata TEXT,               -- JSON blob for additional context
    FOREIGN KEY (server_id) REFERENCES servers(id)
);

CREATE INDEX idx_metrics_server_time ON metrics(server_id, timestamp DESC);
CREATE INDEX idx_metrics_type_time ON metrics(metric_type, timestamp DESC);

CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY,         -- "192.168.1.100:3000"
    display_name TEXT,
    ip TEXT NOT NULL,
    port INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    last_seen INTEGER
);

CREATE TABLE IF NOT EXISTS aggregates (
    server_id TEXT NOT NULL,
    metric_type TEXT NOT NULL,
    window_start INTEGER NOT NULL,  -- Start of time window
    window_size INTEGER NOT NULL,   -- Window size in seconds (300, 3600, 86400)
    min_value REAL,
    max_value REAL,
    avg_value REAL,
    count INTEGER,
    PRIMARY KEY (server_id, metric_type, window_start, window_size)
);

CREATE INDEX idx_aggregates_lookup ON aggregates(server_id, metric_type, window_start);
```

**Code Example:**

```rust
// src/storage/sqlite.rs

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

pub struct SqliteBackend {
    pool: SqlitePool,
}

impl SqliteBackend {
    pub async fn new(db_path: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite://{}?mode=rwc", db_path))
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn write_batch(&self, metrics: Vec<MetricRecord>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for metric in metrics {
            sqlx::query(
                "INSERT INTO metrics (server_id, timestamp, metric_type, value, metadata)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(&metric.server_id)
            .bind(metric.timestamp.timestamp_millis())
            .bind(&metric.metric_type)
            .bind(metric.value)
            .bind(metric.metadata.map(|m| serde_json::to_string(&m).unwrap()))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn query_range(
        &self,
        server_id: &str,
        metric_type: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MetricRecord>> {
        let rows = sqlx::query(
            "SELECT timestamp, value, metadata
             FROM metrics
             WHERE server_id = ? AND metric_type = ?
               AND timestamp >= ? AND timestamp < ?
             ORDER BY timestamp ASC"
        )
        .bind(server_id)
        .bind(metric_type)
        .bind(start.timestamp_millis())
        .bind(end.timestamp_millis())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let timestamp_ms: i64 = row.get("timestamp");
                let metadata_str: Option<String> = row.get("metadata");
                Ok(MetricRecord {
                    server_id: server_id.to_string(),
                    timestamp: DateTime::from_timestamp_millis(timestamp_ms).unwrap(),
                    metric_type: metric_type.to_string(),
                    value: row.get("value"),
                    metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
                })
            })
            .collect()
    }

    pub async fn aggregate(
        &self,
        server_id: &str,
        metric_type: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<MetricAggregation> {
        let row = sqlx::query(
            "SELECT
                MIN(value) as min_val,
                MAX(value) as max_val,
                AVG(value) as avg_val,
                COUNT(*) as count
             FROM metrics
             WHERE server_id = ? AND metric_type = ?
               AND timestamp >= ? AND timestamp < ?"
        )
        .bind(server_id)
        .bind(metric_type)
        .bind(start.timestamp_millis())
        .bind(end.timestamp_millis())
        .fetch_one(&self.pool)
        .await?;

        Ok(MetricAggregation {
            min: row.get("min_val"),
            max: row.get("max_val"),
            avg: row.get("avg_val"),
            count: row.get("count"),
        })
    }
}
```

---

### 2. PostgreSQL Backend (Production)

**Pros:**
- Production-grade reliability
- High write throughput with tuning
- TimescaleDB extension for time-series optimization
- Horizontal scaling via replication
- Advanced SQL features

**Cons:**
- Requires separate server process
- More complex operations
- Higher resource usage

**Implementation:**

```rust
// Cargo.toml
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "chrono"] }
```

**TimescaleDB Setup:**

```sql
-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Create table (similar to SQLite)
CREATE TABLE metrics (
    server_id TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    metric_type TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    metadata JSONB
);

-- Convert to hypertable (TimescaleDB's partitioned table)
SELECT create_hypertable('metrics', 'timestamp', chunk_time_interval => INTERVAL '1 day');

-- Create indexes
CREATE INDEX idx_metrics_server_time ON metrics (server_id, timestamp DESC);
CREATE INDEX idx_metrics_type_time ON metrics (metric_type, timestamp DESC);

-- Enable compression
ALTER TABLE metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'server_id,metric_type',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Auto-compress chunks older than 7 days
SELECT add_compression_policy('metrics', INTERVAL '7 days');

-- Continuous aggregates (5-minute windows)
CREATE MATERIALIZED VIEW metrics_5min
WITH (timescaledb.continuous) AS
SELECT
    server_id,
    metric_type,
    time_bucket('5 minutes', timestamp) AS bucket,
    MIN(value) as min_value,
    MAX(value) as max_value,
    AVG(value) as avg_value,
    COUNT(*) as count
FROM metrics
GROUP BY server_id, metric_type, bucket;

-- Auto-refresh aggregate every 10 minutes
SELECT add_continuous_aggregate_policy('metrics_5min',
    start_offset => INTERVAL '1 hour',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '10 minutes');
```

**Code Example:**

```rust
// src/storage/postgres.rs

use sqlx::postgres::{PgPool, PgPoolOptions};

pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn write_batch(&self, metrics: Vec<MetricRecord>) -> Result<()> {
        // Use COPY for maximum write performance
        let mut writer = self.pool
            .begin()
            .await?
            .copy_in_raw("COPY metrics (server_id, timestamp, metric_type, value, metadata) FROM STDIN")
            .await?;

        for metric in metrics {
            let line = format!(
                "{}\t{}\t{}\t{}\t{}\n",
                metric.server_id,
                metric.timestamp.to_rfc3339(),
                metric.metric_type,
                metric.value,
                serde_json::to_string(&metric.metadata).unwrap_or_else(|_| "null".to_string())
            );
            writer.send(line.as_bytes()).await?;
        }

        writer.finish().await?;
        Ok(())
    }

    // Query methods similar to SQLite, but can leverage TimescaleDB functions
    pub async fn query_aggregated(
        &self,
        server_id: &str,
        metric_type: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        window: Duration,
    ) -> Result<Vec<AggregatedPoint>> {
        let interval = format!("{} seconds", window.as_secs());

        let rows = sqlx::query_as::<_, AggregatedPoint>(
            "SELECT
                time_bucket($1, timestamp) as bucket,
                MIN(value) as min_value,
                MAX(value) as max_value,
                AVG(value) as avg_value,
                COUNT(*) as count
             FROM metrics
             WHERE server_id = $2 AND metric_type = $3
               AND timestamp >= $4 AND timestamp < $5
             GROUP BY bucket
             ORDER BY bucket ASC"
        )
        .bind(interval)
        .bind(server_id)
        .bind(metric_type)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
```

---

### 3. Parquet File Backend (Archival)

**Pros:**
- Excellent compression (2-5x vs CSV)
- Columnar format optimized for analytics
- No server process needed
- Easy to backup/archive to S3/GCS
- Interoperable with data science tools

**Cons:**
- Immutable files (must rewrite for updates)
- Query performance varies with file size
- No ACID transactions
- Best for batch writes, not real-time

**Implementation:**

```rust
// Cargo.toml
parquet = "53"
arrow = "53"
```

**Strategy:**
- Write metrics to daily/hourly Parquet files: `metrics/2025/01/15/metrics-14.parquet`
- Each file contains one hour of data
- Close and finalize files on hour boundary
- Query by reading relevant files

**Schema:**

```rust
// src/storage/parquet.rs

use arrow::array::{Float64Array, Int64Array, StringArray, TimestampMillisecondArray};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::sync::Arc;

pub struct ParquetBackend {
    base_path: PathBuf,
    current_writer: Option<ArrowWriter<File>>,
    current_batch: Vec<MetricRecord>,
    batch_size: usize,
}

impl ParquetBackend {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            current_writer: None,
            current_batch: Vec::new(),
            batch_size: 10000,
        }
    }

    fn schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("server_id", DataType::Utf8, false),
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Millisecond, None),
                false,
            ),
            Field::new("metric_type", DataType::Utf8, false),
            Field::new("value", DataType::Float64, false),
            Field::new("metadata", DataType::Utf8, true),
        ]))
    }

    pub async fn write_batch(&mut self, metrics: Vec<MetricRecord>) -> Result<()> {
        self.current_batch.extend(metrics);

        if self.current_batch.len() >= self.batch_size {
            self.flush().await?;
        }

        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if self.current_batch.is_empty() {
            return Ok(());
        }

        let batch = self.create_record_batch()?;
        let writer = self.get_or_create_writer().await?;
        writer.write(&batch)?;

        self.current_batch.clear();
        Ok(())
    }

    fn create_record_batch(&self) -> Result<RecordBatch> {
        let server_ids: StringArray = self.current_batch
            .iter()
            .map(|m| Some(m.server_id.as_str()))
            .collect();

        let timestamps: TimestampMillisecondArray = self.current_batch
            .iter()
            .map(|m| Some(m.timestamp.timestamp_millis()))
            .collect();

        let metric_types: StringArray = self.current_batch
            .iter()
            .map(|m| Some(m.metric_type.as_str()))
            .collect();

        let values: Float64Array = self.current_batch
            .iter()
            .map(|m| Some(m.value))
            .collect();

        let metadata: StringArray = self.current_batch
            .iter()
            .map(|m| m.metadata.as_ref().map(|v| serde_json::to_string(v).unwrap()))
            .collect();

        RecordBatch::try_new(
            Self::schema(),
            vec![
                Arc::new(server_ids),
                Arc::new(timestamps),
                Arc::new(metric_types),
                Arc::new(values),
                Arc::new(metadata),
            ],
        )
        .map_err(Into::into)
    }

    async fn get_or_create_writer(&mut self) -> Result<&mut ArrowWriter<File>> {
        // Rotate file every hour
        let now = Utc::now();
        let file_path = self.base_path
            .join(format!("{}", now.format("%Y/%m/%d")))
            .join(format!("metrics-{:02}.parquet", now.hour()));

        if self.current_writer.is_none() {
            tokio::fs::create_dir_all(file_path.parent().unwrap()).await?;
            let file = File::create(&file_path)?;

            let props = WriterProperties::builder()
                .set_compression(parquet::basic::Compression::ZSTD(parquet::basic::ZstdLevel::default()))
                .set_dictionary_enabled(true)
                .build();

            let writer = ArrowWriter::try_new(file, Self::schema(), Some(props))?;
            self.current_writer = Some(writer);
        }

        Ok(self.current_writer.as_mut().unwrap())
    }

    pub async fn query_range(
        &self,
        server_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MetricRecord>> {
        // List all relevant parquet files in time range
        let files = self.find_files_in_range(start, end).await?;

        let mut results = Vec::new();
        for file_path in files {
            let file = File::open(file_path)?;
            let reader = ParquetRecordBatchReaderBuilder::try_new(file)?
                .with_batch_size(1024)
                .build()?;

            for batch_result in reader {
                let batch = batch_result?;
                // Filter records matching server_id and time range
                // Convert batch to MetricRecord structs
                // Add to results
            }
        }

        Ok(results)
    }
}
```

---

## Storage Trait Abstraction

**Common Interface:**

```rust
// src/storage/mod.rs

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRecord {
    pub server_id: String,
    pub timestamp: DateTime<Utc>,
    pub metric_type: String,  // "cpu_usage", "temperature", "memory_used", etc.
    pub value: f64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct MetricAggregation {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Write a batch of metrics
    async fn write_batch(&self, metrics: Vec<MetricRecord>) -> Result<()>;

    /// Query raw metrics in time range
    async fn query_range(
        &self,
        server_id: &str,
        metric_type: &str,
        range: TimeRange,
    ) -> Result<Vec<MetricRecord>>;

    /// Get aggregated statistics
    async fn aggregate(
        &self,
        server_id: &str,
        metric_type: &str,
        range: TimeRange,
    ) -> Result<MetricAggregation>;

    /// Delete metrics older than given time
    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64>;

    /// Get storage statistics
    async fn stats(&self) -> Result<StorageStats>;
}

#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_metrics: u64,
    pub disk_usage_bytes: u64,
    pub oldest_metric: Option<DateTime<Utc>>,
    pub newest_metric: Option<DateTime<Utc>>,
}
```

---

## Retention Policies

### Configuration

```json
{
  "storage": {
    "backend": "sqlite",
    "path": "/var/lib/guardia/metrics.db",
    "retention": {
      "raw_metrics": "7d",       // Keep raw metrics for 7 days
      "aggregated_5min": "30d",  // Keep 5-min aggregates for 30 days
      "aggregated_1hour": "1y"   // Keep hourly aggregates for 1 year
    },
    "pruning_interval": "1h"     // Run cleanup every hour
  }
}
```

### Implementation

```rust
// src/storage/retention.rs

pub struct RetentionPolicy {
    raw_retention: Duration,
    aggregate_5min_retention: Duration,
    aggregate_1hour_retention: Duration,
}

impl RetentionPolicy {
    pub async fn apply(&self, backend: &dyn StorageBackend) -> Result<u64> {
        let now = Utc::now();

        // Prune raw metrics
        let raw_cutoff = now - self.raw_retention;
        let pruned = backend.prune_before(raw_cutoff).await?;

        info!("Pruned {} raw metrics older than {}", pruned, raw_cutoff);

        Ok(pruned)
    }
}
```

---

## Integration with StorageActor

```rust
// src/actors/storage.rs

pub struct StorageActor {
    backend: Box<dyn StorageBackend>,
    metric_rx: broadcast::Receiver<MetricEvent>,
    command_rx: mpsc::Receiver<StorageCommand>,
    write_buffer: Vec<MetricRecord>,
    flush_interval: Duration,
}

pub enum StorageCommand {
    Query {
        server_id: String,
        metric_type: String,
        range: TimeRange,
        respond_to: oneshot::Sender<Result<Vec<MetricRecord>>>,
    },
    Stats {
        respond_to: oneshot::Sender<Result<StorageStats>>,
    },
    Flush,
    Shutdown,
}

impl StorageActor {
    pub async fn run(mut self) {
        let mut flush_timer = interval(self.flush_interval);

        loop {
            tokio::select! {
                // Receive metrics from broadcast
                Ok(event) = self.metric_rx.recv() => {
                    self.buffer_metric(event);
                }

                // Flush timer
                _ = flush_timer.tick() => {
                    if let Err(e) = self.flush().await {
                        error!("Failed to flush metrics: {}", e);
                    }
                }

                // Handle commands
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        StorageCommand::Query { server_id, metric_type, range, respond_to } => {
                            let result = self.backend.query_range(&server_id, &metric_type, range).await;
                            let _ = respond_to.send(result);
                        }
                        StorageCommand::Stats { respond_to } => {
                            let result = self.backend.stats().await;
                            let _ = respond_to.send(result);
                        }
                        StorageCommand::Flush => {
                            let _ = self.flush().await;
                        }
                        StorageCommand::Shutdown => break,
                    }
                }
            }
        }

        // Final flush before shutdown
        let _ = self.flush().await;
    }

    fn buffer_metric(&mut self, event: MetricEvent) {
        // Convert ServerMetrics to multiple MetricRecords
        let records = self.extract_metrics(event);
        self.write_buffer.extend(records);

        // Auto-flush if buffer is large
        if self.write_buffer.len() >= 1000 {
            tokio::spawn(async move {
                // Flush in background to avoid blocking
            });
        }
    }

    async fn flush(&mut self) -> Result<()> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }

        let batch = std::mem::take(&mut self.write_buffer);
        self.backend.write_batch(batch).await?;
        Ok(())
    }

    fn extract_metrics(&self, event: MetricEvent) -> Vec<MetricRecord> {
        let mut records = Vec::new();
        let metrics = event.metrics;

        // CPU average usage
        records.push(MetricRecord {
            server_id: event.server_id.clone(),
            timestamp: event.timestamp,
            metric_type: "cpu_usage_avg".to_string(),
            value: metrics.cpus.average_usage as f64,
            metadata: None,
        });

        // Temperature
        if let Some(temp) = metrics.components.average_temperature {
            records.push(MetricRecord {
                server_id: event.server_id.clone(),
                timestamp: event.timestamp,
                metric_type: "temperature_avg".to_string(),
                value: temp as f64,
                metadata: None,
            });
        }

        // Memory
        let memory_pct = (metrics.memory.used as f64 / metrics.memory.total as f64) * 100.0;
        records.push(MetricRecord {
            server_id: event.server_id.clone(),
            timestamp: event.timestamp,
            metric_type: "memory_used_pct".to_string(),
            value: memory_pct,
            metadata: Some(serde_json::json!({
                "total": metrics.memory.total,
                "used": metrics.memory.used
            })),
        });

        records
    }
}
```

---

## Configuration

```json
{
  "storage": {
    "backend": "sqlite",
    "sqlite": {
      "path": "./data/metrics.db"
    },
    "postgres": {
      "url": "postgres://user:pass@localhost/monitoring",
      "pool_size": 20
    },
    "parquet": {
      "base_path": "./data/parquet",
      "batch_size": 10000
    },
    "buffer_size": 1000,
    "flush_interval_secs": 30,
    "retention": {
      "raw_metrics_days": 7,
      "aggregated_5min_days": 30,
      "aggregated_1hour_days": 365
    }
  }
}
```

---

## Testing Strategy

1. **Unit Tests:** Test each backend in isolation
2. **Integration Tests:** Write → Query → Verify roundtrip
3. **Performance Tests:** Benchmark write/query speed
4. **Stress Tests:** Sustained high write rate for hours
5. **Corruption Tests:** Power-off simulation, verify recovery

---

## Migration Path

1. Start with SQLite (simple, works immediately)
2. Add PostgreSQL support for users who need it
3. Add Parquet for long-term archival
4. Eventually support hot/warm/cold storage tiers

---

## Success Metrics

- [ ] Store 1M metrics without degradation
- [ ] Query latency <100ms for typical dashboard queries
- [ ] Zero data loss during normal operations
- [ ] Retention policies execute correctly
- [ ] Storage stats are accurate
