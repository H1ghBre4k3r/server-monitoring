# API Design & Specification

## Overview

This document specifies the REST and WebSocket APIs for the guardia system. These APIs enable:
- **Remote dashboard access** (TUI viewer connecting to hub)
- **Third-party integrations** (custom tools, scripts, automation)
- **Real-time metric streaming** (WebSocket for live updates)
- **Historical data queries** (REST for analytics)

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     API Layer (Axum)                    │
├─────────────────┬──────────────────┬────────────────────┤
│  REST Endpoints │  WebSocket       │  Authentication    │
│  (HTTP)         │  (WS)            │  (Middleware)      │
└────────┬────────┴────────┬─────────┴─────────┬──────────┘
         │                 │                   │
         ▼                 ▼                   ▼
┌────────────────┐  ┌─────────────┐  ┌─────────────────┐
│  StorageActor  │  │ Broadcast   │  │  Auth Service   │
│  (Query)       │  │ Channel     │  │  (Tokens)       │
└────────────────┘  └─────────────┘  └─────────────────┘
```

---

## Technology Stack

| Component | Library | Version |
|-----------|---------|---------|
| Web Framework | **Axum** | 0.7+ |
| WebSocket | **axum-tungstenite** | 0.1+ |
| Serialization | **serde + serde_json** | 1.0+ |
| Authentication | **jsonwebtoken** | 9.0+ |
| API Documentation | **utoipa** (OpenAPI) | 5.0+ |

```toml
[dependencies]
axum = { version = "0.7", features = ["ws"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "compression"] }
tokio = { version = "1.47", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
jsonwebtoken = "9"
utoipa = "5"
utoipa-swagger-ui = { version = "8", features = ["axum"] }
```

---

## REST API Specification

### Base URL

```
http://localhost:8080/api/v1
```

### Authentication

All endpoints (except `/health`) require authentication via Bearer token:

```
Authorization: Bearer <jwt_token>
```

### Global Headers

**Request:**
```
Content-Type: application/json
Accept: application/json
Authorization: Bearer <token>
```

**Response:**
```
Content-Type: application/json
X-Request-ID: <uuid>
X-RateLimit-Remaining: <count>
```

---

## Endpoints

### System

#### `GET /health`

Health check endpoint (no auth required)

**Response 200:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 86400,
  "timestamp": "2025-01-15T14:32:05Z"
}
```

#### `GET /version`

Get hub version information

**Response 200:**
```json
{
  "version": "1.0.0",
  "commit": "abc123def",
  "build_date": "2025-01-15",
  "rust_version": "1.75.0"
}
```

---

### Servers

#### `GET /servers`

List all monitored servers

**Query Parameters:**
- `status` (optional): Filter by status (`up`, `down`, `unknown`)
- `page` (optional): Page number (default: 1)
- `per_page` (optional): Results per page (default: 50, max: 100)

**Response 200:**
```json
{
  "servers": [
    {
      "id": "192.168.1.100:3000",
      "display_name": "Production Server",
      "ip": "192.168.1.100",
      "port": 3000,
      "status": "up",
      "last_seen": "2025-01-15T14:32:03Z",
      "current_metrics": {
        "cpu_usage_avg": 45.2,
        "temperature_avg": 68.5,
        "memory_used_pct": 62.3
      }
    }
  ],
  "pagination": {
    "page": 1,
    "per_page": 50,
    "total": 10,
    "total_pages": 1
  }
}
```

#### `GET /servers/:id`

Get detailed server information

**Path Parameters:**
- `id`: Server ID (e.g., `192.168.1.100:3000`)

**Response 200:**
```json
{
  "id": "192.168.1.100:3000",
  "display_name": "Production Server",
  "ip": "192.168.1.100",
  "port": 3000,
  "status": "up",
  "last_seen": "2025-01-15T14:32:03Z",
  "system_info": {
    "os": "Ubuntu 22.04 LTS",
    "kernel": "5.15.0-91",
    "cpu_arch": "x86_64",
    "cpu_count": 16,
    "memory_total_gb": 32.0
  },
  "limits": {
    "temperature": {
      "limit": 75,
      "grace": 3
    },
    "usage": {
      "limit": 80,
      "grace": 5
    }
  },
  "uptime": {
    "seconds": 1296000,
    "human": "15d 7h 23m"
  }
}
```

#### `GET /servers/:id/metrics`

Query historical metrics for a server

**Path Parameters:**
- `id`: Server ID

**Query Parameters:**
- `metric_type` (required): Metric type (`cpu_usage_avg`, `temperature_avg`, `memory_used_pct`)
- `start` (required): Start time (ISO 8601 or Unix timestamp)
- `end` (required): End time (ISO 8601 or Unix timestamp)
- `aggregation` (optional): Aggregation window (`raw`, `1m`, `5m`, `1h`, `1d`) (default: `raw`)
- `limit` (optional): Max results (default: 1000, max: 10000)

**Response 200:**
```json
{
  "server_id": "192.168.1.100:3000",
  "metric_type": "cpu_usage_avg",
  "aggregation": "5m",
  "data_points": [
    {
      "timestamp": "2025-01-15T14:00:00Z",
      "value": 45.2,
      "min": 42.1,
      "max": 48.3,
      "count": 10
    },
    {
      "timestamp": "2025-01-15T14:05:00Z",
      "value": 52.8,
      "min": 50.2,
      "max": 55.1,
      "count": 10
    }
  ],
  "metadata": {
    "start": "2025-01-15T14:00:00Z",
    "end": "2025-01-15T15:00:00Z",
    "count": 12
  }
}
```

#### `GET /servers/:id/alerts`

Get alert history for a server

**Query Parameters:**
- `severity` (optional): Filter by severity (`info`, `warning`, `critical`)
- `start` (optional): Start time
- `end` (optional): End time
- `limit` (optional): Max results (default: 100)

**Response 200:**
```json
{
  "alerts": [
    {
      "id": "alert_123abc",
      "timestamp": "2025-01-15T14:31:25Z",
      "severity": "critical",
      "type": "temperature",
      "message": "Temperature exceeded 75°C",
      "value": 76.8,
      "threshold": 75.0
    }
  ]
}
```

---

### Services

#### `GET /services`

List all monitored services

**Query Parameters:**
- `status` (optional): Filter by status (`up`, `down`, `degraded`)

**Response 200:**
```json
{
  "services": [
    {
      "id": "api-prod",
      "name": "Production API",
      "type": "http",
      "url": "https://api.example.com/health",
      "status": "up",
      "uptime_24h": 99.95,
      "uptime_7d": 99.87,
      "uptime_30d": 99.92,
      "last_check": "2025-01-15T14:32:01Z",
      "response_time_ms": 45.2,
      "response_time_p95_ms": 78.5
    }
  ]
}
```

#### `GET /services/:id`

Get detailed service information

**Response 200:**
```json
{
  "id": "api-prod",
  "name": "Production API",
  "type": "http",
  "url": "https://api.example.com/health",
  "status": "up",
  "last_check": {
    "timestamp": "2025-01-15T14:32:01Z",
    "status": "up",
    "response_time_ms": 45.2,
    "status_code": 200,
    "error": null
  },
  "uptime": {
    "last_24h": 99.95,
    "last_7d": 99.87,
    "last_30d": 99.92
  },
  "response_times": {
    "avg_ms": 48.3,
    "min_ms": 28.1,
    "max_ms": 125.7,
    "p50_ms": 45.2,
    "p95_ms": 78.5,
    "p99_ms": 105.3
  },
  "check_count": {
    "total": 2880,
    "failed": 2,
    "last_24h": 2880
  },
  "ssl": {
    "expires": "2025-06-15T00:00:00Z",
    "days_until_expiry": 152,
    "issuer": "Let's Encrypt"
  }
}
```

#### `GET /services/:id/checks`

Get check history for a service

**Query Parameters:**
- `start` (optional): Start time
- `end` (optional): End time
- `status` (optional): Filter by status
- `limit` (optional): Max results (default: 100)

**Response 200:**
```json
{
  "checks": [
    {
      "timestamp": "2025-01-15T14:32:01Z",
      "status": "up",
      "response_time_ms": 45.2,
      "status_code": 200,
      "error": null
    }
  ]
}
```

---

### Alerts

#### `GET /alerts`

Get alert history across all servers and services

**Query Parameters:**
- `severity` (optional): Filter by severity
- `type` (optional): Filter by type (`temperature`, `cpu_usage`, `service_down`)
- `server_id` (optional): Filter by server
- `service_id` (optional): Filter by service
- `start` (optional): Start time
- `end` (optional): End time
- `limit` (optional): Max results (default: 100)

**Response 200:**
```json
{
  "alerts": [
    {
      "id": "alert_123abc",
      "timestamp": "2025-01-15T14:31:25Z",
      "severity": "critical",
      "type": "temperature",
      "source_type": "server",
      "source_id": "192.168.1.100:3000",
      "message": "Temperature exceeded 75°C",
      "value": 76.8,
      "threshold": 75.0,
      "acknowledged": false
    }
  ]
}
```

#### `POST /alerts/:id/acknowledge`

Acknowledge an alert

**Request Body:**
```json
{
  "note": "Investigating high temperature issue"
}
```

**Response 200:**
```json
{
  "success": true,
  "alert_id": "alert_123abc",
  "acknowledged_at": "2025-01-15T14:35:00Z"
}
```

---

### Statistics

#### `GET /stats/overview`

Get system-wide statistics

**Response 200:**
```json
{
  "servers": {
    "total": 10,
    "up": 9,
    "down": 1,
    "unknown": 0
  },
  "services": {
    "total": 25,
    "up": 23,
    "down": 1,
    "degraded": 1
  },
  "alerts": {
    "last_24h": 5,
    "last_7d": 23,
    "last_30d": 87
  },
  "metrics": {
    "total_stored": 15823947,
    "storage_size_mb": 2048.5,
    "oldest_timestamp": "2024-12-15T00:00:00Z"
  }
}
```

---

## WebSocket API

### Connection

```
ws://localhost:8080/ws?token=<jwt_token>
```

### Authentication

Token can be provided as:
1. Query parameter: `?token=<jwt_token>`
2. First message after connection:
   ```json
   {
     "type": "auth",
     "token": "<jwt_token>"
   }
   ```

### Message Format

#### Client → Server

**Subscribe to metrics:**
```json
{
  "type": "subscribe",
  "channels": ["metrics", "services", "alerts"],
  "filters": {
    "server_ids": ["192.168.1.100:3000"],
    "service_ids": ["api-prod"]
  }
}
```

**Unsubscribe:**
```json
{
  "type": "unsubscribe",
  "channels": ["metrics"]
}
```

**Ping:**
```json
{
  "type": "ping"
}
```

#### Server → Client

**Pong:**
```json
{
  "type": "pong",
  "timestamp": "2025-01-15T14:32:05Z"
}
```

**Metric Event:**
```json
{
  "type": "metric",
  "server_id": "192.168.1.100:3000",
  "timestamp": "2025-01-15T14:32:05Z",
  "metrics": {
    "cpu_usage_avg": 45.2,
    "temperature_avg": 68.5,
    "memory_used_pct": 62.3,
    "memory_total_bytes": 34359738368,
    "memory_used_bytes": 21411201024
  }
}
```

**Service Check Event:**
```json
{
  "type": "service_check",
  "service_id": "api-prod",
  "timestamp": "2025-01-15T14:32:01Z",
  "status": "up",
  "response_time_ms": 45.2,
  "metadata": {
    "status_code": 200
  }
}
```

**Alert Event:**
```json
{
  "type": "alert",
  "id": "alert_123abc",
  "timestamp": "2025-01-15T14:31:25Z",
  "severity": "critical",
  "source_type": "server",
  "source_id": "192.168.1.100:3000",
  "message": "Temperature exceeded 75°C",
  "value": 76.8,
  "threshold": 75.0
}
```

**Error:**
```json
{
  "type": "error",
  "code": "INVALID_TOKEN",
  "message": "Authentication failed"
}
```

---

## Implementation

### Axum Server Setup

```rust
// src/api/mod.rs

use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path, Query},
    Json,
    response::{IntoResponse, Response},
    http::StatusCode,
};
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    trace::TraceLayer,
};

pub struct ApiServer {
    storage: Arc<dyn StorageBackend>,
    metric_broadcast: broadcast::Receiver<MetricEvent>,
}

impl ApiServer {
    pub async fn run(self, addr: &str) -> Result<()> {
        let app = Router::new()
            // Health
            .route("/health", get(health_handler))
            .route("/version", get(version_handler))

            // Servers
            .route("/api/v1/servers", get(list_servers))
            .route("/api/v1/servers/:id", get(get_server))
            .route("/api/v1/servers/:id/metrics", get(query_metrics))
            .route("/api/v1/servers/:id/alerts", get(get_server_alerts))

            // Services
            .route("/api/v1/services", get(list_services))
            .route("/api/v1/services/:id", get(get_service))
            .route("/api/v1/services/:id/checks", get(get_service_checks))

            // Alerts
            .route("/api/v1/alerts", get(list_alerts))
            .route("/api/v1/alerts/:id/acknowledge", post(acknowledge_alert))

            // Stats
            .route("/api/v1/stats/overview", get(stats_overview))

            // WebSocket
            .route("/ws", get(websocket_handler))

            // Swagger UI
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))

            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(CorsLayer::permissive())
            .with_state(Arc::new(self));

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
```

### Handler Example

```rust
// src/api/handlers/servers.rs

#[derive(Deserialize)]
struct QueryMetricsParams {
    metric_type: String,
    start: String,
    end: String,
    aggregation: Option<String>,
    limit: Option<usize>,
}

async fn query_metrics(
    State(api): State<Arc<ApiServer>>,
    Path(server_id): Path<String>,
    Query(params): Query<QueryMetricsParams>,
) -> Result<Json<MetricsResponse>, ApiError> {
    // Parse timestamps
    let start = parse_timestamp(&params.start)?;
    let end = parse_timestamp(&params.end)?;

    // Query storage
    let metrics = api.storage
        .query_range(
            &server_id,
            &params.metric_type,
            TimeRange { start, end },
        )
        .await?;

    // Apply aggregation if requested
    let data_points = match params.aggregation.as_deref() {
        Some("5m") => aggregate_metrics(&metrics, Duration::minutes(5)),
        Some("1h") => aggregate_metrics(&metrics, Duration::hours(1)),
        _ => metrics.into_iter().map(Into::into).collect(),
    };

    Ok(Json(MetricsResponse {
        server_id,
        metric_type: params.metric_type,
        aggregation: params.aggregation,
        data_points,
        metadata: MetricsMetadata {
            start,
            end,
            count: data_points.len(),
        },
    }))
}
```

### WebSocket Handler

```rust
// src/api/websocket.rs

use axum::extract::ws::{WebSocket, Message};
use futures::{StreamExt, SinkExt};

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(api): State<Arc<ApiServer>>,
    Query(params): Query<WebSocketParams>,
) -> impl IntoResponse {
    // Authenticate
    let token = params.token.ok_or(ApiError::Unauthorized)?;
    let user = verify_token(&token)?;

    ws.on_upgrade(move |socket| handle_socket(socket, api, user))
}

async fn handle_socket(mut socket: WebSocket, api: Arc<ApiServer>, user: User) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channels
    let mut metric_rx = api.metric_broadcast.subscribe();
    let mut service_rx = api.service_broadcast.subscribe();
    let mut alert_rx = api.alert_broadcast.subscribe();

    // Client message handler task
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match client_msg {
                        ClientMessage::Subscribe { channels, filters } => {
                            // Update subscription filters
                        }
                        ClientMessage::Ping => {
                            // Send pong
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // Broadcast forwarder task
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(metric) = metric_rx.recv() => {
                    let msg = ServerMessage::Metric(metric);
                    if let Ok(json) = serde_json::to_string(&msg) {
                        let _ = sender.send(Message::Text(json)).await;
                    }
                }
                Ok(service_check) = service_rx.recv() => {
                    let msg = ServerMessage::ServiceCheck(service_check);
                    if let Ok(json) = serde_json::to_string(&msg) {
                        let _ = sender.send(Message::Text(json)).await;
                    }
                }
                Ok(alert) = alert_rx.recv() => {
                    let msg = ServerMessage::Alert(alert);
                    if let Ok(json) = serde_json::to_string(&msg) {
                        let _ = sender.send(Message::Text(json)).await;
                    }
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = recv_task => {},
        _ = send_task => {},
    }
}
```

### Authentication

```rust
// src/api/auth.rs

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,  // User ID
    exp: usize,   // Expiration
    iat: usize,   // Issued at
    role: String, // User role
}

pub fn generate_token(user_id: &str, role: &str) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
        iat: Utc::now().timestamp() as usize,
        role: role.to_string(),
    };

    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn verify_token(token: &str) -> Result<Claims> {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}
```

---

## Error Handling

### Error Response Format

```json
{
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "message": "Server not found",
    "details": {
      "server_id": "192.168.1.100:3000"
    }
  },
  "request_id": "req_123abc"
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_REQUEST` | 400 | Malformed request |
| `UNAUTHORIZED` | 401 | Missing or invalid token |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `RESOURCE_NOT_FOUND` | 404 | Resource doesn't exist |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |
| `STORAGE_ERROR` | 503 | Database unavailable |

---

## Rate Limiting

```rust
// Implement using tower-governor or similar

use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

let governor_conf = Box::new(
    GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(20)
        .finish()
        .unwrap()
);

let governor_layer = GovernorLayer {
    config: Box::leak(governor_conf),
};

let app = Router::new()
    .route("/api/v1/servers", get(list_servers))
    .layer(governor_layer);
```

---

## API Documentation (OpenAPI)

```rust
// src/api/openapi.rs

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_handler,
        list_servers,
        get_server,
        query_metrics,
        // ... all endpoints
    ),
    components(
        schemas(ServerResponse, MetricsResponse, AlertResponse)
    ),
    tags(
        (name = "servers", description = "Server management endpoints"),
        (name = "services", description = "Service monitoring endpoints"),
        (name = "alerts", description = "Alert management endpoints"),
    ),
    info(
        title = "Guardia Monitoring API",
        version = "1.0.0",
        description = "Server and service monitoring API"
    )
)]
struct ApiDoc;
```

---

## Configuration

```json
{
  "api": {
    "bind_address": "0.0.0.0",
    "port": 8080,
    "jwt_secret": "${JWT_SECRET}",
    "cors": {
      "allowed_origins": ["http://localhost:3000"],
      "allowed_methods": ["GET", "POST", "PUT", "DELETE"],
      "allowed_headers": ["Authorization", "Content-Type"]
    },
    "rate_limit": {
      "requests_per_second": 10,
      "burst_size": 20
    },
    "websocket": {
      "ping_interval_secs": 30,
      "max_connections": 1000
    }
  }
}
```

---

## Security Considerations

1. **HTTPS in production:** Use reverse proxy (nginx, caddy) with TLS
2. **Token expiration:** Implement refresh token mechanism
3. **Input validation:** Validate all query parameters
4. **SQL injection:** Use parameterized queries
5. **DoS protection:** Rate limiting, connection limits
6. **Secrets management:** Never commit JWT_SECRET, use env vars

---

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_list_servers() {
        let app = create_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/servers")
                    .header("Authorization", "Bearer test_token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

---

## Performance Targets

- **REST endpoints:** <50ms p95 latency
- **WebSocket:** <10ms message delivery
- **Concurrent connections:** 1000+ WebSocket clients
- **Throughput:** 10,000 requests/sec

---

## Future Enhancements

- GraphQL endpoint as alternative to REST
- Server-Sent Events (SSE) for browsers
- gRPC API for high-performance integrations
- API versioning strategy (v2, v3)
- Batch operations (query multiple servers at once)
- Webhook management (register callbacks for events)

---

## Success Criteria

- [ ] All endpoints documented with OpenAPI
- [ ] Authentication working with JWT
- [ ] WebSocket streaming 100+ clients
- [ ] Rate limiting prevents abuse
- [ ] Error handling is consistent
- [ ] API tests have >80% coverage
