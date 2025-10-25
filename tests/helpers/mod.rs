//! Test helpers and utilities for integration and end-to-end tests

use chrono::Utc;
use guardia::{
    config::{Alert, Discord, Limit, Limits, ResolvedLimit, ResolvedLimits, ResolvedServerConfig, ResolvedServiceConfig, ServerConfig, Webhook},
    ServerMetrics, ComponentOverview, CpuOverview, MemoryInformation, SystemInformation,
    actors::messages::MetricEvent,
};
use std::net::IpAddr;
use std::str::FromStr;

/// Create a test ServerConfig with sensible defaults
pub fn create_test_server_config(ip: &str, port: u16) -> ServerConfig {
    ServerConfig {
        ip: IpAddr::from_str(ip).unwrap(),
        port,
        interval: Some(5),
        token: Some("test-token".to_string()),
        display: Some(format!("Test Server {ip}:{port}")),
        limits: None,
    }
}

/// Create a ServerConfig with configured limits
pub fn create_test_server_with_limits(
    ip: &str,
    port: u16,
    temp_limit: Option<usize>,
    cpu_limit: Option<usize>,
    grace: usize,
) -> ServerConfig {
    let mut config = create_test_server_config(ip, port);

    config.limits = Some(Limits {
        temperature: temp_limit.map(|limit| Limit {
            limit,
            grace: Some(grace),
            alert: None,
        }),
        usage: cpu_limit.map(|limit| Limit {
            limit,
            grace: Some(grace),
            alert: None,
        }),
    });

    config
}

/// Create a ServerConfig with Discord alerts
pub fn create_test_server_with_discord_alert(
    ip: &str,
    port: u16,
    webhook_url: &str,
) -> ServerConfig {
    let mut config = create_test_server_config(ip, port);

    let discord_alert = Alert::Discord(Discord {
        url: webhook_url.to_string(),
        user_id: Some("123456789".to_string()),
    });

    config.limits = Some(Limits {
        temperature: Some(Limit {
            limit: 70,
            grace: Some(3),
            alert: Some(discord_alert.clone()),
        }),
        usage: Some(Limit {
            limit: 80,
            grace: Some(5),
            alert: Some(discord_alert),
        }),
    });

    config
}

/// Create a ServerConfig with generic webhook alerts
pub fn create_test_server_with_webhook_alert(
    ip: &str,
    port: u16,
    webhook_url: &str,
) -> ServerConfig {
    let mut config = create_test_server_config(ip, port);

    let webhook_alert = Alert::Webhook(Webhook {
        url: webhook_url.to_string(),
    });

    config.limits = Some(Limits {
        temperature: Some(Limit {
            limit: 70,
            grace: Some(3),
            alert: Some(webhook_alert.clone()),
        }),
        usage: Some(Limit {
            limit: 80,
            grace: Some(5),
            alert: Some(webhook_alert),
        }),
    });

    config
}

/// Create test ServerMetrics with custom values
pub fn create_test_metrics(cpu_usage: f32, temperature: Option<f32>) -> ServerMetrics {
    ServerMetrics {
        system: SystemInformation {
            name: Some("TestOS".to_string()),
            kernel_version: Some("5.0.0".to_string()),
            os_version: Some("TestOS 1.0".to_string()),
            host_name: Some("test-host".to_string()),
        },
        memory: MemoryInformation {
            total: 16_000_000_000,
            used: 8_000_000_000,
            total_swap: 4_000_000_000,
            used_swap: 1_000_000_000,
        },
        cpus: CpuOverview {
            total: 8,
            arch: "x86_64".to_string(),
            average_usage: cpu_usage,
            cpus: vec![],
        },
        components: ComponentOverview {
            average_temperature: temperature,
            components: vec![],
        },
    }
}

/// Create a MetricEvent for testing
pub fn create_test_metric_event(
    server_id: &str,
    cpu_usage: f32,
    temperature: Option<f32>,
) -> MetricEvent {
    MetricEvent {
        server_id: server_id.to_string(),
        metrics: create_test_metrics(cpu_usage, temperature),
        timestamp: Utc::now(),
        display_name: format!("Test {server_id}"),
    }
}

/// Create default test metrics (50% CPU, 45°C)
pub fn create_default_test_metrics() -> ServerMetrics {
    create_test_metrics(50.0, Some(45.0))
}

/// Wait for a metric event on a broadcast channel with timeout
/// Returns the received event or None if timeout
pub async fn wait_for_metric_event(
    rx: &mut tokio::sync::broadcast::Receiver<MetricEvent>,
    timeout_ms: u64,
) -> Option<MetricEvent> {
    tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout_ms),
        rx.recv(),
    )
    .await
    .ok()?
    .ok()
}

/// Create a test ResolvedServerConfig with sensible defaults
pub fn create_test_resolved_server_config(ip: &str, port: u16) -> ResolvedServerConfig {
    ResolvedServerConfig {
        ip: IpAddr::from_str(ip).unwrap(),
        port,
        interval: 5,
        token: Some("test-token".to_string()),
        display: Some(format!("Test Server {ip}:{port}")),
        limits: None,
    }
}

/// Create a ResolvedServerConfig with configured limits
pub fn create_test_resolved_server_with_limits(
    ip: &str,
    port: u16,
    temp_limit: Option<usize>,
    cpu_limit: Option<usize>,
    grace: usize,
) -> ResolvedServerConfig {
    let mut config = create_test_resolved_server_config(ip, port);

    config.limits = Some(ResolvedLimits {
        temperature: temp_limit.map(|limit| ResolvedLimit {
            limit,
            grace: Some(grace),
            alert: None,
        }),
        usage: cpu_limit.map(|limit| ResolvedLimit {
            limit,
            grace: Some(grace),
            alert: None,
        }),
    });

    config
}

/// Create a ResolvedServerConfig with Discord alerts
pub fn create_test_resolved_server_with_discord_alert(
    ip: &str,
    port: u16,
    webhook_url: &str,
) -> ResolvedServerConfig {
    let mut config = create_test_resolved_server_config(ip, port);

    let discord_alert = Alert::Discord(Discord {
        url: webhook_url.to_string(),
        user_id: Some("123456789".to_string()),
    });

    config.limits = Some(ResolvedLimits {
        temperature: Some(ResolvedLimit {
            limit: 70,
            grace: Some(3),
            alert: Some(discord_alert.clone()),
        }),
        usage: Some(ResolvedLimit {
            limit: 80,
            grace: Some(5),
            alert: Some(discord_alert),
        }),
    });

    config
}

/// Create a ResolvedServerConfig with generic webhook alerts
pub fn create_test_resolved_server_with_webhook_alert(
    ip: &str,
    port: u16,
    webhook_url: &str,
) -> ResolvedServerConfig {
    let mut config = create_test_resolved_server_config(ip, port);

    let webhook_alert = Alert::Webhook(Webhook {
        url: webhook_url.to_string(),
    });

    config.limits = Some(ResolvedLimits {
        temperature: Some(ResolvedLimit {
            limit: 70,
            grace: Some(3),
            alert: Some(webhook_alert.clone()),
        }),
        usage: Some(ResolvedLimit {
            limit: 80,
            grace: Some(5),
            alert: Some(webhook_alert),
        }),
    });

    config
}
