//! API shared state containing actor handles

use tokio::sync::broadcast;

use crate::actors::{
    alert::AlertHandle,
    collector::CollectorHandle,
    messages::{MetricEvent, ServiceCheckEvent},
    service_monitor::ServiceHandle,
    storage::StorageHandle,
};

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
        }
    }
}
