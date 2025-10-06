//! Shared utilities for API and UI components
//!
//! This module contains common utilities that are used across multiple components
//! to avoid code duplication and ensure consistency.

use ratatui::style::Color;

use super::types::{MonitoringStatus, ServerHealthStatus, ServiceHealthStatus};

// ============================================================================
// Status Color Mapping Utilities
// ============================================================================

impl From<ServerHealthStatus> for Color {
    fn from(value: ServerHealthStatus) -> Self {
        match value {
            ServerHealthStatus::Up => Color::Green,
            ServerHealthStatus::Down => Color::Red,
            ServerHealthStatus::Stale => Color::Yellow,
            ServerHealthStatus::Unknown => Color::Gray,
        }
    }
}

impl From<ServiceHealthStatus> for Color {
    fn from(value: ServiceHealthStatus) -> Self {
        match value {
            ServiceHealthStatus::Up => Color::Green,
            ServiceHealthStatus::Down => Color::Red,
            ServiceHealthStatus::Degraded => Color::Yellow,
            ServiceHealthStatus::Stale => Color::Magenta,
            ServiceHealthStatus::Unknown => Color::Gray,
        }
    }
}

impl From<MonitoringStatus> for Color {
    fn from(value: MonitoringStatus) -> Self {
        match value {
            MonitoringStatus::Active => Color::Green,
            MonitoringStatus::Paused => Color::Yellow,
            MonitoringStatus::Disabled => Color::Gray,
        }
    }
}

// ============================================================================
// Common Constants
// ============================================================================

/// Maximum age in seconds before data is considered stale
pub const STALE_THRESHOLD_SECS: i64 = 300; // 5 minutes

/// Maximum age in seconds before polling failures indicate server is down
pub const DOWN_THRESHOLD_SECS: i64 = 120; // 2 minutes

// ============================================================================
// Health Status Determination Utilities
// ============================================================================

use chrono::{DateTime, Utc};
use std::time::Duration;

/// Determine if metrics are stale based on timestamp
pub fn is_metrics_stale(timestamp: DateTime<Utc>) -> bool {
    let age_secs = (Utc::now() - timestamp).num_seconds();
    age_secs > STALE_THRESHOLD_SECS
}

// ============================================================================
// HTTP Request Utilities
// ============================================================================

/// Create an HTTP client with common settings for monitoring
pub fn create_monitoring_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client")
}

/// Add optional authentication header to a request builder
pub fn add_auth_header(
    request: reqwest::RequestBuilder,
    token: Option<&str>,
) -> reqwest::RequestBuilder {
    if let Some(token) = token {
        request.header("X-MONITORING-SECRET", token)
    } else {
        request
    }
}

/// Handle common HTTP request errors
pub fn handle_http_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "Request timeout".to_string()
    } else if error.is_connect() {
        "Connection failed".to_string()
    } else if error.is_request() {
        format!("HTTP request failed: {}", error)
    } else {
        format!("Unexpected error: {}", error)
    }
}

/// Build a complete URL for monitoring endpoints
pub fn build_monitoring_url(ip: &str, port: u16, endpoint: &str) -> String {
    format!("http://{}:{}{}", ip, port, endpoint)
}

/// Determine if polling is stale based on timestamp
pub fn is_polling_stale(timestamp: DateTime<Utc>) -> bool {
    let age_secs = (Utc::now() - timestamp).num_seconds();
    age_secs > STALE_THRESHOLD_SECS
}

/// Determine if recent polling failures indicate server is down
pub fn is_server_down(last_error_time: Option<DateTime<Utc>>) -> bool {
    if let Some(error_time) = last_error_time {
        let error_age_secs = (Utc::now() - error_time).num_seconds();
        error_age_secs < DOWN_THRESHOLD_SECS
    } else {
        false
    }
}

/// Determine server health status based on metric age and polling status
pub fn determine_server_health(
    metric_timestamp: Option<DateTime<Utc>>,
    last_poll_success: Option<DateTime<Utc>>,
    last_poll_error: Option<DateTime<Utc>>,
) -> ServerHealthStatus {
    // Check if we have recent polling failures (server is down)
    if is_server_down(last_poll_error) {
        return ServerHealthStatus::Down;
    }

    // Check if we have recent successful polls and recent metrics
    if let Some(success_time) = last_poll_success {
        let success_age_secs = (Utc::now() - success_time).num_seconds();

        if let Some(metric_time) = metric_timestamp {
            let metric_age_secs = (Utc::now() - metric_time).num_seconds();

            // If last poll was successful but metrics are old, mark as stale
            if metric_age_secs > STALE_THRESHOLD_SECS && success_age_secs < STALE_THRESHOLD_SECS {
                return ServerHealthStatus::Stale;
            }

            // If both polling and metrics are recent, mark as up
            if metric_age_secs <= STALE_THRESHOLD_SECS && success_age_secs < STALE_THRESHOLD_SECS {
                return ServerHealthStatus::Up;
            }
        } else if success_age_secs < STALE_THRESHOLD_SECS {
            // No metrics but recent successful polling
            return ServerHealthStatus::Up;
        }
    }

    // If no recent polling data, check metric age
    if let Some(metric_time) = metric_timestamp {
        if is_metrics_stale(metric_time) {
            ServerHealthStatus::Stale
        } else {
            ServerHealthStatus::Up
        }
    } else {
        ServerHealthStatus::Unknown
    }
}
