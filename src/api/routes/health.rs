//! Health check endpoint

use axum::Json;
use serde_json::{Value, json};

/// GET /api/v1/health
///
/// Returns a simple health check response
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
