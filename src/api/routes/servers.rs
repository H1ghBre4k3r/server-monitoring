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
        types::{
            LatestMetricsResponse, MetricsResponse, MonitoringStatus, ServerInfo, ServersResponse,
        },
        utils::determine_server_health,
    },
    storage::backend::QueryRange,
};

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
                    let timestamp = metric.timestamp.to_rfc3339();
                    let health_status = determine_server_health(
                        Some(metric.timestamp),
                        polling_status.last_success_timestamp,
                        polling_status.last_error_timestamp,
                    );
                    (health_status, Some(timestamp))
                }
                _ => {
                    // No metrics available - use shared utility
                    let health_status = determine_server_health(
                        None,
                        polling_status.last_success_timestamp,
                        polling_status.last_error_timestamp,
                    );
                    (health_status, None)
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
