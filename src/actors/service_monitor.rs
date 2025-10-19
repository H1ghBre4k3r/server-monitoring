//! ServiceMonitorActor - Monitors HTTP/HTTPS service endpoints
//!
//! This actor performs periodic health checks on configured services.
//!
//! ## Key Features
//!
//! 1. **HTTP/HTTPS support** - Can check any HTTP or HTTPS endpoint
//! 2. **Configurable checks** - Method, status codes, body pattern matching
//! 3. **Response time tracking** - Measures and reports response times
//! 4. **Broadcast pattern** - Publishes ServiceCheckEvent to multiple consumers
//!
//! ## Message Flow
//!
//! ```text
//! Timer tick → HTTP check → Validate response → Publish ServiceCheckEvent → [AlertActor, StorageActor, ...]
//!     ↑
//!     └─── Commands (CheckNow, UpdateInterval, Shutdown)
//! ```

use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::interval;
use tracing::{debug, error, instrument, trace, warn};

use crate::config::{HttpMethod, ResolvedServiceConfig};

use super::messages::{ServiceCheckEvent, ServiceCommand, ServiceStatus};

/// Actor that monitors a single service endpoint
///
/// Each service gets its own monitor actor. The actor runs in an infinite loop,
/// checking the service at the configured interval and publishing results to a broadcast channel.
pub struct ServiceMonitorActor {
    /// Service configuration
    config: ResolvedServiceConfig,

    /// HTTP client (reused across requests for efficiency)
    client: reqwest::Client,

    /// Command receiver for control messages
    command_rx: mpsc::Receiver<ServiceCommand>,

    /// Broadcast sender for publishing check results
    event_tx: broadcast::Sender<ServiceCheckEvent>,

    /// Current check interval
    interval_duration: Duration,
}

impl ServiceMonitorActor {
    /// Create a new service monitor actor
    pub fn new(
        config: ResolvedServiceConfig,
        command_rx: mpsc::Receiver<ServiceCommand>,
        event_tx: broadcast::Sender<ServiceCheckEvent>,
    ) -> Self {
        let interval_duration = Duration::from_secs(config.interval as u64);
        let timeout = Duration::from_secs(config.timeout as u64);

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("Failed to build HTTP client"),
            command_rx,
            event_tx,
            interval_duration,
        }
    }

    /// Run the actor's main loop
    ///
    /// This is the entry point for the actor. It runs until:
    /// - A Shutdown command is received
    /// - The command channel is closed
    #[instrument(skip(self), fields(service = %self.config.name))]
    pub async fn run(mut self) {
        debug!("starting service monitor actor");

        let mut ticker = interval(self.interval_duration);

        loop {
            tokio::select! {
                // Timer tick - perform health check
                _ = ticker.tick() => {
                    if let Err(e) = self.perform_check().await {
                        error!("health check failed: {:#}", e);
                    }
                }

                // Handle commands
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        ServiceCommand::CheckNow { respond_to } => {
                            debug!("received CheckNow command");
                            let result = self.perform_check().await;
                            let _ = respond_to.send(result);
                        }

                        ServiceCommand::UpdateInterval { interval_secs } => {
                            debug!("updating interval to {interval_secs}s");
                            self.interval_duration = Duration::from_secs(interval_secs);
                            ticker = interval(self.interval_duration);
                        }

                        ServiceCommand::Shutdown => {
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

        debug!("service monitor actor stopped");
    }

    /// Perform a health check on the service
    ///
    /// This method:
    /// 1. Makes HTTP request to the configured URL
    /// 2. Validates response (status code, body pattern)
    /// 3. Measures response time
    /// 4. Publishes a ServiceCheckEvent to the broadcast channel
    ///
    /// Errors are captured in the event (service marked as Down).
    #[instrument(skip(self), fields(service = %self.config.name))]
    async fn perform_check(&self) -> Result<()> {
        trace!("checking service at {}", self.config.url);

        let start = std::time::Instant::now();

        // Perform HTTP request
        let check_result = self.execute_request().await;
        let response_time_ms = start.elapsed().as_millis() as u64;

        // Create event based on result
        let event = match check_result {
            Ok((status_code, body)) => {
                let status = self.evaluate_response(status_code, &body);
                ServiceCheckEvent {
                    service_name: self.config.name.clone(),
                    url: self.config.url.clone(),
                    timestamp: Utc::now(),
                    status,
                    response_time_ms: Some(response_time_ms),
                    http_status_code: Some(status_code),
                    ssl_expiry_days: None, // TODO: Implement SSL cert checking
                    error_message: if status != ServiceStatus::Up {
                        Some(format!("Unexpected status code: {}", status_code))
                    } else {
                        None
                    },
                }
            }
            Err(e) => {
                warn!("service check failed: {:#}", e);
                ServiceCheckEvent {
                    service_name: self.config.name.clone(),
                    url: self.config.url.clone(),
                    timestamp: Utc::now(),
                    status: ServiceStatus::Down,
                    response_time_ms: None,
                    http_status_code: None,
                    ssl_expiry_days: None,
                    error_message: Some(e.to_string()),
                }
            }
        };

        // Publish event
        if let Err(e) = self.event_tx.send(event) {
            error!("failed to publish service check event: {}", e);
        }

        Ok(())
    }

    /// Execute the HTTP request
    ///
    /// Returns (status_code, body) on success
    async fn execute_request(&self) -> Result<(u16, String)> {
        let method = match self.config.method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Head => reqwest::Method::HEAD,
        };

        let response = self
            .client
            .request(method, &self.config.url)
            .send()
            .await
            .context("HTTP request failed")?;

        let status_code = response.status().as_u16();

        // Get body (skip for HEAD requests)
        let body = if matches!(self.config.method, HttpMethod::Head) {
            String::new()
        } else {
            response
                .text()
                .await
                .context("Failed to read response body")?
        };

        Ok((status_code, body))
    }

    /// Evaluate the response to determine service status
    ///
    /// Checks:
    /// 1. Status code matches expected codes (or is 2xx if not specified)
    /// 2. Body matches pattern (if configured)
    fn evaluate_response(&self, status_code: u16, body: &str) -> ServiceStatus {
        // Check status code
        let status_ok = if let Some(ref expected) = self.config.expected_status {
            expected.contains(&status_code)
        } else {
            // Default: any 2xx status is success
            (200..300).contains(&status_code)
        };

        if !status_ok {
            return ServiceStatus::Down;
        }

        // Check body pattern if configured
        if let Some(ref pattern) = self.config.body_pattern {
            match regex::Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(body) {
                        return ServiceStatus::Degraded;
                    }
                }
                Err(e) => {
                    error!("invalid regex pattern '{}': {}", pattern, e);
                    return ServiceStatus::Degraded;
                }
            }
        }

        ServiceStatus::Up
    }
}

