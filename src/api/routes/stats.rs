//! System statistics endpoint

use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::api::{error::ApiResult, state::ApiState};

/// GET /api/v1/stats
///
/// Returns system statistics including storage stats and actor counts
pub async fn get_stats(State(state): State<ApiState>) -> ApiResult<Json<Value>> {
    // Get storage statistics
    let storage_stats = state.storage.get_stats().await.unwrap_or_default();

    Ok(Json(json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "storage": {
            "total_metrics": storage_stats.total_metrics,
            "buffer_size": storage_stats.buffer_size,
            "flush_count": storage_stats.flush_count,
            "last_cleanup": storage_stats.last_cleanup_time.map(|t| t.to_rfc3339()),
            "total_metrics_deleted": storage_stats.total_metrics_deleted,
            "total_service_checks_deleted": storage_stats.total_service_checks_deleted,
        },
        "collectors": state.collectors.len(),
        "service_monitors": state.service_monitors.len(),
    })))
}
