//! Service monitoring endpoints

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

use crate::api::{
    error::ApiResult,
    state::ApiState,
    types::{ServiceChecksResponse, ServiceInfo, ServicesResponse, UptimeResponse},
};

/// Maximum age in seconds before a service check is considered stale
const STALE_THRESHOLD_SECS: i64 = 300; // 5 minutes

/// Default lookback period for service checks (24 hours)
const DEFAULT_LOOKBACK_HOURS: i64 = 24;

/// Query parameters for service check history
#[derive(Debug, Deserialize)]
pub struct ServiceCheckQuery {
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
}

/// Query parameters for uptime statistics
#[derive(Debug, Deserialize)]
pub struct UptimeQuery {
    since: Option<DateTime<Utc>>,
}

/// Determine service health status and metadata from latest check
fn determine_service_health(
    check: &crate::storage::schema::ServiceCheckRow,
) -> (String, String, String) {
    let age_secs = (Utc::now() - check.timestamp).num_seconds();
    let status_str = check.status.as_str().to_string();
    let timestamp = check.timestamp.to_rfc3339();

    if age_secs > STALE_THRESHOLD_SECS {
        ("stale".to_string(), timestamp, status_str)
    } else {
        (status_str.clone(), timestamp, status_str)
    }
}

/// GET /api/v1/services
///
/// List all monitored services with health status
pub async fn list_services(State(state): State<ApiState>) -> ApiResult<Json<ServicesResponse>> {
    let mut services = Vec::new();

    for monitor in &state.service_monitors {
        let service_name = monitor.service_name().to_string();
        let url = monitor.service_url().to_string();

        // Query latest service check to determine health
        let (health_status, last_check, last_status) = match state
            .storage
            .query_latest_service_checks(service_name.clone(), 1)
            .await
        {
            Ok(checks) if !checks.is_empty() => {
                let check = &checks[0];
                let (health, timestamp, status) = determine_service_health(check);
                (health, Some(timestamp), Some(status))
            }
            _ => ("unknown".to_string(), None, None),
        };

        services.push(ServiceInfo {
            name: service_name,
            url,
            monitoring_status: "active".to_string(),
            health_status,
            last_check,
            last_status,
        });
    }

    let count = services.len();
    Ok(Json(ServicesResponse { services, count }))
}

/// GET /api/v1/services/:name/checks
///
/// Get service check history for a specific service
pub async fn get_service_checks(
    State(state): State<ApiState>,
    Path(service_name): Path<String>,
    Query(query): Query<ServiceCheckQuery>,
) -> ApiResult<Json<ServiceChecksResponse>> {
    let end = query.end.unwrap_or_else(Utc::now);
    let start = query
        .start
        .unwrap_or_else(|| end - Duration::hours(DEFAULT_LOOKBACK_HOURS));

    let checks = state
        .storage
        .query_service_checks_range(service_name.clone(), start, end)
        .await?;

    let count = checks.len();

    Ok(Json(ServiceChecksResponse {
        service_name,
        start: start.to_rfc3339(),
        end: end.to_rfc3339(),
        count,
        checks,
    }))
}

/// GET /api/v1/services/:name/uptime
///
/// Get uptime statistics for a service
pub async fn get_uptime(
    State(state): State<ApiState>,
    Path(service_name): Path<String>,
    Query(query): Query<UptimeQuery>,
) -> ApiResult<Json<UptimeResponse>> {
    let since = query
        .since
        .unwrap_or_else(|| Utc::now() - Duration::hours(DEFAULT_LOOKBACK_HOURS));

    let uptime_stats = state
        .storage
        .calculate_uptime(service_name.clone(), since)
        .await?;

    Ok(Json(UptimeResponse {
        service_name,
        since: since.to_rfc3339(),
        start: uptime_stats.start.to_rfc3339(),
        end: uptime_stats.end.to_rfc3339(),
        uptime_percentage: uptime_stats.uptime_percentage,
        total_checks: uptime_stats.total_checks,
        successful_checks: uptime_stats.successful_checks,
        avg_response_time_ms: uptime_stats.avg_response_time_ms,
    }))
}
