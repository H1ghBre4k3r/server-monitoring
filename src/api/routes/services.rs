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
                let age = Utc::now() - check.timestamp;

                // Consider stale if older than 5 minutes
                if age.num_seconds() > 300 {
                    (
                        "stale".to_string(),
                        Some(check.timestamp.to_rfc3339()),
                        Some(check.status.as_str().to_string()),
                    )
                } else {
                    // Use the actual check status
                    let status = check.status.as_str().to_string();
                    (
                        status.clone(),
                        Some(check.timestamp.to_rfc3339()),
                        Some(status),
                    )
                }
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
    let start = query.start.unwrap_or_else(|| end - Duration::hours(24));

    let checks = state
        .storage
        .query_service_checks_range(service_name.clone(), start, end)
        .await?;

    Ok(Json(ServiceChecksResponse {
        service_name,
        start: start.to_rfc3339(),
        end: end.to_rfc3339(),
        count: checks.len(),
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
        .unwrap_or_else(|| Utc::now() - Duration::hours(24));

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

#[derive(Debug, Deserialize)]
pub struct ServiceCheckQuery {
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UptimeQuery {
    since: Option<DateTime<Utc>>,
}
