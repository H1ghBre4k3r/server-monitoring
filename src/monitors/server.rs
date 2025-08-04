use std::time::Duration;

use tracing::{debug, error, instrument, trace};

use crate::{ServerMetrics, config::ServerConfig, monitors::resources::ResourceMonitor};

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

    // TODO: dispatch this as another thread and use channel for communication
    let mut monitor = ResourceMonitor::new(config);

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

        // trace!("received server metrics for {url}: {metrics:?}");

        monitor.update(&metrics).await;
    }
}
