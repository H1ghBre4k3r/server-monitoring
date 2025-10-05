//! Health check endpoint

use crate::api::types::HealthResponse;
use axum::Json;

/// GET /api/v1/health
///
/// Returns a simple health check response
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
