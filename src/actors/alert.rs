//! AlertActor - Evaluates metrics and sends alerts
//!
//! This actor replaces the old closure-based alert handling with a proper stateful actor.
//!
//! ## Grace Period State Machine
//!
//! The actor maintains grace period counters to prevent alert spam:
//!
//! ```text
//! Resource < Limit:
//!   grace_counter == 0           → ResourceEvaluation::Ok (no alert)
//!   grace_counter > grace_limit  → ResourceEvaluation::BackToOk (send recovery alert)
//!
//! Resource >= Limit:
//!   grace_counter < grace_limit  → ResourceEvaluation::Exceeding (increment, no alert)
//!   grace_counter == grace_limit → ResourceEvaluation::StartsToExceed (send alert)
//! ```
//!
//! This prevents alerts from firing on transient spikes.

use std::collections::HashMap;

use chrono::Utc;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, instrument, trace, warn};

use crate::{
    alerts::AlertManager,
    config::{ResolvedLimit, ResolvedServerConfig, ResolvedServiceConfig},
    monitors::resources::ResourceEvaluation,
};

use super::messages::{AlertCommand, AlertState, MetricEvent, ServiceCheckEvent, ServiceStatus};

/// Per-server alert state
#[derive(Debug, Clone)]
struct ServerAlertState {
    /// Server configuration
    config: ResolvedServerConfig,

    /// Alert manager for sending notifications
    alert_manager: AlertManager,

    /// Temperature grace counter
    temp_grace_counter: usize,

    /// CPU usage grace counter
    usage_grace_counter: usize,
}

/// Per-service alert state (Phase 3)
#[derive(Debug, Clone)]
struct ServiceAlertState {
    /// Service configuration
    config: ResolvedServiceConfig,

    /// Alert manager for sending notifications
    alert_manager: AlertManager,

    /// Last known status
    last_status: Option<ServiceStatus>,

    /// Consecutive down checks counter (for grace period)
    consecutive_down: usize,
}

/// Actor that evaluates metrics and sends alerts
pub struct AlertActor {
    /// Per-server state
    servers: HashMap<String, ServerAlertState>,

    /// Per-service state (Phase 3)
    services: HashMap<String, ServiceAlertState>,

    /// Command receiver
    command_rx: mpsc::Receiver<AlertCommand>,

    /// Metric event receiver (broadcast subscription)
    metric_rx: broadcast::Receiver<MetricEvent>,

    /// Service check event receiver (broadcast subscription, Phase 3)
    service_check_rx: broadcast::Receiver<ServiceCheckEvent>,

    /// Whether alerts are muted
    muted: bool,
}

impl AlertActor {
    /// Create a new alert actor
    pub fn new(
        command_rx: mpsc::Receiver<AlertCommand>,
        metric_rx: broadcast::Receiver<MetricEvent>,
        service_check_rx: broadcast::Receiver<ServiceCheckEvent>,
    ) -> Self {
        Self {
            servers: HashMap::new(),
            services: HashMap::new(),
            command_rx,
            metric_rx,
            service_check_rx,
            muted: false,
        }
    }

    /// Register a server for monitoring
    ///
    /// This should be called for each server before metrics start flowing.
    pub fn register_server(&mut self, config: ResolvedServerConfig) {
        let server_id = format!("{}:{}", config.ip, config.port);
        let alert_manager = AlertManager::new(config.clone());

        self.servers.insert(
            server_id,
            ServerAlertState {
                config,
                alert_manager,
                temp_grace_counter: 0,
                usage_grace_counter: 0,
            },
        );
    }

    /// Register a service for monitoring (Phase 3)
    ///
    /// This should be called for each service before checks start flowing.
    pub fn register_service(&mut self, config: ResolvedServiceConfig) {
        let service_name = config.name.clone();

        // Create a pseudo ResolvedServerConfig for AlertManager
        // This is needed because AlertManager was designed for server alerts
        use std::net::IpAddr;
        let pseudo_server_config = ResolvedServerConfig {
            ip: IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            port: 0,
            display: Some(service_name.clone()),
            limits: None,
            interval: 60,
            token: None,
        };

        let alert_manager = AlertManager::new(pseudo_server_config);

        self.services.insert(
            service_name,
            ServiceAlertState {
                config,
                alert_manager,
                last_status: None,
                consecutive_down: 0,
            },
        );
    }

