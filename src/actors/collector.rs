//! MetricCollectorActor - Polls agent endpoints for metrics
//!
//! This actor replaces the old `server_monitor` loop with a cleaner actor-based design.
//!
//! ## Key Improvements over Old Design
//!
//! 1. **Reuses HTTP client** - Old code created new client on every request (inefficient)
//! 2. **Command-based control** - Can be controlled externally (poll now, update interval, shutdown)
//! 3. **Broadcast pattern** - Metrics published to channel, multiple consumers can subscribe
//! 4. **Testable** - Can inject mock channels and test in isolation
//!
//! ## Message Flow
//!
//! ```text
//! Timer tick → Poll agent → Parse metrics → Publish MetricEvent → [AlertActor, StorageActor, ...]
//!     ↑
//!     └─── Commands (PollNow, UpdateInterval, Shutdown)
//! ```

use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::interval;
use tracing::{debug, error, instrument, trace, warn};

use crate::{ServerMetrics, config::ServerConfig};

use super::messages::{CollectorCommand, MetricEvent};

/// Actor that polls a single server for metrics
///
/// Each server gets its own collector actor. The actor runs in an infinite loop,
/// polling at the configured interval and publishing metrics to a broadcast channel.
pub struct MetricCollectorActor {
    /// Server configuration
    config: ServerConfig,

    /// HTTP client (reused across requests for efficiency)
    client: reqwest::Client,

    /// Command receiver for control messages
    command_rx: mpsc::Receiver<CollectorCommand>,

    /// Broadcast sender for publishing metrics
    metric_tx: broadcast::Sender<MetricEvent>,

    /// Display name for logging
    display_name: String,

    /// Current polling interval
    interval_duration: Duration,
}

impl MetricCollectorActor {
    /// Create a new collector actor
    pub fn new(
        config: ServerConfig,
        command_rx: mpsc::Receiver<CollectorCommand>,
        metric_tx: broadcast::Sender<MetricEvent>,
    ) -> Self {
        let display_name = config
            .display
            .clone()
            .unwrap_or_else(|| format!("{}:{}", config.ip, config.port));

        let interval_duration = Duration::from_secs(config.interval as u64);

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            command_rx,
            metric_tx,
            display_name,
            interval_duration,
        }
    }

    /// Run the actor's main loop
    ///
    /// This is the entry point for the actor. It runs until:
    /// - A Shutdown command is received
    /// - The command channel is closed
    #[instrument(skip(self), fields(server = %self.display_name))]
    pub async fn run(mut self) {
        debug!("starting collector actor");

        let mut ticker = interval(self.interval_duration);

        loop {
            tokio::select! {
                // Timer tick - poll for metrics
                _ = ticker.tick() => {
                    if let Err(e) = self.poll_metrics().await {
                        error!("failed to poll metrics: {:#}", e);
                    }
                }

                // Handle commands
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        CollectorCommand::PollNow { respond_to } => {
                            debug!("received PollNow command");
                            let result = self.poll_metrics().await;
                            let _ = respond_to.send(result);
                        }

                        CollectorCommand::UpdateInterval { interval_secs } => {
                            debug!("updating interval to {interval_secs}s");
                            self.interval_duration = Duration::from_secs(interval_secs);
                            ticker = interval(self.interval_duration);
                        }

                        CollectorCommand::Shutdown => {
                            debug!("received shutdown command");
                            break;
                        }
                    }
                }

                // Command channel closed - exit
                else => {
                    warn!("command channel closed, shutting down");
                    break;
                }
            }
        }

        debug!("collector actor stopped");
    }

    /// Poll the agent endpoint for metrics
    ///
    /// This method:
    /// 1. Makes HTTP request to agent's /metrics endpoint
    /// 2. Parses the JSON response
    /// 3. Publishes a MetricEvent to the broadcast channel
    ///
    /// Errors are logged but do not crash the actor (retry on next interval).
    #[instrument(skip(self), fields(server = %self.display_name))]
    async fn poll_metrics(&self) -> Result<()> {
        let url = format!("http://{}:{}/metrics", self.config.ip, self.config.port);

        trace!("requesting metrics from {url}");

        // Build request with optional auth token
        let mut request = self.client.get(&url);

        if let Some(token) = &self.config.token {
            request = request.header("X-MONITORING-SECRET", token);
        }

        // Send request with timeout
        let response = request
            .send()
            .await
            .context("failed to send HTTP request")?;

        // Check status code
        if !response.status().is_success() {
            anyhow::bail!("HTTP error: {}", response.status());
        }

        // Parse response body
        let body = response
            .text()
            .await
            .context("failed to read response body")?;

        let metrics: ServerMetrics =
            serde_json::from_str(&body).context("failed to parse metrics JSON")?;

        trace!("successfully parsed metrics");

        // Create event
        let event = MetricEvent {
            server_id: format!("{}:{}", self.config.ip, self.config.port),
            metrics,
            timestamp: Utc::now(),
            display_name: self.display_name.clone(),
        };

        // Publish to broadcast channel
        // Note: We ignore send errors. It's OK if there are no subscribers.
        // The broadcast channel will also lag/drop messages for slow subscribers,
        // which is acceptable for real-time metrics.
        match self.metric_tx.send(event) {
            Ok(num_receivers) => {
                trace!("published metric event to {num_receivers} receivers");
            }
            Err(_) => {
                trace!("no receivers for metric event (this is OK)");
            }
        }

        Ok(())
    }
}

