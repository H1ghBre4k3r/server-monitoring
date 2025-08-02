use std::time::Duration;

use clap::Parser;
use server_monitoring::{
    ServerMetrics,
    config::{Config, ServerConfig, read_config_file},
    resource_monitor::ResourceMonitor,
};
use tokio::{join, spawn};
use tracing::{debug, error, instrument, level_filters::LevelFilter, trace};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, Parser)]
struct Args {
    /// Config file
    #[arg(short)]
    file: String,
}

fn init() {
    let filter = filter::Targets::new().with_targets(vec![
        ("server_monitoring", LevelFilter::TRACE),
        ("hub", LevelFilter::TRACE),
    ]);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .compact()
                .with_ansi(false),
        )
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init();
    let args = Args::parse();
    trace!("started with args: {args:?}");

    let config = read_config_file(&args.file)?;

    let servers = dispatch_servers(&config);

    join!(servers);

    Ok(())
}

async fn dispatch_servers(config: &Config) {
    let mut handles = vec![];
    if let Some(servers) = &config.servers {
        for server in servers {
            let server = server.clone();

            handles.push(spawn(server_monitor(server)));
        }
    }

    for handler in handles {
        if let Err(e) = handler.await {
            error!("{e}");
        }
    }
}

#[instrument(skip_all)]
async fn server_monitor(config: ServerConfig) {
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