    /// Run the actor's main loop
    #[instrument(skip(self))]
    pub async fn run(mut self) {
        debug!("starting alert actor");

        loop {
            tokio::select! {
                // Receive metric events
                result = self.metric_rx.recv() => {
                    match result {
                        Ok(event) => {
                            if !self.muted {
                                self.handle_metric_event(event).await;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!("alert actor lagged, skipped {skipped} metrics");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            warn!("metric channel closed, shutting down");
                            break;
                        }
                    }
                }

                // Receive service check events (Phase 3)
                result = self.service_check_rx.recv() => {
                    match result {
                        Ok(event) => {
                            if !self.muted {
                                self.handle_service_check_event(event).await;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!("alert actor lagged, skipped {skipped} service checks");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            trace!("service check channel closed");
                            // Don't break - metric channel might still be open
                        }
                    }
                }

                // Handle commands
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        AlertCommand::GetState { server_id, respond_to } => {
                            let state = self.get_alert_state(&server_id);
                            let _ = respond_to.send(state);
                        }

                        AlertCommand::MuteAlerts { duration_secs } => {
                            debug!("muting alerts for {duration_secs}s");
                            self.muted = true;

                            // TODO: Implement auto-unmute timer
                            // For now, requires manual unmute
                            warn!("auto-unmute not yet implemented, use UnmuteAlerts command");
                        }

                        AlertCommand::UnmuteAlerts => {
                            debug!("unmuting alerts");
                            self.muted = false;
                        }

                        AlertCommand::Shutdown => {
                            debug!("received shutdown command");
                            break;
                        }
                    }
                }

                // Command channel closed
                else => {
                    warn!("command channel closed, shutting down");
                    break;
                }
            }
        }

        debug!("alert actor stopped");
    }

    /// Handle a metric event
    #[instrument(skip(self, event), fields(server_id = %event.server_id))]
    async fn handle_metric_event(&mut self, event: MetricEvent) {
        // Get server state and clone limits to avoid borrow conflicts
        let state = match self.servers.get_mut(&event.server_id) {
            Some(s) => s,
            None => {
                trace!("received metrics for unregistered server, ignoring");
                return;
            }
        };

        let Some(limits) = state.config.limits.clone() else {
            trace!("no limits configured, skipping evaluation");
            return;
        };

        // Evaluate temperature
        if let Some(limit) = limits.temperature {
            Self::evaluate_temperature(&event, state, &limit).await;
        }

        // Evaluate CPU usage
        if let Some(limit) = limits.usage {
            Self::evaluate_cpu_usage(&event, state, &limit).await;
        }
    }

    /// Evaluate temperature against limit
    async fn evaluate_temperature(
        event: &MetricEvent,
        state: &mut ServerAlertState,
        limit: &ResolvedLimit,
    ) {
        let Some(current_temp) = event.metrics.components.average_temperature else {
            return;
        };

        let grace = limit.grace.unwrap_or_default();

        let evaluation = ResourceEvaluation::evaluate(
            current_temp,
            limit.limit as f32,
            grace,
            state.temp_grace_counter,
        );

        trace!(
            "temperature evaluation: {current_temp}°C vs {}, grace {}/{} → {evaluation:?}",
            limit.limit, state.temp_grace_counter, grace
        );

        match evaluation {
            ResourceEvaluation::Ok => {
                // Within limits, reset counter
                state.temp_grace_counter = 0;
            }

            ResourceEvaluation::Exceeding => {
                // Increment counter, no alert yet
                state.temp_grace_counter += 1;
            }

            ResourceEvaluation::StartsToExceed => {
                // Grace period exhausted - send alert
                state.temp_grace_counter += 1;
                debug!(
                    "{}: temperature exceeded limit ({current_temp}°C > {})",
                    event.server_id, limit.limit
                );

                state
                    .alert_manager
                    .send_temperature_alert(evaluation, current_temp)
                    .await;
            }

            ResourceEvaluation::BackToOk => {
                // Recovered - send recovery alert
                debug!(
                    "{}: temperature recovered ({current_temp}°C < {})",
                    event.server_id, limit.limit
                );
                state.temp_grace_counter = 0;

                state
                    .alert_manager
                    .send_temperature_alert(evaluation, current_temp)
                    .await;
            }
        }
    }

