//! Application state management

use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};

use crate::ServerMetrics;

#[cfg(feature = "api")]
use crate::api::{ServerInfo, ServiceInfo};

/// Maximum number of metrics to keep in memory per server
const MAX_METRICS_BUFFER: usize = 1000;

/// Maximum number of alerts to keep in memory
const MAX_ALERTS_BUFFER: usize = 500;

/// Tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Servers,
    Services,
    Alerts,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Servers => Tab::Services,
            Tab::Services => Tab::Alerts,
            Tab::Alerts => Tab::Servers,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Tab::Servers => Tab::Alerts,
            Tab::Services => Tab::Servers,
            Tab::Alerts => Tab::Services,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Servers => "Servers",
            Tab::Services => "Services",
            Tab::Alerts => "Alerts",
        }
    }
}

/// Alert entry for the alerts timeline
#[derive(Debug, Clone)]
pub struct AlertEntry {
    pub timestamp: DateTime<Utc>,
    pub server_id: String,
    pub alert_type: String,
    pub message: String,
    pub severity: AlertSeverity,
}

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Metric data point with timestamp
#[derive(Debug, Clone)]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub metrics: ServerMetrics,
}

/// Application state
pub struct AppState {
    /// Current selected tab
    pub current_tab: Tab,

    /// Server list with health status
    pub servers: Vec<ServerInfo>,

    /// Service list with health status
    pub services: Vec<ServiceInfo>,

    /// Alert timeline
    pub alerts: VecDeque<AlertEntry>,

    /// Metric history per server (ring buffer)
    pub metrics_history: HashMap<String, VecDeque<MetricPoint>>,

    /// Selected server index (for Servers tab)
    pub selected_server: usize,

    /// Selected service index (for Services tab)
    pub selected_service: usize,

    /// Selected alert index (for Alerts tab)
    pub selected_alert: usize,

    /// Paused state (stops live updates)
    pub paused: bool,

    /// Last update timestamp
    pub last_update: Option<DateTime<Utc>>,

    /// Connection status
    pub connected: bool,

    /// Error message (if any)
    pub error_message: Option<String>,

    /// Chart time window in seconds (for sliding window display)
    pub time_window_seconds: u64,

    /// Number of data points to load (calculated from terminal width)
    pub data_limit: usize,
}

impl AppState {
    pub fn new(time_window_seconds: u64) -> Self {
        Self {
            current_tab: Tab::Servers,
            servers: Vec::new(),
            services: Vec::new(),
            alerts: VecDeque::new(),
            metrics_history: HashMap::new(),
            selected_server: 0,
            selected_service: 0,
            selected_alert: 0,
            paused: false,
            last_update: None,
            connected: false,
            error_message: None,
            time_window_seconds,
            data_limit: 100, // Default, will be updated based on terminal size
        }
    }

    /// Add a metric event to history
    pub fn add_metric(
        &mut self,
        server_id: String,
        metrics: ServerMetrics,
        timestamp: DateTime<Utc>,
    ) {
        let history = self.metrics_history.entry(server_id).or_default();

        history.push_back(MetricPoint { timestamp, metrics });

        // Time-based cleanup: remove metrics older than 2x time window
        let cutoff = Utc::now() - chrono::Duration::seconds((self.time_window_seconds * 2) as i64);
        while let Some(front) = history.front() {
            if front.timestamp < cutoff {
                history.pop_front();
            } else {
                break;
            }
        }

        // Safety: also trim to max buffer size if needed
        if history.len() > MAX_METRICS_BUFFER {
            history.pop_front();
        }

        self.last_update = Some(Utc::now());
    }

    /// Add an alert to the timeline
    pub fn add_alert(&mut self, alert: AlertEntry) {
        self.alerts.push_back(alert);

        // Trim to max buffer size
        if self.alerts.len() > MAX_ALERTS_BUFFER {
            self.alerts.pop_front();
        }
    }

    /// Update server list from API response
    pub fn update_servers(&mut self, servers: Vec<ServerInfo>) {
        self.servers = servers;

        // Clamp selection
        if self.selected_server >= self.servers.len() && !self.servers.is_empty() {
            self.selected_server = self.servers.len() - 1;
        }
    }

    /// Update service list from API response
    pub fn update_services(&mut self, services: Vec<ServiceInfo>) {
        self.services = services;

        // Clamp selection
        if self.selected_service >= self.services.len() && !self.services.is_empty() {
            self.selected_service = self.services.len() - 1;
        }
    }

    /// Select next item in current tab
    pub fn select_next(&mut self) {
        match self.current_tab {
            Tab::Servers => {
                if !self.servers.is_empty() {
                    self.selected_server = (self.selected_server + 1) % self.servers.len();
                }
            }
            Tab::Services => {
                if !self.services.is_empty() {
                    self.selected_service = (self.selected_service + 1) % self.services.len();
                }
            }
            Tab::Alerts => {
                if !self.alerts.is_empty() {
                    self.selected_alert = (self.selected_alert + 1) % self.alerts.len();
                }
            }
        }
    }

    /// Select previous item in current tab
    pub fn select_previous(&mut self) {
        match self.current_tab {
            Tab::Servers => {
                if !self.servers.is_empty() {
                    self.selected_server = if self.selected_server == 0 {
                        self.servers.len() - 1
                    } else {
                        self.selected_server - 1
                    };
                }
            }
            Tab::Services => {
                if !self.services.is_empty() {
                    self.selected_service = if self.selected_service == 0 {
                        self.services.len() - 1
                    } else {
                        self.selected_service - 1
                    };
                }
            }
            Tab::Alerts => {
                if !self.alerts.is_empty() {
                    self.selected_alert = if self.selected_alert == 0 {
                        self.alerts.len() - 1
                    } else {
                        self.selected_alert - 1
                    };
                }
            }
        }
    }

    /// Get currently selected server
    pub fn get_selected_server(&self) -> Option<&ServerInfo> {
        self.servers.get(self.selected_server)
    }

    /// Get metrics history for a server
    pub fn get_metrics_history(&self, server_id: &str) -> Option<&VecDeque<MetricPoint>> {
        self.metrics_history.get(server_id)
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Clear error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Check if connection appears to be stale based on last update time
    pub fn check_connection_timeout(&mut self, timeout_seconds: u64) {
        if let Some(last_update) = self.last_update {
            let elapsed = Utc::now().signed_duration_since(last_update);
            if elapsed.num_seconds() > timeout_seconds as i64 && self.connected {
                // Mark as disconnected due to timeout
                self.connected = false;
                self.error_message = Some(format!(
                    "Connection timeout (no data for {}s)",
                    timeout_seconds
                ));
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(300) // Default 5 minute window
    }
}
