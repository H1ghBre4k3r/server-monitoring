//! Helper functions for integration tests

use guardia::config::{ResolvedLimit, ResolvedLimits, ResolvedServerConfig};
use std::net::IpAddr;
use std::str::FromStr;

pub fn create_test_server_config(ip: &str, port: u16) -> ResolvedServerConfig {
    ResolvedServerConfig {
        ip: IpAddr::from_str(ip).unwrap(),
        port,
        interval: 5, // Already resolved, so this is usize not Option
        token: Some("test-token".to_string()),
        display: Some(format!("Test {ip}:{port}")),
        limits: None,
    }
}

pub fn create_test_server_with_limits(
    ip: &str,
    port: u16,
    temp_limit: Option<usize>,
    cpu_limit: Option<usize>,
    grace: usize,
) -> ResolvedServerConfig {
    let mut config = create_test_server_config(ip, port);

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

pub fn create_mock_metrics_json(cpu_usage: f32, temperature: Option<f32>) -> serde_json::Value {
    serde_json::json!({
        "system": {
            "name": "TestOS",
            "kernel_version": "5.0.0",
            "os_version": "Test 1.0",
            "host_name": "test-host"
        },
        "memory": {
            "total": 16000000000u64,
            "used": 8000000000u64,
            "total_swap": 4000000000u64,
            "used_swap": 1000000000u64
        },
        "cpus": {
            "total": 8,
            "arch": "x86_64",
            "average_usage": cpu_usage,
            "cpus": []
        },
        "components": {
            "average_temperature": temperature,
            "components": []
        }
    })
}