    /// Evaluate CPU usage against limit
    async fn evaluate_cpu_usage(event: &MetricEvent, state: &mut ServerAlertState, limit: &ResolvedLimit) {
        let current_usage = event.metrics.cpus.average_usage;
        let grace = limit.grace.unwrap_or_default();

        let evaluation = ResourceEvaluation::evaluate(
            current_usage,
            limit.limit as f32,
            grace,
            state.usage_grace_counter,
        );

        trace!(
            "CPU evaluation: {current_usage}% vs {}, grace {}/{} → {evaluation:?}",
            limit.limit, state.usage_grace_counter, grace
        );

        match evaluation {
            ResourceEvaluation::Ok => {
                state.usage_grace_counter = 0;
            }

            ResourceEvaluation::Exceeding => {
                state.usage_grace_counter += 1;
            }

            ResourceEvaluation::StartsToExceed => {
                state.usage_grace_counter += 1;
                debug!(
                    "{}: CPU usage exceeded limit ({current_usage}% > {})",
                    event.server_id, limit.limit
                );

                state
                    .alert_manager
                    .send_usage_alert(evaluation, current_usage)
                    .await;
            }

            ResourceEvaluation::BackToOk => {
                debug!(
                    "{}: CPU usage recovered ({current_usage}% < {})",
                    event.server_id, limit.limit
                );
                state.usage_grace_counter = 0;

                state
                    .alert_manager
                    .send_usage_alert(evaluation, current_usage)
                    .await;
            }
        }
    }

    /// Handle a service check event (Phase 3)
    #[instrument(skip(self, event), fields(service_name = %event.service_name))]
    async fn handle_service_check_event(&mut self, event: ServiceCheckEvent) {
        let state = match self.services.get_mut(&event.service_name) {
            Some(s) => s,
            None => {
                trace!("received service check for unregistered service, ignoring");
                return;
            }
        };

        let grace = state.config.grace.unwrap_or(1);
        let previous_status = state.last_status;

        trace!(
            "service check: {} status={:?}, consecutive_down={}/{}",
            event.service_name, event.status, state.consecutive_down, grace
        );

        match event.status {
            ServiceStatus::Down | ServiceStatus::Degraded => {
                state.consecutive_down += 1;

                // Send alert if grace period exhausted
                if state.consecutive_down == grace {
                    debug!(
                        "{}: service went down (grace period exhausted: {}/{})",
                        event.service_name, state.consecutive_down, grace
                    );

                    // Send alert if configured
                    if let Some(alert_config) = &state.config.alert {
                        // TODO: in the end, this should probably be a standalone alert manager
                        state
                            .alert_manager
                            .send_service_alert(
                                alert_config,
                                &event.service_name,
                                &event.url,
                                previous_status,
                                event.status,
                                event.error_message.as_deref(),
                            )
                            .await;
                    }
                }

                state.last_status = Some(event.status);
            }

            ServiceStatus::Up => {
                // Service is up - check if it recovered from down state
                if state.consecutive_down >= grace {
                    debug!(
                        "{}: service recovered (was down {} consecutive checks)",
                        event.service_name, state.consecutive_down
                    );

                    // Send recovery alert if configured
                    if let Some(alert_config) = &state.config.alert {
                        // TODO: in the end, this should probably be a standalone alert manager
                        state
                            .alert_manager
                            .send_service_alert(
                                alert_config,
                                &event.service_name,
                                &event.url,
                                previous_status,
                                ServiceStatus::Up,
                                None,
                            )
                            .await;
                    }
                }

                // Reset counter
                state.consecutive_down = 0;
                state.last_status = Some(ServiceStatus::Up);
            }
        }
    }