/// Handle for controlling a MetricCollectorActor
///
/// This handle provides a typed API for sending commands to the actor.
/// It can be cloned and shared across threads.
#[derive(Clone)]
pub struct CollectorHandle {
    /// Command sender
    sender: mpsc::Sender<CollectorCommand>,

    /// Server ID for identification
    pub server_id: String,

    /// Display name
    pub display_name: String,
}

impl CollectorHandle {
    /// Spawn a new collector actor
    ///
    /// This creates the actor, spawns it as a tokio task, and returns a handle.
    pub fn spawn(config: ServerConfig, metric_tx: broadcast::Sender<MetricEvent>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        let server_id = format!("{}:{}", config.ip, config.port);
        let display_name = config.display.clone().unwrap_or_else(|| server_id.clone());

        let actor = MetricCollectorActor::new(config, cmd_rx, metric_tx);

        tokio::spawn(actor.run());

        Self {
            sender: cmd_tx,
            server_id,
            display_name,
        }
    }

    /// Trigger an immediate poll
    ///
    /// This bypasses the interval timer and polls immediately.
    /// Useful for testing and manual refresh operations.
    pub async fn poll_now(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(CollectorCommand::PollNow { respond_to: tx })
            .await
            .context("failed to send PollNow command")?;

        rx.await.context("failed to receive response")??;
        Ok(())
    }

    /// Update the polling interval
    pub async fn update_interval(&self, interval_secs: u64) -> Result<()> {
        self.sender
            .send(CollectorCommand::UpdateInterval { interval_secs })
            .await
            .context("failed to send UpdateInterval command")?;
        Ok(())
    }

    /// Gracefully shut down the collector
    pub async fn shutdown(&self) -> Result<()> {
        self.sender
            .send(CollectorCommand::Shutdown)
            .await
            .context("failed to send Shutdown command")?;
        Ok(())
    }

