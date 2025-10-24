//! WebSocket client for real-time metric streaming

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest, http::Uri},
};

use crate::api::types::WsEvent;

/// WebSocket client for streaming events from the API
pub struct WebSocketClient {
    url: String,
    auth_token: Option<String>,
}

impl WebSocketClient {
    pub fn new(api_url: &str, auth_token: Option<String>) -> Self {
        // Convert http:// to ws:// and https:// to wss://
        let ws_url = api_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");

        Self {
            url: format!("{}/api/v1/stream", ws_url),
            auth_token,
        }
    }

    /// Connect to WebSocket and start streaming events
    pub async fn connect(self) -> Result<mpsc::UnboundedReceiver<WsEvent>> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Clone the URL for use in error handling
        let url = self.url.clone();

        // Spawn connection task
        tokio::spawn(async move {
            if let Err(e) = self.run(tx.clone()).await {
                tracing::error!("WebSocket connection error: {}", e);
                // Try to send an error message via the channel if possible
                // Use a simple format that won't break the TUI
                let error_msg = format!("WebSocket: {}", e);
                tx.send(WsEvent::ServiceCheck {
                    service_name: "Connection".to_string(),
                    url: url.clone(),
                    timestamp: chrono::Utc::now(),
                    status: crate::actors::messages::ServiceStatus::Down,
                    response_time_ms: None,
                    http_status_code: None,
                    ssl_expiry_days: None,
                    error_message: Some(error_msg),
                })
                .ok();
            }
        });

        Ok(rx)
    }

    async fn run(self, tx: mpsc::UnboundedSender<WsEvent>) -> Result<()> {
        loop {
            tracing::info!("Connecting to WebSocket: {}", self.url);

            match self.connect_once(&tx).await {
                Ok(_) => {
                    tracing::info!("WebSocket disconnected, reconnecting in 5s...");
                }
                Err(e) => {
                    // Provide detailed error context
                    let error_str = e.to_string().to_lowercase();
                    if error_str.contains("tls")
                        || error_str.contains("ssl")
                        || error_str.contains("certificate")
                    {
                        tracing::error!(
                            "WebSocket TLS/SSL error: {}. \
                            If using wss://, ensure TLS features are enabled in Cargo.toml. \
                            Reconnecting in 5s...",
                            e
                        );
                    } else {
                        tracing::error!("WebSocket connection error: {}. Reconnecting in 5s...", e);
                    }
                }
            }

            // Wait before reconnecting
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    async fn connect_once(&self, tx: &mpsc::UnboundedSender<WsEvent>) -> Result<()> {
        // Build URL with auth token if provided
        let url = if let Some(token) = &self.auth_token {
            format!("{}?token={}", self.url, token)
        } else {
            self.url.clone()
        };

        // Parse URL to extract host and scheme for headers
        let uri: Uri = url.parse().context("Failed to parse WebSocket URL")?;

        let host = uri
            .authority()
            .ok_or_else(|| anyhow::anyhow!("WebSocket URL missing host"))?
            .as_str();

        let scheme = uri
            .scheme_str()
            .ok_or_else(|| anyhow::anyhow!("WebSocket URL missing scheme"))?;

        // Build Origin header (wss -> https, ws -> http)
        let origin_scheme = if scheme == "wss" { "https" } else { "http" };
        let origin = format!("{}://{}", origin_scheme, host);

        // Save URL for logging (into_client_request consumes the url)
        let url_for_logging = url.clone();

        // Create WebSocket request using into_client_request() to preserve TLS/SNI configuration
        // This is critical for connecting through reverse proxies like Traefik with Let's Encrypt
        let mut request = url
            .into_client_request()
            .context("Failed to create WebSocket request")?;

        // Add custom headers for Traefik/proxy compatibility
        let headers = request.headers_mut();
        headers.insert(
            "Host",
            host.parse().context("Failed to parse Host header value")?,
        );
        headers.insert(
            "Origin",
            origin
                .parse()
                .context("Failed to parse Origin header value")?,
        );
        headers.insert(
            "User-Agent",
            "guardia-viewer/0.5.0"
                .parse()
                .context("Failed to parse User-Agent header value")?,
        );

        tracing::debug!(
            "Connecting to WebSocket: url={}, scheme={}, host={}, origin={}",
            url_for_logging,
            scheme,
            host,
            origin
        );

        let (ws_stream, _) = connect_async(request).await.with_context(|| {
            format!(
                "Failed to connect to WebSocket at {} (scheme: {}, host: {})",
                url_for_logging, scheme, host
            )
        })?;

        tracing::info!("WebSocket connected");

        // Send connection established event
        tx.send(WsEvent::ServiceCheck {
            service_name: "Connection".to_string(),
            url: url_for_logging.clone(),
            timestamp: chrono::Utc::now(),
            status: crate::actors::messages::ServiceStatus::Up,
            response_time_ms: None,
            http_status_code: None,
            ssl_expiry_days: None,
            error_message: None,
        })
        .ok();

        let (mut write, mut read) = ws_stream.split();

        // Send ping periodically to keep connection alive
        let ping_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                if write.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
        });

        // Read messages from WebSocket
        while let Some(msg) = read.next().await {
            let msg = msg.context("WebSocket message error")?;

            match msg {
                Message::Text(text) => {
                    // Parse and forward event
                    match serde_json::from_str::<WsEvent>(&text) {
                        Ok(event) => {
                            if tx.send(event).is_err() {
                                // Receiver dropped, exit
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to parse WebSocket event: {}\nRaw JSON: {}",
                                e,
                                text
                            );
                        }
                    }
                }
                Message::Close(_) => {
                    tracing::info!("WebSocket closed by server");
                    // Send connection lost event
                    tx.send(WsEvent::ServiceCheck {
                        service_name: "Connection".to_string(),
                        url: url_for_logging.clone(),
                        timestamp: chrono::Utc::now(),
                        status: crate::actors::messages::ServiceStatus::Down,
                        response_time_ms: None,
                        http_status_code: None,
                        ssl_expiry_days: None,
                        error_message: Some("Connection closed by server".to_string()),
                    })
                    .ok();
                    break;
                }
                Message::Pong(_) => {
                    // Ignore pong messages
                }
                _ => {
                    // Ignore other message types
                }
            }
        }

        ping_task.abort();

        // Send connection lost event for unexpected disconnections
        tx.send(WsEvent::ServiceCheck {
            service_name: "Connection".to_string(),
            url: url_for_logging.clone(),
            timestamp: chrono::Utc::now(),
            status: crate::actors::messages::ServiceStatus::Down,
            response_time_ms: None,
            http_status_code: None,
            ssl_expiry_days: None,
            error_message: Some("Connection lost unexpectedly".to_string()),
        })
        .ok();

        Ok(())
    }
}
