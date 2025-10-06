//! Integration tests for the full actor pipeline
//!
//! These tests verify that actors work correctly together:
//! - Collector → Alert → Storage
//! - Multiple collectors to single alert actor
//! - Graceful shutdown of entire system

use server_monitoring::actors::{
    alert::AlertHandle, collector::CollectorHandle, storage::StorageHandle,
};
use tokio::sync::broadcast;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::helpers::*;

#[tokio::test]
async fn test_metric_flows_from_collector_to_alert() {
    // Start mock agent
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(85.0, Some(75.0))),
        )
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_with_limits(
        mock_url.host_str().unwrap(),
        mock_url.port().unwrap(),
        Some(70), // Temp limit
        Some(80), // CPU limit
        2,        // Grace period
    );

    let server_id = format!("{}:{}", config.ip, config.port);

    // Create actor system
    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let (_service_tx, service_rx) = broadcast::channel(256);

    let alert_handle = AlertHandle::spawn(
        vec![config.clone()],
        vec![],
        metric_tx.subscribe(),
        service_rx,
    );
    let collector_handle = CollectorHandle::spawn(
        config,
        metric_tx.clone(),
        tokio::sync::broadcast::channel(16).0,
    );

    // Wait a moment for actor startup
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Trigger polls to exceed grace period
    for _ in 0..3 {
        collector_handle.poll_now().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }

    // Verify alert actor tracked the exceeded state
    let state = alert_handle.get_state(server_id.clone()).await.unwrap();
    assert!(
        state.temp_consecutive_exceeds >= 2,
        "Temperature should have exceeded grace"
    );
    assert!(
        state.cpu_consecutive_exceeds >= 2,
        "CPU should have exceeded grace"
    );

    // Cleanup
    collector_handle.shutdown().await.unwrap();
    alert_handle.shutdown().await;
}

#[tokio::test]
async fn test_metric_flows_from_collector_to_storage() {
    // Start mock agent
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

    // Create actor system
    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let (_service_tx, service_rx) = broadcast::channel(256);

    let storage_handle = StorageHandle::spawn(metric_tx.subscribe(), service_rx);
    let collector_handle = CollectorHandle::spawn(
        config,
        metric_tx.clone(),
        tokio::sync::broadcast::channel(16).0,
    );

    // Trigger a few polls
    for _ in 0..3 {
        collector_handle.poll_now().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }

    // Verify storage received metrics
    let stats = storage_handle.get_stats().await.unwrap();
    assert!(
        stats.total_metrics >= 3,
        "Storage should have received at least 3 metrics"
    );

    // Cleanup
    collector_handle.shutdown().await.unwrap();
    storage_handle.shutdown().await;
}

#[tokio::test]
async fn test_multiple_collectors_single_alert_actor() {
    // Start two mock agents
    let mock_server1 = MockServer::start().await;
    let mock_server2 = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(50.0, Some(65.0))),
        )
        .mount(&mock_server1)
        .await;

    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(90.0, Some(50.0))),
        )
        .mount(&mock_server2)
        .await;

    let mock_url1 = url::Url::parse(&mock_server1.uri()).unwrap();
    let mock_url2 = url::Url::parse(&mock_server2.uri()).unwrap();

    let config1 = create_test_server_with_limits(
        mock_url1.host_str().unwrap(),
        mock_url1.port().unwrap(),
        Some(70),
        Some(80),
        2,
    );

    let config2 = create_test_server_with_limits(
        mock_url2.host_str().unwrap(),
        mock_url2.port().unwrap(),
        Some(70),
        Some(80),
        2,
    );

    let server1_id = format!("{}:{}", config1.ip, config1.port);
    let server2_id = format!("{}:{}", config2.ip, config2.port);

    // Create actor system
    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let (_service_tx, service_rx) = broadcast::channel(256);

    let alert_handle = AlertHandle::spawn(
        vec![config1.clone(), config2.clone()],
        vec![],
        metric_tx.subscribe(),
        service_rx,
    );
    let collector1 = CollectorHandle::spawn(
        config1,
        metric_tx.clone(),
        tokio::sync::broadcast::channel(16).0,
    );
    let collector2 = CollectorHandle::spawn(
        config2,
        metric_tx.clone(),
        tokio::sync::broadcast::channel(16).0,
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Poll both servers
    for _ in 0..3 {
        collector1.poll_now().await.unwrap();
        collector2.poll_now().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }

    // Server 1 should be OK (below limits)
    let state1 = alert_handle.get_state(server1_id).await.unwrap();
    assert_eq!(
        state1.cpu_consecutive_exceeds, 0,
        "Server 1 CPU should be OK"
    );
    assert_eq!(
        state1.temp_consecutive_exceeds, 0,
        "Server 1 temp should be OK"
    );

    // Server 2 should have exceeded CPU limit
    let state2 = alert_handle.get_state(server2_id).await.unwrap();
    assert!(
        state2.cpu_consecutive_exceeds >= 2,
        "Server 2 CPU should have exceeded"
    );

    // Cleanup
    collector1.shutdown().await.unwrap();
    collector2.shutdown().await.unwrap();
    alert_handle.shutdown().await;
}

