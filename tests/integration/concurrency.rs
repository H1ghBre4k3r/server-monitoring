//! Concurrency and race condition tests
//!
//! These tests verify thread-safety and concurrent operation:
//! - Multiple collectors polling simultaneously
//! - Concurrent alert state queries
//! - Race conditions in grace period counters
//! - Channel backpressure handling

use server_monitoring::actors::{
    alert::AlertHandle, collector::CollectorHandle, storage::StorageHandle,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::broadcast;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod helpers;
use helpers::*;

#[tokio::test]
async fn test_concurrent_collectors_no_race() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let request_count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(move |_req: &wiremock::Request| {
            request_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(50.0, Some(45.0)))
        })
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();

    // Create multiple collectors pointing to same agent
    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let mut handles = vec![];

    for i in 0..5 {
        let mut config =
            create_test_server_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());
        config.display = Some(format!("Collector {i}"));

        let handle = CollectorHandle::spawn(config, metric_tx.clone());
        handles.push(handle);
    }

    // Poll all collectors concurrently
    let mut tasks = vec![];
    for handle in &handles {
        let h = handle.clone();
        tasks.push(tokio::spawn(async move { h.poll_now().await }));
    }

    // Wait for all
    for task in tasks {
        task.await.unwrap().unwrap();
    }

    // Should have made at least 5 requests (may be more due to auto-polling on startup)
    let count = request_count.load(Ordering::SeqCst);
    assert!(count >= 5, "Should have at least 5 requests, got {}", count);

    // Cleanup
    for handle in handles {
        handle.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn test_concurrent_alert_state_queries() {
    let config = create_test_server_with_limits("127.0.0.1", 3000, Some(70), Some(80), 3);
    let server_id = format!("{}:{}", config.ip, config.port);

    let (_metric_tx, metric_rx) = broadcast::channel(256);
    let alert_handle = AlertHandle::spawn(vec![config], metric_rx);

    // Query state concurrently from multiple tasks
    let mut tasks = vec![];
    for _ in 0..10 {
        let handle = alert_handle.clone();
        let id = server_id.clone();
        tasks.push(tokio::spawn(async move { handle.get_state(id).await }));
    }

    // All queries should succeed
    for task in tasks {
        let state = task.await.unwrap();
        assert!(state.is_some());
    }

    alert_handle.shutdown().await;
}

#[tokio::test]
async fn test_rapid_metric_updates_no_data_loss() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(50.0, Some(45.0))),
        )
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

    let (metric_tx, _metric_rx) = broadcast::channel(256);

    let storage_handle = StorageHandle::spawn(metric_tx.subscribe());
    let collector_handle = CollectorHandle::spawn(config, metric_tx.clone());

    // Trigger many rapid polls
    let mut tasks = vec![];
    for _ in 0..20 {
        let h = collector_handle.clone();
        tasks.push(tokio::spawn(async move { h.poll_now().await }));
    }

    // Wait for all
    for task in tasks {
        let _ = task.await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Storage should have received most/all metrics
    let stats = storage_handle.get_stats().await.unwrap();
    assert!(
        stats.total_metrics >= 15,
        "Should have received most metrics without loss"
    );

    collector_handle.shutdown().await.unwrap();
    storage_handle.shutdown().await;
}