/// Handle for controlling a ServiceMonitorActor
#[derive(Clone)]
pub struct ServiceHandle {
    sender: mpsc::Sender<ServiceCommand>,
    service_name: String,
    service_url: String,
}

impl ServiceHandle {
    /// Spawn a new service monitor actor
    pub fn spawn(config: ResolvedServiceConfig, event_tx: broadcast::Sender<ServiceCheckEvent>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let service_name = config.name.clone();
        let service_url = config.url.clone();

        let actor = ServiceMonitorActor::new(config, cmd_rx, event_tx);

        tokio::spawn(actor.run());

        Self {
            sender: cmd_tx,
            service_name,
            service_url,
        }
    }

    /// Trigger an immediate health check
    pub async fn check_now(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ServiceCommand::CheckNow { respond_to: tx })
            .await?;

        rx.await??;
        Ok(())
    }

    /// Update the check interval
    pub async fn update_interval(&self, interval_secs: u64) -> Result<()> {
        self.sender
            .send(ServiceCommand::UpdateInterval { interval_secs })
            .await?;
        Ok(())
    }

    /// Shut down the service monitor
    pub async fn shutdown(self) {
        let _ = self.sender.send(ServiceCommand::Shutdown).await;
    }

    /// Get the service name
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the service URL
    pub fn service_url(&self) -> &str {
        &self.service_url
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_service_handle_creation() {
        let (event_tx, _) = broadcast::channel(16);

        let config = ResolvedServiceConfig {
            name: "test-service".to_string(),
            url: "http://example.com".to_string(),
            interval: 60,
            timeout: 10,
            method: HttpMethod::Get,
            expected_status: None,
            body_pattern: None,
            grace: None,
            alert: None,
        };

        let handle = ServiceHandle::spawn(config, event_tx);

        assert_eq!(handle.service_name(), "test-service");

        // Cleanup
        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_update_interval() {
        let (event_tx, _) = broadcast::channel(16);

        let config = ResolvedServiceConfig {
            name: "test-service".to_string(),
            url: "http://example.com".to_string(),
            interval: 60,
            timeout: 10,
            method: HttpMethod::Get,
            expected_status: None,
            body_pattern: None,
            grace: None,
            alert: None,
        };

        let handle = ServiceHandle::spawn(config, event_tx);

        // Should not panic
        handle.update_interval(30).await.unwrap();

        // Cleanup
        handle.shutdown().await;
    }
}
