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
                    let age = Utc::now() - metric.timestamp;

                    // Consider stale if older than 5 minutes
                    if age.num_seconds() > 300 {
                        ("stale", Some(metric.timestamp.to_rfc3339()))
                    } else {
                        ("up", Some(metric.timestamp.to_rfc3339()))
                    }
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
    let limit = query.limit.unwrap_or(1000).min(10000);

    let query_range = QueryRange {
        server_id: server_id.clone(),
        start,
        end,
        limit: Some(limit),
    };

    let metrics = state.storage.query_range(query_range).await?;

    Ok(Json(MetricsResponse {
        server_id,
        start: start.to_rfc3339(),
        end: end.to_rfc3339(),
        count: metrics.len(),
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
    let limit = query.limit.unwrap_or(100).min(1000);

    let metrics = state.storage.query_latest(server_id.clone(), limit).await?;

    Ok(Json(LatestMetricsResponse {
        server_id,
        count: metrics.len(),
        metrics,
    }))
}

#[derive(Debug, Deserialize)]
pub struct LatestQuery {
    limit: Option<usize>,
}
