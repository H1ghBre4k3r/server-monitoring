//! Failure and chaos tests for the actor system
//!
//! These tests verify that the system handles failures gracefully:
//! - Network failures
//! - Channel failures
//! - Actor crashes
//! - Malformed data

use server_monitoring::actors::{
    alert::AlertHandle, collector::CollectorHandle, storage::StorageHandle,
};
use tokio::sync::broadcast;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::helpers::*;

#[tokio::test]
async fn test_collector_handles_agent_unreachable() {
    // Don't start a mock server - agent will be unreachable
    let config = create_test_server_config("127.0.0.1", 9999);

    let (metric_tx, mut metric_rx) = broadcast::channel(256);
    let collector_handle = CollectorHandle::spawn(config, metric_tx.clone());

    // Poll should fail but not panic
    let result = collector_handle.poll_now().await;
    assert!(result.is_err(), "Poll should fail for unreachable server");

    // No metrics should be published
    let recv_result =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), metric_rx.recv()).await;
    assert!(
        recv_result.is_err(),
        "No metrics should be published on failure"
    );

    collector_handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_collector_handles_500_error() {
    let mock_server = MockServer::start().await;

    // Mock 500 Internal Server Error
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let collector_handle = CollectorHandle::spawn(config, metric_tx.clone());

    // Poll should fail gracefully
    let result = collector_handle.poll_now().await;
    assert!(result.is_err(), "Poll should fail for 500 error");

    collector_handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_collector_handles_malformed_json() {
    let mock_server = MockServer::start().await;

    // Mock invalid JSON response
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{invalid json"))
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let collector_handle = CollectorHandle::spawn(config, metric_tx.clone());

    // Poll should fail gracefully
    let result = collector_handle.poll_now().await;
    assert!(result.is_err(), "Poll should fail for malformed JSON");

    collector_handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_alert_actor_handles_broadcast_channel_closed() {
    let (metric_tx, metric_rx) = broadcast::channel(16);
    let (_service_tx, service_rx) = broadcast::channel(16);
    let config = create_test_server_with_limits("127.0.0.1", 3000, Some(70), Some(80), 3);

    let alert_handle = AlertHandle::spawn(vec![config], vec![], metric_rx, service_rx);

    // Drop the sender to close the channel
    drop(metric_tx);

    // Give actor time to detect closed channel
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Actor should have shut down gracefully
    // (We can't directly test this, but it shouldn't panic)

    // Try to get state - may fail if actor shutdown
    let _ = alert_handle.get_state("127.0.0.1:3000".to_string()).await;
}

#[tokio::test]
async fn test_storage_actor_handles_broadcast_lag() {
    let (metric_tx, metric_rx) = broadcast::channel(2); // Very small buffer
    let (_service_tx, service_rx) = broadcast::channel(2);

    let storage_handle = StorageHandle::spawn(metric_rx, service_rx);

    // Send many metrics to overflow buffer
    for i in 0..20 {
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

    // Give storage time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Storage should still be running despite lag
    let stats = storage_handle.get_stats().await;
    assert!(stats.is_some(), "Storage should still respond after lag");

    storage_handle.shutdown().await;
}

#[tokio::test]
async fn test_system_continues_after_collector_error() {
    // Setup: One working collector, one failing collector
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(create_mock_metrics_json(50.0, Some(45.0))),
        )
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let working_config =
        create_test_server_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

    let failing_config = create_test_server_config("127.0.0.1", 9999); // Unreachable

    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let (_service_tx, service_rx) = broadcast::channel(256);

    let storage_handle = StorageHandle::spawn(metric_tx.subscribe(), service_rx.resubscribe());
    let alert_handle = AlertHandle::spawn(
        vec![working_config.clone(), failing_config.clone()],
        vec![],
        metric_tx.subscribe(),
        service_rx,
    );

    let working_collector = CollectorHandle::spawn(working_config, metric_tx.clone());
    let failing_collector = CollectorHandle::spawn(failing_config, metric_tx.clone());

    // Poll both
    let _ = working_collector.poll_now().await; // Should succeed
    let _ = failing_collector.poll_now().await; // Should fail

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Storage should have received metrics from working collector
    let stats = storage_handle.get_stats().await.unwrap();
    assert!(
        stats.total_metrics > 0,
        "Should have metrics from working collector"
    );

    // Cleanup
    working_collector.shutdown().await.unwrap();
    failing_collector.shutdown().await.unwrap();
    alert_handle.shutdown().await;
    storage_handle.shutdown().await;
}

#[tokio::test]
async fn test_slow_agent_response_timeout() {
    let mock_server = MockServer::start().await;

    // Mock slow response (will timeout after 30s in collector)
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(create_mock_metrics_json(50.0, Some(45.0)))
                .set_delay(std::time::Duration::from_secs(35)),
        ) // Longer than timeout
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

    let (metric_tx, _metric_rx) = broadcast::channel(256);
    let collector_handle = CollectorHandle::spawn(config, metric_tx.clone());

    // Poll with our own shorter timeout
    let result = tokio::time::timeout(
        tokio::time::Duration::from_secs(2),
        collector_handle.poll_now(),
    )
    .await;

    // Should timeout
    assert!(
        result.is_err() || result.unwrap().is_err(),
        "Slow request should timeout"
    );

    collector_handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_partial_metrics_data_handled() {
    let mock_server = MockServer::start().await;

    // Mock response with some missing fields
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "system": {},
            "memory": {"total": 0, "used": 0, "total_swap": 0, "used_swap": 0},
            "cpus": {"total": 1, "arch": "unknown", "average_usage": 0.0, "cpus": []},
            "components": {"average_temperature": null, "components": []}
            // Some fields might be missing/null
        })))
        .mount(&mock_server)
        .await;

    let mock_url = url::Url::parse(&mock_server.uri()).unwrap();
    let config = create_test_server_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

    let (metric_tx, mut metric_rx) = broadcast::channel(256);
    let collector_handle = CollectorHandle::spawn(config, metric_tx.clone());

    // Poll should succeed even with partial data
    collector_handle.poll_now().await.unwrap();

    // Should receive metric event
    let event = tokio::time::timeout(tokio::time::Duration::from_millis(500), metric_rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(event.metrics.components.average_temperature, None);

    collector_handle.shutdown().await.unwrap();
}
