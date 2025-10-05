//! WebSocket handler for real-time metric streaming

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures::{SinkExt, stream::StreamExt};
use tracing::{debug, info};

use crate::{actors::messages::MetricEvent, api::state::ApiState};

/// WebSocket upgrade handler
///
/// GET /api/v1/stream
pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<ApiState>) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

/// Handle WebSocket connection
async fn handle_websocket(socket: WebSocket, state: ApiState) {
    info!("WebSocket client connected");

    let (mut sender, mut receiver) = socket.split();

    // Subscribe to metric and service check events
    let mut metric_rx = state.metric_tx.subscribe();
    let mut service_rx = state.service_check_tx.subscribe();

    // Spawn task to forward events to WebSocket
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Forward metric events
                Ok(MetricEvent { server_id, metrics, timestamp, display_name }) = metric_rx.recv() => {
                    let json = serde_json::json!({
                        "type": "metric",
                        "server_id": server_id,
                        "display_name": display_name,
                        "timestamp": timestamp.to_rfc3339(),
                        "metrics": metrics,
                    });

                    if let Ok(text) = serde_json::to_string(&json)
                        && sender.send(Message::Text(text)).await.is_err() {
                            debug!("WebSocket send failed, client disconnected");
                            break;
                        }
                }

                // Forward service check events
                Ok(event) = service_rx.recv() => {
                    let json = serde_json::json!({
                        "type": "service_check",
                        "service_name": event.service_name,
                        "url": event.url,
                        "timestamp": event.timestamp.to_rfc3339(),
                        "status": event.status,
                        "response_time_ms": event.response_time_ms,
                        "http_status_code": event.http_status_code,
                        "error_message": event.error_message,
                    });

                    if let Ok(text) = serde_json::to_string(&json)
                        && sender.send(Message::Text(text)).await.is_err() {
                            debug!("WebSocket send failed, client disconnected");
                            break;
                        }
                }

                else => {
                    debug!("Broadcast channel closed");
                    break;
                }
            }
        }
    });

    // Handle incoming messages (for future use - could be used for subscriptions)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(data) => {
                    // Pong is automatically sent by axum
                    debug!("Received ping");
                }
                _ => {
                    // Ignore other message types for now
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }

    info!("WebSocket client disconnected");
}