#[tokio::test]
async fn test_graceful_shutdown_all_actors() {
    // Start mock agent
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

    // Create full actor system
    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let (_service_tx, service_rx) = broadcast::channel(256);

    let storage_handle = StorageHandle::spawn(metric_tx.subscribe(), service_rx.resubscribe());
    let alert_handle = AlertHandle::spawn(
        vec![config.clone()],
        vec![],
        metric_tx.subscribe(),
        service_rx,
    );
    let collector_handle = CollectorHandle::spawn(
        config,
        metric_tx.clone(),
        tokio::sync::broadcast::channel(16).0,
    );

    // Let them run briefly
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Shutdown all actors gracefully
    let start = std::time::Instant::now();

    collector_handle.shutdown().await.unwrap();
    alert_handle.shutdown().await;
    storage_handle.shutdown().await;

    let shutdown_duration = start.elapsed();

    // Should shutdown quickly (< 1 second)
    assert!(
        shutdown_duration.as_millis() < 1000,
        "Shutdown took too long: {:?}",
        shutdown_duration
    );
}

#[tokio::test]
async fn test_alert_triggered_after_exact_grace_period() {
    // Start mock agent
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(85.0, Some(45.0))),
        )
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_with_limits(
        mock_url.host_str().unwrap(),
        mock_url.port().unwrap(),
        None,
        Some(80), // CPU limit
        3,        // Grace = 3
    );

    let server_id = format!("{}:{}", config.ip, config.port);

    // Create actor system
    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let (_service_tx, service_rx) = broadcast::channel(256);

    let alert_handle = AlertHandle::spawn(
        vec![config.clone()],
        vec![],
        metric_tx.subscribe(),
        service_rx,
    );
    let collector_handle = CollectorHandle::spawn(
        config,
        metric_tx.clone(),
        tokio::sync::broadcast::channel(16).0,
    );

    // Wait for actors to initialize and any initial auto-polling to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Get initial state (may have auto-polled once on startup)
    let initial_state = alert_handle.get_state(server_id.clone()).await.unwrap();
    let initial_count = initial_state.cpu_consecutive_exceeds;

    // Poll exactly grace_period times explicitly
    for i in 0..3 {
        collector_handle.poll_now().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        let state = alert_handle.get_state(server_id.clone()).await.unwrap();

        // Counter should increment from initial count
        let expected = initial_count + i + 1;
        assert_eq!(
            state.cpu_consecutive_exceeds, expected,
            "After poll {}, expected {}, got {}",
            i, expected, state.cpu_consecutive_exceeds
        );
    }

    // After 3 polls from initial state, should have reached grace limit
    let final_state = alert_handle.get_state(server_id.clone()).await.unwrap();
    assert!(
        final_state.cpu_consecutive_exceeds >= 3,
        "Final count should be at least 3, got {}",
        final_state.cpu_consecutive_exceeds
    );

    // Cleanup
    collector_handle.shutdown().await.unwrap();
    alert_handle.shutdown().await;
}

#[tokio::test]
async fn test_recovery_alert_when_back_to_ok() {
    // Start mock agent that can return different metrics
    let mock_server = MockServer::start().await;

    // First return high CPU
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(90.0, Some(45.0))),
        )
        .expect(4) // Expect exactly 4 calls (startup + 3 explicit)
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_with_limits(
        mock_url.host_str().unwrap(),
        mock_url.port().unwrap(),
        None,
        Some(80),
        2,
    );

    let server_id = format!("{}:{}", config.ip, config.port);

    // Create actor system
    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let (_service_tx, service_rx) = broadcast::channel(256);

    let alert_handle = AlertHandle::spawn(
        vec![config.clone()],
        vec![],
        metric_tx.subscribe(),
        service_rx,
    );
    let collector_handle = CollectorHandle::spawn(
        config,
        metric_tx.clone(),
        tokio::sync::broadcast::channel(16).0,
    );

    // Wait for initial auto-poll to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Exceed grace period with explicit polls
    for _ in 0..3 {
        collector_handle.poll_now().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let state = alert_handle.get_state(server_id.clone()).await.unwrap();
    assert!(
        state.cpu_consecutive_exceeds >= 2,
        "Should have exceeded grace period"
    );

    // Reset mock server and return low CPU (recovery)
    mock_server.reset().await;
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(50.0, Some(45.0))),
        )
        .mount(&mock_server)
        .await;

    // Wait a bit for mock to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Poll again with low CPU
    collector_handle.poll_now().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Should have reset counter (BackToOk)
    let state = alert_handle.get_state(server_id.clone()).await.unwrap();
    assert_eq!(
        state.cpu_consecutive_exceeds, 0,
        "Counter should reset to 0 after recovery"
    );

    // Cleanup
    collector_handle.shutdown().await.unwrap();
    alert_handle.shutdown().await;
}
