use tracing::{debug, instrument};

use crate::{
    ServerMetrics,
    config::{Limit, ServerConfig},
};

#[derive(Debug, Clone)]
pub struct LimitMonitor {
    config: ServerConfig,
    graces: Graces,
}

#[derive(Debug, Clone, Copy, Default)]
struct Graces {
    temperature: usize,
    usage: usize,
}

impl LimitMonitor {
    pub fn new(config: ServerConfig) -> LimitMonitor {
        Self {
            config,
            graces: Graces::default(),
        }
    }

    fn server(&self) -> String {
        let ServerConfig { ip, port, .. } = self.config;
        format!("{ip}:{port}")
    }

    #[instrument(skip_all)]
    pub async fn update(&mut self, metrics: &ServerMetrics) {
        let Some(limits) = self.config.limits.clone() else {
            return;
        };

        if let Some(limit) = limits.temperature {
            self.update_temperature(metrics, limit).await;
        }

        if let Some(limit) = limits.usage {
            self.update_usage(metrics, limit).await;
        }
    }

    #[instrument(skip_all)]
    async fn update_temperature(&mut self, metrics: &ServerMetrics, limit: Limit) {
        let Some(current_temp) = metrics.components.average_temperature else {
            return;
        };

        let Limit { limit, grace } = limit;
        let grace = grace.unwrap_or_default();

        // check, if we are under the limit
        if current_temp < limit as f32 {
            // if we are now under the limit but the grace period has been exceeded, send
            // notification that it is now okay
            if self.graces.temperature > grace {
                debug!("{}: temperature is back to normal", self.server());
                // TODO: send notification
            }

            // set grace period back to 0
            self.graces.temperature = 0;
            return;
        }

        // check, if we are _now_ starting to exceed the grace period
        let exceeds_grace = self.graces.temperature == grace;
        if exceeds_grace {
            debug!("{}: temperature exceeds grace period", self.server());
            // TODO: send notification
        }
        self.graces.temperature += 1;
        println!("Temperature: {:?}", metrics.components.average_temperature);
    }

    #[instrument(skip_all)]
    async fn update_usage(&mut self, metrics: &ServerMetrics, limit: Limit) {
        let current_usage = metrics.cpus.average_usage;

        let Limit { limit, grace } = limit;
        let grace = grace.unwrap_or_default();

        // check, if we are under the limit
        if current_usage < limit as f32 {
            // if we are now under the limit but the grace period has been exceeded, send
            // notification that it is now okay
            if self.graces.usage > grace {
                debug!("{}: CPU usage is back to normal", self.server());
                // TODO: send notification
            }

            self.graces.usage = 0;
            return;
        }

        // check, if we are _now_ starting to exceed the grace period
        let exceeds_grace = self.graces.usage == grace;
        if exceeds_grace {
            debug!("{}: CPU usage exceeds grace period", self.server());
            // TODO: send notification
        }
        self.graces.usage += 1;
        println!("CPU: {}", metrics.cpus.average_usage);
    }
}