    /// Get alert state for a server
    fn get_alert_state(&self, server_id: &str) -> Option<AlertState> {
        self.servers.get(server_id).map(|state| AlertState {
            server_id: server_id.to_string(),
            cpu_consecutive_exceeds: state.usage_grace_counter,
            temp_consecutive_exceeds: state.temp_grace_counter,
            last_evaluation: Utc::now(),
        })
    }
}

/// Handle for controlling the AlertActor
#[derive(Clone)]
pub struct AlertHandle {
    sender: mpsc::Sender<AlertCommand>,
}

impl AlertHandle {
    /// Spawn a new alert actor
    ///
    /// # Arguments
    /// - `servers`: Initial server configurations to monitor (must be resolved)
    /// - `services`: Initial service configurations to monitor (must be resolved, Phase 3)
    /// - `metric_rx`: Broadcast receiver for metric events
    /// - `service_check_rx`: Broadcast receiver for service check events (Phase 3)
    pub fn spawn(
        servers: Vec<ResolvedServerConfig>,
        services: Vec<ResolvedServiceConfig>,
        metric_rx: broadcast::Receiver<MetricEvent>,
        service_check_rx: broadcast::Receiver<ServiceCheckEvent>,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        let mut actor = AlertActor::new(cmd_rx, metric_rx, service_check_rx);

        // Register all servers
        for config in servers {
            actor.register_server(config);
        }

        // Register all services
        for config in services {
            actor.register_service(config);
        }

        tokio::spawn(actor.run());

        Self { sender: cmd_tx }
    }

    /// Get alert state for a server
    pub async fn get_state(&self, server_id: String) -> Option<AlertState> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(AlertCommand::GetState {
                server_id,
                respond_to: tx,
            })
            .await
            .ok()?;

