//! API shared state containing actor handles

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::actors::{
    alert::AlertHandle,
    collector::CollectorHandle,
    messages::{MetricEvent, ServiceCheckEvent, PollingStatusEvent},
    service_monitor::ServiceHandle,
    storage::StorageHandle,
};

/// Polling status information for a server
#[derive(Debug, Clone)]
pub struct PollingStatus {
    pub last_success: Option<String>,
    pub last_error: Option<String>,
    pub last_success_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Shared polling status store
#[derive(Debug, Default)]
pub struct PollingStatusStore {
    statuses: Arc<RwLock<HashMap<String, PollingStatus>>>,
}

impl PollingStatusStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update polling status for a server
    pub async fn update_status(&self, server_id: &str, success: bool, error_message: Option<String>) {
        let mut statuses = self.statuses.write().await;
        let status = statuses.entry(server_id.to_string()).or_insert_with(PollingStatus::default);

        let now = chrono::Utc::now();
        if success {
            status.last_success = Some(now.to_rfc3339());
            status.last_success_timestamp = Some(now);
            status.last_error = None;
            status.last_error_timestamp = None;
        } else {
            status.last_error = error_message;
            status.last_error_timestamp = Some(now);
        }
    }

    /// Get polling status for a server
    pub async fn get_status(&self, server_id: &str) -> PollingStatus {
        let statuses = self.statuses.read().await;
        statuses.get(server_id).cloned().unwrap_or_default()
    }

    /// Process a PollingStatusEvent
    pub async fn handle_event(&self, event: &PollingStatusEvent) {
        self.update_status(&event.server_id, event.success, event.error_message.clone()).await;
    }
}

impl Default for PollingStatus {
    fn default() -> Self {
        Self {
            last_success: None,
            last_error: None,
            last_success_timestamp: None,
            last_error_timestamp: None,
        }
    }
}

/// Shared state passed to all API handlers
#[derive(Clone)]
pub struct ApiState {
    /// Handle to storage actor for querying metrics and service checks
    pub storage: StorageHandle,

    /// Handle to alert actor for alert status
    pub alerts: AlertHandle,

    /// Handle to collector actors for server status
    pub collectors: Vec<CollectorHandle>,

    /// Handle to service monitor actors for service status
    pub service_monitors: Vec<ServiceHandle>,

    /// Broadcast sender for metric events (for WebSocket streaming)
    pub metric_tx: broadcast::Sender<MetricEvent>,

    /// Broadcast sender for service check events (for WebSocket streaming)
    pub service_check_tx: broadcast::Sender<ServiceCheckEvent>,

    /// Polling status store for tracking server availability
    pub polling_store: Arc<PollingStatusStore>,
}

impl ApiState {
    /// Create new API state with all actor handles
    pub fn new(
        storage: StorageHandle,
        alerts: AlertHandle,
        collectors: Vec<CollectorHandle>,
        service_monitors: Vec<ServiceHandle>,
        metric_tx: broadcast::Sender<MetricEvent>,
        service_check_tx: broadcast::Sender<ServiceCheckEvent>,
    ) -> Self {
        Self {
            storage,
            alerts,
            collectors,
            service_monitors,
            metric_tx,
            service_check_tx,
            polling_store: Arc::new(PollingStatusStore::new()),
        }
    }
}
