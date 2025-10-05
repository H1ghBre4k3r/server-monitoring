//! Integration tests for storage persistence
//!
//! These tests verify that:
//! - Metrics are persisted to SQLite backend
//! - Batch writes work correctly
//! - Queries return correct data
//! - Retention cleanup removes old metrics

use chrono::{Duration, Utc};
use server_monitoring::ServerMetrics;
use server_monitoring::actors::messages::MetricEvent;
use server_monitoring::actors::storage::StorageHandle;
use server_monitoring::storage::StorageBackend;
use server_monitoring::storage::sqlite::SqliteBackend;
use tempfile::tempdir;
use tokio::sync::broadcast;

#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn test_full_persistence_pipeline() {
    // Create temp database
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_metrics.db");

    // Initialize backend
    let backend = SqliteBackend::new(&db_path).await.unwrap();

    // Create broadcast channels
    let (metric_tx, _) = broadcast::channel(256);
    let (_service_tx, service_rx) = broadcast::channel(256);

    // Spawn storage actor with backend
    let storage_handle = StorageHandle::spawn_with_backend(
        metric_tx.subscribe(),
        service_rx,
        Some(Box::new(backend) as Box<dyn StorageBackend>),
        Some(30), // 30 days retention
    );

    // Give actor time to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Create test metrics with clear time differences
    let server_id = "test-server:3000".to_string();
    let base_time = Utc::now();

    let event1 = MetricEvent {
        server_id: server_id.clone(),
        display_name: "Test Server".to_string(),
        timestamp: base_time,
        metrics: ServerMetrics::default(),
    };

    let event2 = MetricEvent {
        server_id: server_id.clone(),
        display_name: "Test Server".to_string(),
        timestamp: base_time + Duration::seconds(60), // 60 seconds later
        metrics: ServerMetrics::default(),
    };

    // Send metrics via broadcast
    metric_tx.send(event1).unwrap();
    metric_tx.send(event2).unwrap();

    // Give time for actor to process metrics
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Manual flush to persist immediately
    storage_handle.flush().await.unwrap();

    // Give time for flush to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Verify stats show metrics were stored
    let stats = storage_handle.get_stats().await.unwrap();
    // In-memory buffer should have 2 metrics
    assert_eq!(
        stats.buffer_size, 2,
        "Should have 2 metrics in memory buffer"
    );
    assert!(stats.flush_count >= 1, "Should have flushed at least once");

    // Query latest metrics through the actor
    let latest = storage_handle
        .query_latest(server_id.clone(), 10)
        .await
        .unwrap();
    assert_eq!(latest.len(), 2, "Should retrieve 2 metrics from database");

    // Verify both metrics are present (with tolerance for timestamp precision)
    // SQLite stores timestamps with millisecond precision, so we check within 1 second tolerance
    let has_first_metric = latest
        .iter()
        .any(|m| (m.timestamp - base_time).num_milliseconds().abs() < 1000);
    let has_second_metric = latest.iter().any(|m| {
        (m.timestamp - (base_time + Duration::seconds(60)))
            .num_milliseconds()
            .abs()
            < 1000
    });

    assert!(has_first_metric, "Should contain first metric");
    assert!(has_second_metric, "Should contain second metric");

    // Cleanup
    storage_handle.shutdown().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn test_retention_cleanup() {
    // Create temp database
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_retention.db");

    // Initialize backend
    let backend = SqliteBackend::new(&db_path).await.unwrap();

    // Insert old metrics (older than retention period)
    let server_id = "retention-test:3000".to_string();
    let old_timestamp = Utc::now() - Duration::days(35); // 35 days old
    let recent_timestamp = Utc::now() - Duration::hours(1); // 1 hour old

    let old_metrics = vec![
        server_monitoring::storage::schema::MetricRow::from_server_metrics(
            server_id.clone(),
            "Retention Test".to_string(),
            old_timestamp,
            &ServerMetrics::default(),
        ),
    ];

    let recent_metrics = vec![
        server_monitoring::storage::schema::MetricRow::from_server_metrics(
            server_id.clone(),
            "Retention Test".to_string(),
            recent_timestamp,
            &ServerMetrics::default(),
        ),
    ];

    // Insert both old and recent metrics
    backend.insert_batch(old_metrics).await.unwrap();
    backend.insert_batch(recent_metrics).await.unwrap();

    // Verify both are present
    let all_metrics = backend.query_latest(&server_id, 10).await.unwrap();
    assert_eq!(all_metrics.len(), 2, "Should have 2 metrics before cleanup");

    // Run cleanup (30 days retention)
    let cutoff = Utc::now() - Duration::days(30);
    let deleted_count = backend.cleanup_old_metrics(cutoff).await.unwrap();

    assert_eq!(deleted_count, 1, "Should delete 1 old metric");

    // Verify only recent metric remains
    let remaining_metrics = backend.query_latest(&server_id, 10).await.unwrap();
    assert_eq!(
        remaining_metrics.len(),
        1,
        "Should have 1 metric after cleanup"
    );
    assert!(
        remaining_metrics[0].timestamp > cutoff,
        "Remaining metric should be after cutoff"
    );

    // Cleanup backend
    backend.close().await.unwrap();
}