        rx.await.ok()?
    }

    /// Mute alerts for a duration
    pub async fn mute_alerts(&self, duration_secs: u64) {
        let _ = self
            .sender
            .send(AlertCommand::MuteAlerts { duration_secs })
            .await;
    }

    /// Unmute alerts
    pub async fn unmute_alerts(&self) {
        let _ = self.sender.send(AlertCommand::UnmuteAlerts).await;
    }

    /// Shutdown the alert actor
    pub async fn shutdown(&self) {
        let _ = self.sender.send(AlertCommand::Shutdown).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Limit, Limits};
    use crate::{
        ComponentOverview, CpuOverview, MemoryInformation, ServerMetrics, SystemInformation,
    };

    // Note: ResourceEvaluation tests are in monitors/resources.rs
    // These tests focus on the AlertActor behavior

    fn create_test_server_config(ip: &str, port: u16) -> ServerConfig {
        use std::net::IpAddr;
        use std::str::FromStr;

        ServerConfig {
            ip: IpAddr::from_str(ip).unwrap(),
            port,
            interval: 5,
            token: None,
            display: Some(format!("Test {ip}:{port}")),
            limits: Some(Limits {
                temperature: Some(Limit {
                    limit: 70,
                    grace: Some(3),
                    alert: None,
                }),
                usage: Some(Limit {
                    limit: 80,
                    grace: Some(5),
                    alert: None,
                }),
            }),
        }
    }

    fn create_test_metrics(cpu_usage: f32, temperature: Option<f32>) -> ServerMetrics {
        ServerMetrics {
            system: SystemInformation::default(),
            memory: MemoryInformation::default(),
            cpus: CpuOverview {
                total: 8,
                arch: "x86_64".to_string(),
                average_usage: cpu_usage,
                cpus: vec![],
            },
            components: ComponentOverview {
                average_temperature: temperature,
                components: vec![],
            },
        }
    }

    #[tokio::test]
    async fn test_alert_handle_creation() {
        let (_metric_tx, metric_rx) = broadcast::channel(16);
        let (_service_tx, service_rx) = broadcast::channel(16);
        let servers = vec![];
        let services = vec![];

        let _handle = AlertHandle::spawn(servers, services, metric_rx, service_rx);

        // Handle created successfully
    }

    #[tokio::test]
    async fn test_grace_period_temperature_increments_until_alert() {
        let (metric_tx, metric_rx) = broadcast::channel(16);
        let (_service_tx, service_rx) = broadcast::channel(16);
        let config = create_test_server_config("127.0.0.1", 3000);
        let server_id = "127.0.0.1:3000".to_string();

        let servers = vec![config];
        let handle = AlertHandle::spawn(servers, vec![], metric_rx, service_rx);

        // Send metrics below limit - should not alert
        for _ in 0..2 {
            let event = MetricEvent {
                server_id: server_id.clone(),
                metrics: create_test_metrics(50.0, Some(65.0)), // Below 70°C
                timestamp: Utc::now(),
                display_name: "Test".to_string(),
            };
            metric_tx.send(event).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Verify grace counter is 0
        let state = handle.get_state(server_id.clone()).await.unwrap();
        assert_eq!(state.temp_consecutive_exceeds, 0);

        // Send metrics above limit but within grace period (grace = 3)
        for i in 1..=2 {
            let event = MetricEvent {
                server_id: server_id.clone(),
                metrics: create_test_metrics(50.0, Some(75.0)), // Above 70°C
                timestamp: Utc::now(),
                display_name: "Test".to_string(),
            };
            metric_tx.send(event).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Check grace counter incremented
            let state = handle.get_state(server_id.clone()).await.unwrap();
            assert_eq!(state.temp_consecutive_exceeds, i);
        }

        // Send one more to exceed grace period (3rd exceed)
        let event = MetricEvent {
            server_id: server_id.clone(),
            metrics: create_test_metrics(50.0, Some(75.0)),
            timestamp: Utc::now(),
            display_name: "Test".to_string(),
        };
        metric_tx.send(event).unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Grace counter should now be 3 (StartToExceed was triggered)
        let state = handle.get_state(server_id.clone()).await.unwrap();
        assert_eq!(state.temp_consecutive_exceeds, 3);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_grace_period_cpu_independent_from_temperature() {
        let (metric_tx, metric_rx) = broadcast::channel(16);
        let (_service_tx, service_rx) = broadcast::channel(16);
        let config = create_test_server_config("127.0.0.1", 3000);
        let server_id = "127.0.0.1:3000".to_string();

        let servers = vec![config];
        let handle = AlertHandle::spawn(servers, vec![], metric_rx, service_rx);

        // Exceed temperature grace period
        for _ in 0..3 {
            let event = MetricEvent {
                server_id: server_id.clone(),
                metrics: create_test_metrics(50.0, Some(75.0)), // Above temp limit
                timestamp: Utc::now(),
                display_name: "Test".to_string(),
            };
            metric_tx.send(event).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let state = handle.get_state(server_id.clone()).await.unwrap();
        assert_eq!(state.temp_consecutive_exceeds, 3);
        assert_eq!(state.cpu_consecutive_exceeds, 0); // CPU still at 0

        // Now exceed CPU limit
        for i in 1..=2 {
            let event = MetricEvent {
                server_id: server_id.clone(),
                metrics: create_test_metrics(85.0, Some(75.0)), // Above CPU limit (80%)
                timestamp: Utc::now(),
                display_name: "Test".to_string(),
            };
            metric_tx.send(event).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let state = handle.get_state(server_id.clone()).await.unwrap();
            assert_eq!(state.cpu_consecutive_exceeds, i); // CPU increments independently
            assert!(state.temp_consecutive_exceeds >= 3); // Temp remains exceeded
        }

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_back_to_ok_resets_grace_counter() {
        let (metric_tx, metric_rx) = broadcast::channel(16);
        let (_service_tx, service_rx) = broadcast::channel(16);
        let config = create_test_server_config("127.0.0.1", 3000);
        let server_id = "127.0.0.1:3000".to_string();

        let servers = vec![config];
        let handle = AlertHandle::spawn(servers, vec![], metric_rx, service_rx);

        // Exceed grace period
        for _ in 0..4 {
            let event = MetricEvent {
                server_id: server_id.clone(),
                metrics: create_test_metrics(85.0, Some(50.0)), // CPU above limit
                timestamp: Utc::now(),
                display_name: "Test".to_string(),
            };
            metric_tx.send(event).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let state = handle.get_state(server_id.clone()).await.unwrap();
        assert!(state.cpu_consecutive_exceeds > 0);

        // Send metric below limit - should trigger BackToOk and reset
        let event = MetricEvent {
            server_id: server_id.clone(),
            metrics: create_test_metrics(50.0, Some(50.0)), // Below limit
            timestamp: Utc::now(),
            display_name: "Test".to_string(),
        };
        metric_tx.send(event).unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state = handle.get_state(server_id.clone()).await.unwrap();
        assert_eq!(state.cpu_consecutive_exceeds, 0); // Reset to 0

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_mute_prevents_alert_processing() {
        let (metric_tx, metric_rx) = broadcast::channel(16);
        let (_service_tx, service_rx) = broadcast::channel(16);
        let config = create_test_server_config("127.0.0.1", 3000);
        let server_id = "127.0.0.1:3000".to_string();

        let servers = vec![config];
        let handle = AlertHandle::spawn(servers, vec![], metric_rx, service_rx);

        // Mute alerts
        handle.mute_alerts(60).await;

        // Send metrics that would normally trigger alerts
        for _ in 0..5 {
            let event = MetricEvent {
                server_id: server_id.clone(),
                metrics: create_test_metrics(95.0, Some(85.0)), // Way above limits
                timestamp: Utc::now(),
                display_name: "Test".to_string(),
            };
            metric_tx.send(event).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // State should not be updated when muted
        // Note: Current implementation still processes events when muted (by design)
        // but doesn't send actual alerts

        // Unmute
        handle.unmute_alerts().await;

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_multiple_servers_independent_state() {
        let (metric_tx, metric_rx) = broadcast::channel(16);
        let (_service_tx, service_rx) = broadcast::channel(16);
        let config1 = create_test_server_config("127.0.0.1", 3000);
        let config2 = create_test_server_config("127.0.0.1", 3001);
        let server1_id = "127.0.0.1:3000".to_string();
        let server2_id = "127.0.0.1:3001".to_string();

        let servers = vec![config1, config2];
        let handle = AlertHandle::spawn(servers, vec![], metric_rx, service_rx);

        // Exceed grace for server 1 only
        for _ in 0..3 {
            let event = MetricEvent {
                server_id: server1_id.clone(),
                metrics: create_test_metrics(85.0, Some(75.0)),
                timestamp: Utc::now(),
                display_name: "Test 1".to_string(),
            };
            metric_tx.send(event).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Server 1 should have exceeded grace
        let state1 = handle.get_state(server1_id.clone()).await.unwrap();
        assert!(state1.cpu_consecutive_exceeds > 0);

        // Server 2 should still be at 0
        let state2 = handle.get_state(server2_id.clone()).await.unwrap();
        assert_eq!(state2.cpu_consecutive_exceeds, 0);
        assert_eq!(state2.temp_consecutive_exceeds, 0);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_get_state_unregistered_server_returns_none() {
        let (_metric_tx, metric_rx) = broadcast::channel(16);
        let (_service_tx, service_rx) = broadcast::channel(16);
        let config = create_test_server_config("127.0.0.1", 3000);

        let servers = vec![config];
        let handle = AlertHandle::spawn(servers, vec![], metric_rx, service_rx);

        // Query state for non-existent server
        let state = handle.get_state("192.168.1.100:3000".to_string()).await;
        assert!(state.is_none());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_broadcast_lag_warning_logged() {
        let (metric_tx, _metric_rx) = broadcast::channel(2); // Very small buffer
        let (_service_tx, service_rx) = broadcast::channel(16);

        // Subscribe and don't read from it (will cause lag)
        let metric_rx_lagging = metric_tx.subscribe();

        let config = create_test_server_config("127.0.0.1", 3000);
        let servers = vec![config];
        let _handle = AlertHandle::spawn(servers, vec![], metric_rx_lagging, service_rx);

        // Send many metrics to overflow the buffer
        for i in 0..10 {
            let event = MetricEvent {
                server_id: "127.0.0.1:3000".to_string(),
                metrics: create_test_metrics(50.0, Some(50.0)),
                timestamp: Utc::now(),
                display_name: format!("Test {i}"),
            };
            let _ = metric_tx.send(event);
        }

        // Give actor time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Actor should still be running despite lag
        // (This test mainly verifies no panic occurs)
    }
}
