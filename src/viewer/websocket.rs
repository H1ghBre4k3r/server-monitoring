//! WebSocket client for real-time metric streaming

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{ServerMetrics, actors::messages::ServiceStatus};

/// WebSocket event from the API server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    Metric {
        server_id: String,
        display_name: String,
        metrics: ServerMetrics,
        timestamp: DateTime<Utc>,
    },
    ServiceCheck {
        service_name: String,
        url: String,
        timestamp: DateTime<Utc>,
        status: ServiceStatus,
        response_time_ms: Option<u64>,
        http_status_code: Option<u16>,
        ssl_expiry_days: Option<i64>,
        error_message: Option<String>,
    },
}

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

        // Spawn connection task
        tokio::spawn(async move {
            if let Err(e) = self.run(tx.clone()).await {
                tracing::error!("WebSocket connection error: {}", e);
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
