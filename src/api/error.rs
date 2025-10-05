//! API error types and conversions

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// API result type
pub type ApiResult<T> = Result<T, ApiError>;

/// API error types
#[derive(Debug)]
pub enum ApiError {
    /// Storage operation failed
    StorageError(String),

    /// Invalid request parameters
    InvalidRequest(String),

    /// Resource not found
    NotFound(String),

    /// Internal server error
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::StorageError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

impl From<crate::storage::error::StorageError> for ApiError {
    fn from(err: crate::storage::error::StorageError) -> Self {
        ApiError::StorageError(err.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}
