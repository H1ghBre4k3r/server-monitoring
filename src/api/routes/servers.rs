//! Server metrics endpoints

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

use crate::{
    api::{
        error::ApiResult,
        state::ApiState,
        types::{LatestMetricsResponse, MetricsResponse, ServerInfo, ServersResponse, MonitoringStatus, ServerHealthStatus},
    },
    storage::backend::QueryRange,
};

/// Maximum age in seconds before a metric is considered stale
const STALE_THRESHOLD_SECS: i64 = 300; // 5 minutes

/// Default limit for metrics query
const DEFAULT_METRICS_LIMIT: usize = 1000;

/// Maximum limit for metrics query
const MAX_METRICS_LIMIT: usize = 10000;

/// Default limit for latest metrics query
const DEFAULT_LATEST_LIMIT: usize = 100;

/// Maximum limit for latest metrics query
const MAX_LATEST_LIMIT: usize = 1000;

/// Query parameters for metric time range
#[derive(Debug, Deserialize)]
pub struct MetricQuery {
    /// Start time (ISO 8601 format, default: 1 hour ago)
    start: Option<DateTime<Utc>>,

    /// End time (ISO 8601 format, default: now)
    end: Option<DateTime<Utc>>,

    /// Max results (default: 1000)
    limit: Option<usize>,
}

/// Query parameters for latest metrics
#[derive(Debug, Deserialize)]
pub struct LatestQuery {
    limit: Option<usize>,
}

/// Determine health status based on metric age and polling status
fn determine_health_status(
    metric_age_secs: i64,
    polling_status: &crate::api::state::PollingStatus,
) -> ServerHealthStatus {
    let now = Utc::now();

    // Check if we have recent polling failures (server is down)
    if let Some(last_error_time) = polling_status.last_error_timestamp {
        let error_age_secs = (now - last_error_time).num_seconds();

        // If the last poll was within the last 2 minutes and failed, mark as down
        if error_age_secs < 120 {
            return ServerHealthStatus::Down;
        }
    }

    // Check if we have recent successful polls and recent metrics
    if let Some(last_success_time) = polling_status.last_success_timestamp {
        let success_age_secs = (now - last_success_time).num_seconds();

        // If last poll was successful but metrics are old, mark as stale
        if metric_age_secs > STALE_THRESHOLD_SECS && success_age_secs < STALE_THRESHOLD_SECS {
            return ServerHealthStatus::Stale;
        }

        // If both polling and metrics are recent, mark as up
        if metric_age_secs <= STALE_THRESHOLD_SECS && success_age_secs < STALE_THRESHOLD_SECS {
            return ServerHealthStatus::Up;
        }
    }

    // If no recent polling data, check metric age
    if metric_age_secs > STALE_THRESHOLD_SECS {
        ServerHealthStatus::Stale
    } else {
        ServerHealthStatus::Up
    }
}

/// GET /api/v1/servers
///
/// List all monitored servers with health status
pub async fn list_servers(State(state): State<ApiState>) -> ApiResult<Json<ServersResponse>> {
    let mut servers = Vec::new();

    for collector in &state.collectors {
        let server_id = collector.server_id().to_string();
        let display_name = collector.display_name.clone();

        // Get polling status for this server
        let polling_status = state.polling_store.get_status(&server_id).await;

        // Query latest metric to determine health
        let (health_status, last_seen) =
            match state.storage.query_latest(server_id.clone(), 1).await {
                Ok(metrics) if !metrics.is_empty() => {
                    let metric = &metrics[0];
                    let age_secs = (Utc::now() - metric.timestamp).num_seconds();
                    let status = determine_health_status(age_secs, &polling_status);
                    let timestamp = metric.timestamp.to_rfc3339();
                    (status, Some(timestamp))
                }
                _ => {
                    // No metrics available - check if we have polling data
                    if polling_status.last_error_timestamp.is_some() {
                        (ServerHealthStatus::Down, None)
                    } else {
                        (ServerHealthStatus::Unknown, None)
                    }
                }
            };

        servers.push(ServerInfo {
            server_id,
            display_name,
            monitoring_status: MonitoringStatus::Active,
            health_status,
            last_seen,
            last_poll_success: polling_status.last_success,
            last_poll_error: polling_status.last_error,
        });
    }

    let count = servers.len();
    Ok(Json(ServersResponse { servers, count }))
}

/// GET /api/v1/servers/:id/metrics
///
/// Get metrics for a specific server within time range
pub async fn get_server_metrics(
    State(state): State<ApiState>,
    Path(server_id): Path<String>,
    Query(query): Query<MetricQuery>,
) -> ApiResult<Json<MetricsResponse>> {
    let end = query.end.unwrap_or_else(Utc::now);
    let start = query.start.unwrap_or_else(|| end - Duration::hours(1));
    let limit = query
        .limit
        .unwrap_or(DEFAULT_METRICS_LIMIT)
        .min(MAX_METRICS_LIMIT);

    let query_range = QueryRange {
        server_id: server_id.clone(),
        start,
        end,
        limit: Some(limit),
    };

    let metrics = state.storage.query_range(query_range).await?;
    let count = metrics.len();

    Ok(Json(MetricsResponse {
        server_id,
        start: start.to_rfc3339(),
        end: end.to_rfc3339(),
        count,
        metrics,
    }))
}

/// GET /api/v1/servers/:id/metrics/latest
///
/// Get the N most recent metrics for a server
pub async fn get_latest_metrics(
    State(state): State<ApiState>,
    Path(server_id): Path<String>,
    Query(query): Query<LatestQuery>,
) -> ApiResult<Json<LatestMetricsResponse>> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_LATEST_LIMIT)
        .min(MAX_LATEST_LIMIT);

    let metrics = state.storage.query_latest(server_id.clone(), limit).await?;
    let count = metrics.len();

    Ok(Json(LatestMetricsResponse {
        server_id,
        count,
        metrics,
    }))
}
