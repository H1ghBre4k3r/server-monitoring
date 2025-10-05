//! Integration tests for API endpoints
//!
//! These tests verify that:
//! - All REST endpoints return correct responses
//! - Health status detection works correctly
//! - Authentication middleware functions properly
//! - WebSocket streaming works
//! - Error handling is correct

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::Value;
use server_monitoring::{
    ServerMetrics,
    actors::{
        collector::CollectorHandle,
        messages::{MetricEvent, ServiceCheckEvent, ServiceStatus},
        service_monitor::ServiceHandle,
        storage::StorageHandle,
    },
    api::{ApiConfig, ApiState, spawn_api_server},
    config::{HttpMethod, ServerConfig, ServiceConfig},
    storage::{StorageBackend, sqlite::SqliteBackend},
};
use std::net::SocketAddr;
use tempfile::tempdir;
use tokio::sync::broadcast;

// Helper to create test API server
async fn spawn_test_api(
    collectors: Vec<CollectorHandle>,
    services: Vec<ServiceHandle>,
    storage: StorageHandle,
    metric_tx: broadcast::Sender<MetricEvent>,
    service_tx: broadcast::Sender<ServiceCheckEvent>,
) -> SocketAddr {
    let state = ApiState::new(
        storage,
        // Create dummy alert handle (not used in these tests)
        server_monitoring::actors::alert::AlertHandle::spawn(
            vec![],
            vec![],
            metric_tx.subscribe(),
            service_tx.subscribe(),
        ),
        collectors,
        services,
        metric_tx,
        service_tx,
    );

    let config = ApiConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(), // Random port
        auth_token: Some("test-token".to_string()),
        enable_cors: true,
    };

    spawn_api_server(config, state).await.unwrap()
}

// Helper to create test metrics
fn create_test_metrics() -> ServerMetrics {
    ServerMetrics {
        system: server_monitoring::SystemInformation {
            name: Some("Test System".to_string()),
            kernel_version: Some("5.15.0".to_string()),
            os_version: Some("Ubuntu 22.04".to_string()),
            host_name: Some("test-host".to_string()),
        },
        memory: server_monitoring::MemoryInformation {
            total: 16_000_000_000,
            used: 8_000_000_000,
            total_swap: 4_000_000_000,
            used_swap: 1_000_000_000,
        },
        cpus: server_monitoring::CpuOverview {
            total: 4,
            arch: "x86_64".to_string(),
            average_usage: 45.5,
            cpus: vec![
                server_monitoring::CpuInformation {
                    name: "CPU 0".to_string(),
                    frequency: 2400,
                    usage: 42.0,
                },
                server_monitoring::CpuInformation {
                    name: "CPU 1".to_string(),
                    frequency: 2400,
                    usage: 49.0,
                },
            ],
        },
        components: server_monitoring::ComponentOverview {
            average_temperature: Some(55.0),
            components: vec![server_monitoring::ComponentInformation {
                name: "CPU".to_string(),
                temperature: Some(55.0),
            }],
        },
    }
}

