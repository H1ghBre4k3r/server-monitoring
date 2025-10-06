//! WebSocket client for real-time metric streaming

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

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
                }).ok();
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
                    tracing::error!("WebSocket error: {}, reconnecting in 5s...", e);
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

        let (ws_stream, _) = connect_async(&url)
            .await
            .context("Failed to connect to WebSocket")?;

        tracing::info!("WebSocket connected");

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
        Ok(())
    }
}
