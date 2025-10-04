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
    config::{Limit, ServerConfig},
    monitors::resources::ResourceEvaluation,
};

use super::messages::{AlertCommand, AlertState, MetricEvent};

/// Per-server alert state
#[derive(Debug, Clone)]
struct ServerAlertState {
    /// Server configuration
    config: ServerConfig,

    /// Alert manager for sending notifications
    alert_manager: AlertManager,

    /// Temperature grace counter
    temp_grace_counter: usize,

    /// CPU usage grace counter
    usage_grace_counter: usize,
}

/// Actor that evaluates metrics and sends alerts
pub struct AlertActor {
    /// Per-server state
    servers: HashMap<String, ServerAlertState>,

    /// Command receiver
    command_rx: mpsc::Receiver<AlertCommand>,

    /// Metric event receiver (broadcast subscription)
    metric_rx: broadcast::Receiver<MetricEvent>,

    /// Whether alerts are muted
    muted: bool,
}

impl AlertActor {
    /// Create a new alert actor
    pub fn new(
        command_rx: mpsc::Receiver<AlertCommand>,
        metric_rx: broadcast::Receiver<MetricEvent>,
    ) -> Self {
        Self {
            servers: HashMap::new(),
            command_rx,
            metric_rx,
            muted: false,
        }
    }

    /// Register a server for monitoring
    ///
    /// This should be called for each server before metrics start flowing.
    pub fn register_server(&mut self, config: ServerConfig) {
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
        limit: &Limit,
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
    async fn evaluate_cpu_usage(event: &MetricEvent, state: &mut ServerAlertState, limit: &Limit) {
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
    /// - `servers`: Initial server configurations to monitor
    /// - `metric_rx`: Broadcast receiver for metric events
    pub fn spawn(servers: Vec<ServerConfig>, metric_rx: broadcast::Receiver<MetricEvent>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        let mut actor = AlertActor::new(cmd_rx, metric_rx);

        // Register all servers
        for config in servers {
            actor.register_server(config);
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

    // Note: ResourceEvaluation tests are in monitors/resources.rs
    // These tests focus on the AlertActor behavior

    #[tokio::test]
    async fn test_alert_handle_creation() {
        let (_metric_tx, metric_rx) = broadcast::channel(16);
        let servers = vec![];

        let _handle = AlertHandle::spawn(servers, metric_rx);

        // Handle created successfully
    }
}