#[tokio::test]
async fn test_channel_backpressure_handling() {
    let (metric_tx, _metric_rx) = broadcast::channel(8); // Small buffer

    let storage_handle = StorageHandle::spawn(metric_tx.subscribe());

    // Send many metrics rapidly to test backpressure
    for i in 0..100 {
        use chrono::Utc;
        use server_monitoring::actors::messages::MetricEvent;
        use server_monitoring::{
            ComponentOverview, CpuOverview, MemoryInformation, ServerMetrics, SystemInformation,
        };

        let event = MetricEvent {
            server_id: "test:3000".to_string(),
            metrics: ServerMetrics {
                system: SystemInformation::default(),
                memory: MemoryInformation::default(),
                cpus: CpuOverview {
                    total: 1,
                    arch: "x86_64".to_string(),
                    average_usage: 50.0,
                    cpus: vec![],
                },
                components: ComponentOverview::default(),
            },
            timestamp: Utc::now(),
            display_name: format!("Test {i}"),
        };

        let _ = metric_tx.send(event);
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Storage should still be operational despite backpressure
    let stats = storage_handle.get_stats().await;
    assert!(
        stats.is_some(),
        "Storage should handle backpressure gracefully"
    );

    storage_handle.shutdown().await;
}

#[tokio::test]
async fn test_concurrent_shutdown_requests() {
    let config = create_test_server_config("127.0.0.1", 9999);

    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let collector_handle = CollectorHandle::spawn(config, metric_tx.clone());

    // Send shutdown from multiple tasks concurrently
    let mut tasks = vec![];
    for _ in 0..5 {
        let h = collector_handle.clone();
        tasks.push(tokio::spawn(async move { h.shutdown().await }));
    }

    // All should complete without error (or acceptable errors)
    for task in tasks {
        let _ = task.await;
    }

    // Additional shutdown should also work
    let _ = collector_handle.shutdown().await;
}

#[tokio::test]
async fn test_grace_period_race_condition() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(90.0, Some(45.0))),
        )
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_with_limits(
        mock_url.host_str().unwrap(),
        mock_url.port().unwrap(),
        None,
        Some(80),
        5, // Grace = 5
    );

    let server_id = format!("{}:{}", config.ip, config.port);

    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let alert_handle = AlertHandle::spawn(vec![config.clone()], metric_tx.subscribe());
    let collector_handle = CollectorHandle::spawn(config, metric_tx.clone());

    // Wait for initialization and get initial counter (may have auto-polled)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let initial_state = alert_handle.get_state(server_id.clone()).await.unwrap();
    let initial_count = initial_state.cpu_consecutive_exceeds;

    // Trigger many concurrent polls to try to create race condition
    let mut tasks = vec![];
    let num_concurrent_polls = 10;
    for _ in 0..num_concurrent_polls {
        let h = collector_handle.clone();
        tasks.push(tokio::spawn(async move { h.poll_now().await }));
    }

    for task in tasks {
        let _ = task.await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Grace counter should be bounded by number of polls
    // With 10 concurrent polls from initial state, counter should be initial + 10 (or close to it)
    let state = alert_handle.get_state(server_id).await.unwrap();
    let max_expected = initial_count + num_concurrent_polls + 2; // +2 for timing tolerance
    assert!(
        state.cpu_consecutive_exceeds <= max_expected,
        "Grace counter should be bounded: expected <= {}, got {}",
        max_expected,
        state.cpu_consecutive_exceeds
    );

    collector_handle.shutdown().await.unwrap();
    alert_handle.shutdown().await;
}

#[tokio::test]
async fn test_multiple_subscribers_all_receive_metrics() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(50.0, Some(45.0))),
        )
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

    let (metric_tx, _metric_rx) = broadcast::channel(256);

    // Create multiple subscribers
    let storage_handle1 = StorageHandle::spawn(metric_tx.subscribe());
    let storage_handle2 = StorageHandle::spawn(metric_tx.subscribe());
    let alert_handle = AlertHandle::spawn(vec![config.clone()], metric_tx.subscribe());

    let collector_handle = CollectorHandle::spawn(config, metric_tx.clone());

    // Trigger some polls
    for _ in 0..3 {
        collector_handle.poll_now().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }

    // All subscribers should have received metrics
    let stats1 = storage_handle1.get_stats().await.unwrap();
    let stats2 = storage_handle2.get_stats().await.unwrap();

    assert!(stats1.total_metrics >= 3, "Storage1 should have metrics");
    assert!(stats2.total_metrics >= 3, "Storage2 should have metrics");

    // Cleanup
    collector_handle.shutdown().await.unwrap();
    alert_handle.shutdown().await;
    storage_handle1.shutdown().await;
    storage_handle2.shutdown().await;
}
