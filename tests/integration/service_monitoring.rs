//! Integration tests for service monitoring
//!
//! These tests verify that:
//! - Service checks are performed correctly
//! - Events are published to broadcast channel
//! - Different HTTP methods work
//! - Status code validation works
//! - Body pattern matching works

use guardia::actors::messages::ServiceStatus;
use guardia::actors::service_monitor::ServiceHandle;
use guardia::config::{HttpMethod, ResolvedServiceConfig};
use tokio::sync::broadcast;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_service_check_success() {
    // Start mock HTTP server
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&mock_server)
        .await;

    // Create service configuration
    let config = ResolvedServiceConfig {
        name: "test-service".to_string(),
        url: format!("{}/health", mock_server.uri()),
        interval: 60,
        timeout: 10,
        method: HttpMethod::Get,
        expected_status: Some(vec![200]),
        body_pattern: None,
        grace: None,
        alert: None,
    };

    // Create broadcast channel and subscribe
    let (event_tx, mut event_rx) = broadcast::channel(16);

    // Spawn service monitor
    let handle = ServiceHandle::spawn(config, event_tx);

    // Trigger manual check
    handle.check_now().await.unwrap();

    // Give time for event to be published
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Receive and verify event
    let event = event_rx.recv().await.unwrap();
    assert_eq!(event.service_name, "test-service");
    assert_eq!(event.status, ServiceStatus::Up);
    assert_eq!(event.http_status_code, Some(200));
    assert!(event.response_time_ms.is_some());
    assert!(event.error_message.is_none());

    // Cleanup
    handle.shutdown().await;
}

#[tokio::test]
async fn test_service_check_failure() {
    // Start mock HTTP server that returns 500
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let config = ResolvedServiceConfig {
        name: "failing-service".to_string(),
        url: format!("{}/health", mock_server.uri()),
        interval: 60,
        timeout: 10,
        method: HttpMethod::Get,
        expected_status: Some(vec![200]),
        body_pattern: None,
        grace: None,
        alert: None,
    };

    let (event_tx, mut event_rx) = broadcast::channel(16);
    let handle = ServiceHandle::spawn(config, event_tx);

    handle.check_now().await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let event = event_rx.recv().await.unwrap();
    assert_eq!(event.service_name, "failing-service");
    assert_eq!(event.status, ServiceStatus::Down);
    assert_eq!(event.http_status_code, Some(500));

    handle.shutdown().await;
}

#[tokio::test]
async fn test_service_check_timeout() {
    // Start mock HTTP server with deliberate delay
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(3))
                .set_body_string("Slow response"),
        )
        .mount(&mock_server)
        .await;

    let config = ResolvedServiceConfig {
        name: "slow-service".to_string(),
        url: format!("{}/slow", mock_server.uri()),
        interval: 60,
        timeout: 1, // 1 second timeout
        method: HttpMethod::Get,
        expected_status: None,
        body_pattern: None,
        grace: None,
        alert: None,
    };

    let (event_tx, mut event_rx) = broadcast::channel(16);
    let handle = ServiceHandle::spawn(config, event_tx);

    handle.check_now().await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    let event = event_rx.recv().await.unwrap();
    assert_eq!(event.service_name, "slow-service");
    assert_eq!(event.status, ServiceStatus::Down);
    assert!(event.error_message.is_some());

    handle.shutdown().await;
}

#[tokio::test]
async fn test_service_check_body_pattern_match() {
    // Start mock HTTP server
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"status":"healthy","version":"1.0.0"}"#),
        )
        .mount(&mock_server)
        .await;

    let config = ResolvedServiceConfig {
        name: "api-service".to_string(),
        url: format!("{}/api", mock_server.uri()),
        interval: 60,
        timeout: 10,
        method: HttpMethod::Get,
        expected_status: Some(vec![200]),
        body_pattern: Some(r#""status":"healthy""#.to_string()),
        grace: None,
        alert: None,
    };

    let (event_tx, mut event_rx) = broadcast::channel(16);
    let handle = ServiceHandle::spawn(config, event_tx);

    handle.check_now().await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let event = event_rx.recv().await.unwrap();
    assert_eq!(event.service_name, "api-service");
    assert_eq!(event.status, ServiceStatus::Up);
    assert_eq!(event.http_status_code, Some(200));

    handle.shutdown().await;
}

#[tokio::test]
async fn test_service_check_body_pattern_mismatch() {
    // Start mock HTTP server
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"status":"degraded","version":"1.0.0"}"#),
        )
        .mount(&mock_server)
        .await;

    let config = ResolvedServiceConfig {
        name: "api-service-degraded".to_string(),
        url: format!("{}/api", mock_server.uri()),
        interval: 60,
        timeout: 10,
        method: HttpMethod::Get,
        expected_status: Some(vec![200]),
        body_pattern: Some(r#""status":"healthy""#.to_string()), // Expect "healthy" but get "degraded"
        grace: None,
        alert: None,
    };

    let (event_tx, mut event_rx) = broadcast::channel(16);
    let handle = ServiceHandle::spawn(config, event_tx);

    handle.check_now().await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let event = event_rx.recv().await.unwrap();
    assert_eq!(event.service_name, "api-service-degraded");
    assert_eq!(event.status, ServiceStatus::Degraded); // Should be degraded due to pattern mismatch
    assert_eq!(event.http_status_code, Some(200));

    handle.shutdown().await;
}

#[tokio::test]
async fn test_service_check_post_method() {
    // Start mock HTTP server
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(201).set_body_string("Created"))
        .mount(&mock_server)
        .await;

    let config = ResolvedServiceConfig {
        name: "webhook-service".to_string(),
        url: format!("{}/webhook", mock_server.uri()),
        interval: 60,
        timeout: 10,
        method: HttpMethod::Post,
        expected_status: Some(vec![200, 201]),
        body_pattern: None,
        grace: None,
        alert: None,
    };

    let (event_tx, mut event_rx) = broadcast::channel(16);
    let handle = ServiceHandle::spawn(config, event_tx);

    handle.check_now().await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let event = event_rx.recv().await.unwrap();
    assert_eq!(event.service_name, "webhook-service");
    assert_eq!(event.status, ServiceStatus::Up);
    assert_eq!(event.http_status_code, Some(201));

    handle.shutdown().await;
}