#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn test_batch_write_performance() {
    // Create temp database
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_batch.db");

    // Initialize backend
    let backend = SqliteBackend::new(&db_path).await.unwrap();

    // Create broadcast channels
    let (metric_tx, _) = broadcast::channel(1024);
    let (_service_tx, service_rx) = broadcast::channel(1024);

    // Spawn storage actor with backend
    let storage_handle = StorageHandle::spawn_with_backend(
        metric_tx.subscribe(),
        service_rx,
        Some(Box::new(backend) as Box<dyn StorageBackend>),
        None, // No retention cleanup for this test
    );

    let server_id = "batch-test:3000".to_string();

    // Send 150 metrics (triggers batch flush at 100)
    for i in 0..150 {
        let event = MetricEvent {
            server_id: server_id.clone(),
            display_name: "Batch Test".to_string(),
            timestamp: Utc::now() + Duration::seconds(i),
            metrics: ServerMetrics::default(),
        };
        metric_tx.send(event).unwrap();
    }

    // Wait for automatic batch flush (triggered at 100 metrics)
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Manual flush for remaining 50 metrics
    storage_handle.flush().await.unwrap();

    // Wait for flush to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify all metrics were persisted
    let stats = storage_handle.get_stats().await.unwrap();
    assert_eq!(
        stats.total_metrics, 150,
        "Should have 150 metrics in storage"
    );
    assert!(
        stats.flush_count >= 2,
        "Should have flushed at least twice (auto + manual)"
    );

    // Cleanup
    storage_handle.shutdown().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
}

#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn test_query_range() {
    // Create temp database
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_query_range.db");

    // Initialize backend
    let backend = SqliteBackend::new(&db_path).await.unwrap();

    let server_id = "range-test:3000".to_string();
    // Use timestamp with second precision to avoid SQLite precision issues
    let base_time = Utc::now()
        .date_naive()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();

    // Insert metrics at different times (every hour)
    let mut batch = Vec::new();
    for i in 0..10 {
        let metric = server_monitoring::storage::schema::MetricRow::from_server_metrics(
            server_id.clone(),
            "Range Test".to_string(),
            base_time + Duration::hours(i),
            &ServerMetrics::default(),
        );
        batch.push(metric);
    }

    backend.insert_batch(batch).await.unwrap();

    // Query a specific time range (hours 2-6, should get hours 2,3,4,5,6)
    let start = base_time + Duration::hours(2);
    let end = base_time + Duration::hours(6);

    let query = server_monitoring::storage::backend::QueryRange {
        server_id: server_id.clone(),
        start,
        end,
        limit: None,
    };

    let results = backend.query_range(query).await.unwrap();

    // Should get metrics for hours 2, 3, 4, 5, 6 (5 metrics)
    assert_eq!(results.len(), 5, "Should return 5 metrics in range");

    // Verify all results are within range (with 1 second tolerance for SQLite precision)
    for (i, metric) in results.iter().enumerate() {
        let within_range = metric.timestamp >= start - Duration::seconds(1)
            && metric.timestamp <= end + Duration::seconds(1);
        assert!(
            within_range,
            "Metric {}: timestamp {:?} should be within range [{:?}, {:?}]",
            i, metric.timestamp, start, end
        );
    }

    // Cleanup backend
    backend.close().await.unwrap();
}
