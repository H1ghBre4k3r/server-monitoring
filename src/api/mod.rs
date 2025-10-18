//! REST API and WebSocket server for monitoring hub
//!
//! This module provides HTTP endpoints for querying metrics, service health,
//! and system stats, plus WebSocket support for real-time metric streaming.
//!
//! ## Architecture
//!
//! - **Axum** web framework with Tower middleware
//! - **Actor handles** for querying storage, collectors, alerts
//! - **WebSocket** for real-time metric streaming
//! - **OpenAPI** documentation via utoipa
//!
//! ## Endpoints
//!
//! - `GET /api/v1/health` - Health check
//! - `GET /api/v1/stats` - System statistics
//! - `GET /api/v1/servers` - List monitored servers
//! - `GET /api/v1/servers/{id}/metrics` - Server metrics
//! - `GET /api/v1/services` - List monitored services
//! - `GET /api/v1/services/{name}/uptime` - Service uptime
//! - `WS /api/v1/stream` - Real-time metric streaming

#[cfg(feature = "api")]
pub mod error;
#[cfg(feature = "api")]
pub mod middleware;
#[cfg(feature = "api")]
pub mod routes;
#[cfg(feature = "api")]
pub mod state;
#[cfg(feature = "api")]
pub mod types;
#[cfg(feature = "api")]
pub mod utils;
#[cfg(feature = "api")]
pub mod websocket;

#[cfg(feature = "api")]
pub use error::{ApiError, ApiResult};
#[cfg(feature = "api")]
pub use state::ApiState;
#[cfg(feature = "api")]
pub use types::{
    HealthResponse, LatestMetricsResponse, MetricsResponse, ServerInfo, ServersResponse,
    ServiceChecksResponse, ServiceInfo, ServicesResponse, StatsResponse, UptimeResponse,
};

#[cfg(feature = "api")]
use axum::{Router, routing::get};
use std::net::SocketAddr;
use tracing::info;

/// API server configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Bind address (e.g., "0.0.0.0:8080")
    pub bind_addr: SocketAddr,

    /// Optional authentication token
    pub auth_token: Option<String>,

    /// Enable CORS for dashboard
    pub enable_cors: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            auth_token: None,
            enable_cors: true,
        }
    }
}

/// Spawn the API server
///
/// This starts an Axum HTTP server in a background task.
/// Returns the server's local address.
#[cfg(feature = "api")]
pub async fn spawn_api_server(config: ApiConfig, state: ApiState) -> anyhow::Result<SocketAddr> {
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::trace::TraceLayer;

    info!("starting API server on {}", config.bind_addr);

    // Build router with all routes
    let mut app = Router::new()
        .route("/api/v1/health", get(routes::health::health_check))
        .route("/api/v1/stats", get(routes::stats::get_stats))
        .route("/api/v1/servers", get(routes::servers::list_servers))
        .route(
            "/api/v1/servers/:id/metrics",
            get(routes::servers::get_server_metrics),
        )
        .route(
            "/api/v1/servers/:id/metrics/latest",
            get(routes::servers::get_latest_metrics),
        )
        .route("/api/v1/services", get(routes::services::list_services))
        .route(
            "/api/v1/services/:name/checks",
            get(routes::services::get_service_checks),
        )
        .route(
            "/api/v1/services/:name/uptime",
            get(routes::services::get_uptime),
        )
        .route("/api/v1/stream", get(websocket::websocket_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    // Add web dashboard static files if feature enabled
    #[cfg(feature = "web-dashboard")]
    {
        use std::path::Path;
        use tower_http::services::ServeDir;

        // Try to serve from dist directory (if it exists)
        let dist_path = Path::new("web-dashboard/dist");
        if dist_path.exists() {
            info!("serving web dashboard from {}", dist_path.display());
            let serve_dir = ServeDir::new(dist_path)
                .precompressed_br()
                .precompressed_deflate()
                .precompressed_gzip();

            app = app.nest_service("/", serve_dir);
        } else {
            info!(
                "web dashboard dist directory not found at {}",
                dist_path.display()
            );
            info!("run 'npm run build' in web-dashboard directory to build the dashboard");
        }
    }

    // Add CORS if enabled
    if config.enable_cors {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        app = app.layer(cors);
    }

    // Add auth middleware if token provided
    if let Some(token) = config.auth_token {
        app = app.layer(axum::middleware::from_fn_with_state(
            token,
            middleware::auth::auth_middleware,
        ));
    }

    // Bind and serve
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let addr = listener.local_addr()?;

    info!("API server listening on {}", addr);

    // Spawn server in background
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("API server error: {}", e);
        }
    });

    Ok(addr)
}