    /// Get the server ID (ip:port format)
    pub fn server_id(&self) -> &str {
        &self.server_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn create_test_config(ip: &str, port: u16) -> ServerConfig {
        ServerConfig {
            ip: ip.parse().unwrap(),
            port,
            interval: 10,
            token: None,
            display: Some(format!("Test {ip}:{port}")),
            limits: None,
        }
    }

    #[tokio::test]
    async fn test_collector_handle_creation() {
        let config = create_test_config("127.0.0.1", 3000);
        let (metric_tx, _metric_rx) = broadcast::channel(16);

        let handle = CollectorHandle::spawn(config, metric_tx);

        assert_eq!(handle.server_id, "127.0.0.1:3000");
        assert_eq!(handle.display_name, "Test 127.0.0.1:3000");

        // Clean shutdown
        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_interval() {
        let config = create_test_config("127.0.0.1", 3000);
        let (metric_tx, _metric_rx) = broadcast::channel(16);
        let handle = CollectorHandle::spawn(config, metric_tx);

        // Should not error
        handle.update_interval(5).await.unwrap();

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_poll_now_unreachable_server() {
        // Test that poll_now returns error when server is unreachable
        let config = create_test_config("127.0.0.1", 9999); // Unlikely to be running
        let (metric_tx, _metric_rx) = broadcast::channel(16);
        let handle = CollectorHandle::spawn(config, metric_tx);

        // Poll should fail but not panic
        let result = handle.poll_now().await;
        assert!(result.is_err());

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_metrics_published_to_broadcast() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock the /metrics endpoint
        Mock::given(method("GET"))
            .and(path("/metrics"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "system": {
                    "name": "TestOS",
                    "kernel_version": "5.0",
                    "os_version": "Test",
                    "host_name": "test-host"
                },
                "memory": {
                    "total": 16000000000u64,
                    "used": 8000000000u64,
                    "total_swap": 4000000000u64,
                    "used_swap": 1000000000u64
                },
                "cpus": {
                    "total": 8,
                    "arch": "x86_64",
                    "average_usage": 45.5,
                    "cpus": []
                },
                "components": {
                    "average_temperature": 55.0,
                    "components": []
                }
            })))
            .mount(&mock_server)
            .await;

        // Create config pointing to mock server
        let mock_uri = mock_server.uri();
        let mock_url = url::Url::parse(&mock_uri).unwrap();
        let mut config = create_test_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());
        config.interval = 1;

        let (metric_tx, mut metric_rx) = broadcast::channel(16);
        let handle = CollectorHandle::spawn(config, metric_tx);

        // Trigger poll
        handle.poll_now().await.unwrap();

        // Should receive metric event
        let event = tokio::time::timeout(tokio::time::Duration::from_millis(500), metric_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(event.metrics.cpus.average_usage, 45.5);
        assert_eq!(event.metrics.components.average_temperature, Some(55.0));

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_http_404_error_handled() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock 404 response
        Mock::given(method("GET"))
            .and(path("/metrics"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let mock_uri = mock_server.uri();
        let mock_url = url::Url::parse(&mock_uri).unwrap();
        let config = create_test_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

        let (metric_tx, _metric_rx) = broadcast::channel(16);
        let handle = CollectorHandle::spawn(config, metric_tx);

        // Should return error but not panic
        let result = handle.poll_now().await;
        assert!(result.is_err());

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_invalid_json_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock invalid JSON response
        Mock::given(method("GET"))
            .and(path("/metrics"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let mock_uri = mock_server.uri();
        let mock_url = url::Url::parse(&mock_uri).unwrap();
        let config = create_test_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

        let (metric_tx, _metric_rx) = broadcast::channel(16);
        let handle = CollectorHandle::spawn(config, metric_tx);

        // Should return error for invalid JSON
        let result = handle.poll_now().await;
        assert!(result.is_err());

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_shutdown_stops_polling() {
        let config = create_test_config("127.0.0.1", 9999);
        let (metric_tx, _metric_rx) = broadcast::channel(16);
        let handle = CollectorHandle::spawn(config, metric_tx);

        // Shutdown immediately
        handle.shutdown().await.unwrap();

        // Try to poll - should fail because actor is shutdown
        let result = handle.poll_now().await;
        assert!(result.is_err(), "Poll should fail after shutdown");
    }

    #[tokio::test]
    async fn test_concurrent_poll_now_requests() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Track number of requests
        let request_count = Arc::new(AtomicUsize::new(0));
        let request_count_clone = request_count.clone();

        Mock::given(method("GET"))
            .and(path("/metrics"))
            .respond_with(move |_req: &wiremock::Request| {
                request_count_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "system": {},
                    "memory": {"total": 0, "used": 0, "total_swap": 0, "used_swap": 0},
                    "cpus": {"total": 1, "arch": "x86_64", "average_usage": 50.0, "cpus": []},
                    "components": {"average_temperature": null, "components": []}
                }))
            })
            .mount(&mock_server)
            .await;

        let mock_uri = mock_server.uri();
        let mock_url = url::Url::parse(&mock_uri).unwrap();
        let config = create_test_config(mock_url.host_str().unwrap(), mock_url.port().unwrap());

        let (metric_tx, _metric_rx) = broadcast::channel(16);
        let handle = CollectorHandle::spawn(config, metric_tx);

        // Send multiple concurrent poll requests
        let mut tasks = vec![];
        for _ in 0..5 {
            let handle_clone = handle.clone();
            tasks.push(tokio::spawn(async move { handle_clone.poll_now().await }));
        }

        // Wait for all to complete
        for task in tasks {
            let _ = task.await;
        }

        // Should have received at least 5 requests (may be slightly more due to timing)
        let count = request_count.load(Ordering::SeqCst);
        assert!(count >= 5, "Should have at least 5 requests, got {}", count);

        handle.shutdown().await.unwrap();
    }
}
