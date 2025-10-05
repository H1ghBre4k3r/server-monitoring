//! Bearer token authentication middleware

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Authentication middleware
///
/// Checks for Bearer token in Authorization header
pub async fn auth_middleware(
    State(expected_token): State<String>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    // Get Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    // Check Bearer token format
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidFormat)?;

    // Verify token matches expected
    if token != expected_token {
        return Err(AuthError::InvalidToken);
    }

    Ok(next.run(request).await)
}

/// Authentication errors
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidFormat,
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing Authorization header"),
            AuthError::InvalidFormat => (
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization format (expected: Bearer <token>)",
            ),
            AuthError::InvalidToken => (StatusCode::FORBIDDEN, "Invalid token"),
        };

        (status, message).into_response()
    }
}
