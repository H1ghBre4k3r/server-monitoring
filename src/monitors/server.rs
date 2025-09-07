use std::time::Duration;

use tracing::{debug, error, instrument, trace};

use crate::{
    ServerMetrics, alerts::AlertManager, config::ServerConfig,
    monitors::resources::resource_monitor,
};

#[instrument(skip_all)]
pub async fn server_monitor(config: ServerConfig) {
    let ServerConfig {
        ip,
        port,
        interval,
        token,
        display,
        ..
    } = config.clone();
    let display_name = display.unwrap_or(String::from("unknown"));
    debug!("starting server monitor for {display_name} ({ip}:{port}) with interval {interval}");

    let url = format!("http://{ip}:{port}/metrics");

    let create_channel = || {
        let alert_manager = AlertManager::new(config.clone());
        let temp_alert_manager = alert_manager.clone();
        let usage_alert_manager = alert_manager;

        resource_monitor(
            &config,
            move |eval, temp| {
                let alert_manager = temp_alert_manager.clone();
                tokio::spawn(async move {
                    alert_manager.send_temperature_alert(eval, temp).await;
                });
            },
            move |eval, usage| {
                let alert_manager = usage_alert_manager.clone();
                tokio::spawn(async move {
                    alert_manager.send_usage_alert(eval, usage).await;
                });
            },
        )
    };

    let mut chan = create_channel();

    loop {
        tokio::time::sleep(Duration::from_secs(interval as u64)).await;

        trace!("{url}: requesting metrics");

        let client = reqwest::Client::new();
        let request = client.get(&url).header(
            "X-MONITORING-SECRET",
            token.as_ref().unwrap_or(&String::new()),
        );

        let response = request.send().await;
        let body = match response {
            Ok(body) => body,
            Err(e) => {
                error!("{url}: error during request: {e}");
                continue;
            }
        };

        let body = match body.text().await {
            Ok(body) => body,
            Err(e) => {
                error!("{url}: error during decode: {e}");
                continue;
            }
        };

        let metrics = match serde_json::from_str::<ServerMetrics>(&body) {
            Ok(metrics) => metrics,
            Err(e) => {
                error!("{url}: error while trying to parse the metrics: {e}");
                continue;
            }
        };

        trace!("{url}: received metrics");

        if let Err(e) = chan.send(metrics) {
            error!("{url}: error sending in channel: {e}");
            chan = create_channel();
        }
    }
}
