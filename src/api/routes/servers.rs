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
        types::{LatestMetricsResponse, MetricsResponse, ServerInfo, ServersResponse},
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

/// Determine health status based on metric age
fn determine_health_status(metric_age_secs: i64) -> &'static str {
    if metric_age_secs > STALE_THRESHOLD_SECS {
        "stale"
    } else {
        "up"
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

        // Query latest metric to determine health
        let (health_status, last_seen) =
            match state.storage.query_latest(server_id.clone(), 1).await {
                Ok(metrics) if !metrics.is_empty() => {
                    let metric = &metrics[0];
                    let age_secs = (Utc::now() - metric.timestamp).num_seconds();
                    let status = determine_health_status(age_secs);
                    let timestamp = metric.timestamp.to_rfc3339();
                    (status, Some(timestamp))
                }
                _ => ("unknown", None),
            };

        servers.push(ServerInfo {
            server_id,
            display_name,
            monitoring_status: "active".to_string(),
            health_status: health_status.to_string(),
            last_seen,
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