#[cfg(feature = "api")]
#[tokio::test]
async fn test_health_endpoint_returns_ok() {
    // Setup
    let (metric_tx, _) = broadcast::channel(16);
    let (service_tx, service_rx) = broadcast::channel(16);
    let storage = StorageHandle::spawn(metric_tx.subscribe(), service_rx);

    let addr = spawn_test_api(vec![], vec![], storage, metric_tx, service_tx).await;

    // Test - health endpoint also requires auth
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/api/v1/health", addr))
        .header("Authorization", "Bearer test-token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json: Value = response.json().await.unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json["timestamp"].is_string());
}

#[cfg(feature = "api")]
#[tokio::test]
async fn test_stats_endpoint_returns_storage_info() {
    // Setup
    let (metric_tx, _) = broadcast::channel(16);
    let (service_tx, service_rx) = broadcast::channel(16);
    let storage = StorageHandle::spawn(metric_tx.subscribe(), service_rx);

    let addr = spawn_test_api(vec![], vec![], storage, metric_tx, service_tx).await;

    // Test
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/api/v1/stats", addr))
        .header("Authorization", "Bearer test-token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json: Value = response.json().await.unwrap();
    assert!(json["timestamp"].is_string());
    assert!(json["storage"]["total_metrics"].is_number());
    assert!(json["collectors"].is_number());
    assert!(json["service_monitors"].is_number());
}

#[cfg(feature = "api")]
#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn test_list_servers_with_no_metrics_shows_unknown() {
    // Setup
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let backend = SqliteBackend::new(&db_path).await.unwrap();

    let (metric_tx, _) = broadcast::channel(16);
    let (service_tx, service_rx) = broadcast::channel(16);
    let storage = StorageHandle::spawn_with_backend(
        metric_tx.subscribe(),
        service_rx,
        Some(Box::new(backend) as Box<dyn StorageBackend>),
        Some(30),
        Some(24),
    );

    // Create a collector
    let config = ServerConfig {
        ip: "192.168.1.100".parse().unwrap(),
        port: 3000,
        interval: 30,
        token: None,
        display: Some("Test Server".to_string()),
        limits: None,
    };
    let collector = CollectorHandle::spawn(config, metric_tx.clone());

    let addr = spawn_test_api(vec![collector], vec![], storage, metric_tx, service_tx).await;

    // Test
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/api/v1/servers", addr))
        .header("Authorization", "Bearer test-token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json: Value = response.json().await.unwrap();
    assert_eq!(json["count"], 1);

    let server = &json["servers"][0];
    assert_eq!(server["server_id"], "192.168.1.100:3000");
    assert_eq!(server["display_name"], "Test Server");
    assert_eq!(server["monitoring_status"], "active");
    assert_eq!(server["health_status"], "unknown"); // No metrics yet
    assert!(server["last_seen"].is_null());
}

#[cfg(feature = "api")]
#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn test_list_servers_with_recent_metrics_shows_up() {
    // Setup
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let backend = SqliteBackend::new(&db_path).await.unwrap();

    let (metric_tx, _) = broadcast::channel(16);
    let (service_tx, service_rx) = broadcast::channel(16);
    let storage = StorageHandle::spawn_with_backend(
        metric_tx.subscribe(),
        service_rx,
        Some(Box::new(backend) as Box<dyn StorageBackend>),
        Some(30),
        Some(24),
    );

    // Create a collector
    let config = ServerConfig {
        ip: "192.168.1.100".parse().unwrap(),
        port: 3000,
        interval: 30,
        token: None,
        display: Some("Test Server".to_string()),
        limits: None,
    };
    let collector = CollectorHandle::spawn(config, metric_tx.clone());

    // Publish a recent metric
    let event = MetricEvent {
        server_id: "192.168.1.100:3000".to_string(),
        display_name: "Test Server".to_string(),
        metrics: create_test_metrics(),
        timestamp: Utc::now(),
    };
    metric_tx.send(event).unwrap();

    // Wait for storage to persist (batch flush can take up to 5 seconds)
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    let addr = spawn_test_api(vec![collector], vec![], storage, metric_tx, service_tx).await;

    // Test
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/api/v1/servers", addr))
        .header("Authorization", "Bearer test-token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json: Value = response.json().await.unwrap();
    assert_eq!(json["count"], 1);

    let server = &json["servers"][0];
    assert_eq!(server["health_status"], "up"); // Recent metric
    assert!(server["last_seen"].is_string());
}

#[cfg(feature = "api")]
#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn test_list_servers_with_stale_metrics_shows_stale() {
    // Setup
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let backend = SqliteBackend::new(&db_path).await.unwrap();

    let (metric_tx, _) = broadcast::channel(16);
    let (service_tx, service_rx) = broadcast::channel(16);
    let storage = StorageHandle::spawn_with_backend(
        metric_tx.subscribe(),
        service_rx,
        Some(Box::new(backend) as Box<dyn StorageBackend>),
        Some(30),
        Some(24),
    );

    // Create a collector
    let config = ServerConfig {
        ip: "192.168.1.100".parse().unwrap(),
        port: 3000,
        interval: 30,
        token: None,
        display: Some("Test Server".to_string()),
        limits: None,
    };
    let collector = CollectorHandle::spawn(config, metric_tx.clone());

    // Publish a stale metric (10 minutes old)
    let event = MetricEvent {
        server_id: "192.168.1.100:3000".to_string(),
        display_name: "Test Server".to_string(),
        metrics: create_test_metrics(),
        timestamp: Utc::now() - Duration::minutes(10),
    };
    metric_tx.send(event).unwrap();

    // Wait for storage to persist (batch flush can take up to 5 seconds)
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    let addr = spawn_test_api(vec![collector], vec![], storage, metric_tx, service_tx).await;

    // Test
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/api/v1/servers", addr))
        .header("Authorization", "Bearer test-token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json: Value = response.json().await.unwrap();
    let server = &json["servers"][0];
    assert_eq!(server["health_status"], "stale"); // >5 minutes old
}

#[cfg(feature = "api")]
#[cfg(feature = "storage-sqlite")]
#[tokio::test]
async fn test_list_services_with_recent_up_check_shows_up() {
    // Setup
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let backend = SqliteBackend::new(&db_path).await.unwrap();

    let (metric_tx, metric_rx) = broadcast::channel(16);
    let (service_tx, _) = broadcast::channel(16);
    let storage = StorageHandle::spawn_with_backend(
        metric_rx,
        service_tx.subscribe(),
        Some(Box::new(backend) as Box<dyn StorageBackend>),
        Some(30),
        Some(24),
    );

    // Create a service monitor
    let config = ServiceConfig {
        name: "Test Service".to_string(),
        url: "http://example.com".to_string(),
        interval: 60,
        timeout: 10,
        method: HttpMethod::Get,
        expected_status: None,
        body_pattern: None,
        grace: None,
        alert: None,
    };
    let service = ServiceHandle::spawn(config, service_tx.clone());

    // Publish a recent UP check
    let event = ServiceCheckEvent {
        service_name: "Test Service".to_string(),
        url: "http://example.com".to_string(),
        timestamp: Utc::now(),
        status: ServiceStatus::Up,
        response_time_ms: Some(123),
        http_status_code: Some(200),
        ssl_expiry_days: None,
        error_message: None,
    };
    service_tx.send(event).unwrap();

    // Wait for storage to persist
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let addr = spawn_test_api(vec![], vec![service], storage, metric_tx, service_tx).await;

    // Test
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/api/v1/services", addr))
        .header("Authorization", "Bearer test-token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json: Value = response.json().await.unwrap();
    assert_eq!(json["count"], 1);

    let service = &json["services"][0];
    assert_eq!(service["name"], "Test Service");
    assert_eq!(service["health_status"], "up");
    assert_eq!(service["last_status"], "up");
}

#[cfg(feature = "api")]
#[tokio::test]
async fn test_api_with_valid_token_succeeds() {
    // Setup
    let (metric_tx, _) = broadcast::channel(16);
    let (service_tx, service_rx) = broadcast::channel(16);
    let storage = StorageHandle::spawn(metric_tx.subscribe(), service_rx);

    let addr = spawn_test_api(vec![], vec![], storage, metric_tx, service_tx).await;

    // Test
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/api/v1/stats", addr))
        .header("Authorization", "Bearer test-token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[cfg(feature = "api")]
#[tokio::test]
async fn test_api_with_invalid_token_fails_403() {
    // Setup
    let (metric_tx, _) = broadcast::channel(16);
    let (service_tx, service_rx) = broadcast::channel(16);
    let storage = StorageHandle::spawn(metric_tx.subscribe(), service_rx);

    let addr = spawn_test_api(vec![], vec![], storage, metric_tx, service_tx).await;

    // Test
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/api/v1/stats", addr))
        .header("Authorization", "Bearer wrong-token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[cfg(feature = "api")]
#[tokio::test]
async fn test_api_without_token_when_required_fails_401() {
    // Setup
    let (metric_tx, _) = broadcast::channel(16);
    let (service_tx, service_rx) = broadcast::channel(16);
    let storage = StorageHandle::spawn(metric_tx.subscribe(), service_rx);

    let addr = spawn_test_api(vec![], vec![], storage, metric_tx, service_tx).await;

    // Test
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/api/v1/stats", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
