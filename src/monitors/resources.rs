use tokio::{
    spawn,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use tracing::{debug, instrument, trace};

use crate::{
    ServerMetrics,
    config::{Limit, ServerConfig},
};

#[derive(Debug)]
struct ResourceMonitor<TemperatureHandler, UsageHandler>
where
    TemperatureHandler: Fn(ResourceEvaluation, f32),
    UsageHandler: Fn(ResourceEvaluation, f32),
{
    config: ServerConfig,
    graces: Graces,
    temperature_handler: TemperatureHandler,
    usage_handler: UsageHandler,
}

#[derive(Debug, Clone, Copy, Default)]
struct Graces {
    temperature: usize,
    usage: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceEvaluation {
    Ok,
    Exceeding,
    StartsToExceed,
    BackToOk,
}

impl ResourceEvaluation {
    pub fn evaluate(
        resource: f32,
        limit: f32,
        grace: usize,
        current_grace: usize,
    ) -> ResourceEvaluation {
        // check, if we are under the limit
        if resource < limit {
            // if we are now under the limit but the grace period has been exceeded, send
            // notification that it is now okay
            if current_grace > grace {
                return ResourceEvaluation::BackToOk;
            }
            return ResourceEvaluation::Ok;
        }

        // check, if we are _now_ starting to exceed the grace period
        if current_grace == grace {
            return ResourceEvaluation::StartsToExceed;
        }

        ResourceEvaluation::Exceeding
    }
}

pub fn resource_monitor<TemperatureHandler, UsageHandler>(
    config: &ServerConfig,
    temperature_handler: TemperatureHandler,
    usage_handler: UsageHandler,
) -> UnboundedSender<ServerMetrics>
where
    TemperatureHandler: Fn(ResourceEvaluation, f32) + Send + 'static,
    UsageHandler: Fn(ResourceEvaluation, f32) + Send + 'static,
{
    let (sender, receiver) = unbounded_channel::<ServerMetrics>();
    let mut monitor = ResourceMonitor::new(config.clone(), temperature_handler, usage_handler);

    spawn(async move {
        monitor.start(receiver).await;
    });

    sender
}

impl<TemperatureHandler, UsageHandler> ResourceMonitor<TemperatureHandler, UsageHandler>
where
    TemperatureHandler: Fn(ResourceEvaluation, f32),
    UsageHandler: Fn(ResourceEvaluation, f32),
{
    pub fn new(
        config: ServerConfig,
        temperature_handler: TemperatureHandler,
        usage_handler: UsageHandler,
    ) -> ResourceMonitor<TemperatureHandler, UsageHandler> {
        Self {
            config,
            temperature_handler,
            usage_handler,
            graces: Graces::default(),
        }
    }

    fn server(&self) -> String {
        let ServerConfig { ip, port, .. } = self.config;
        format!("{ip}:{port}")
    }

    async fn start(&mut self, mut chan: UnboundedReceiver<ServerMetrics>) {
        while let Some(metrics) = chan.recv().await {
            self.update(&metrics).await;
        }
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

    async fn update_temperature(&mut self, metrics: &ServerMetrics, limit: Limit) {
        let Some(current_temp) = metrics.components.average_temperature else {
            return;
        };

        let Limit { limit, grace, .. } = limit;
        let grace = grace.unwrap_or_default();

        let evaluation_result = ResourceEvaluation::evaluate(
            current_temp,
            limit as f32,
            grace,
            self.graces.temperature,
        );

        match evaluation_result {
            ResourceEvaluation::Ok => {}
            ResourceEvaluation::Exceeding => {
                self.graces.temperature += 1;
            }
            ResourceEvaluation::StartsToExceed => {
                self.graces.temperature += 1;
                debug!(
                    "{}: temperature starts to exceed grace period",
                    self.server()
                );
                (self.temperature_handler)(evaluation_result, current_temp);
            }
            ResourceEvaluation::BackToOk => {
                debug!("{}: temperature is back to normal", self.server());
                self.graces.temperature = 0;
                (self.temperature_handler)(evaluation_result, current_temp);
            }
        };

        trace!(
            "{}: temperature {current_temp} (max: {limit}) -> {evaluation_result:?} ({}/{grace})",
            self.server(),
            self.graces.temperature
        );
    }

    async fn update_usage(&mut self, metrics: &ServerMetrics, limit: Limit) {
        let current_usage = metrics.cpus.average_usage;

        let Limit { limit, grace, .. } = limit;
        let grace = grace.unwrap_or_default();

        let evaluation_result =
            ResourceEvaluation::evaluate(current_usage, limit as f32, grace, self.graces.usage);

        match evaluation_result {
            ResourceEvaluation::Ok => {}
            ResourceEvaluation::Exceeding => {
                self.graces.usage += 1;
            }
            ResourceEvaluation::StartsToExceed => {
                self.graces.usage += 1;
                debug!("{}: CPU usage starts to exceed grace period", self.server());
                (self.usage_handler)(evaluation_result, current_usage);
            }
            ResourceEvaluation::BackToOk => {
                debug!("{}: CPU usage is back to normal", self.server());
                self.graces.usage = 0;
                (self.usage_handler)(evaluation_result, current_usage);
            }
        }
        trace!(
            "{}: CPU {current_usage} (max: {limit}) -> {evaluation_result:?} ({}/{grace})",
            self.server(),
            self.graces.usage
        );
    }
}
