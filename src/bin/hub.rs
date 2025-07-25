use std::time::Duration;

use clap::Parser;
use reqwest::Request;
use server_monitoring::{
    ServerMetrics,
    config::{Config, ServerConfig, read_config_file},
};
use tokio::{join, spawn};
use tracing::{debug, error, instrument, trace};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, Parser)]
struct Args {
    /// Config file
    #[arg(short)]
    file: String,
}

fn init() {
    let filter = filter::Targets::new().with_target("hub", tracing::metadata::LevelFilter::TRACE);
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

#[instrument]
async fn server_monitor(config: ServerConfig) {
    let ServerConfig {
        ip,
        port,
        interval,
        token,
    } = config;
    debug!("starting server monitor for {ip}:{port} with interval {interval}");

    let url = format!("http://{ip}:{port}/metrics");

    loop {
        tokio::time::sleep(Duration::from_secs(interval as u64)).await;

        trace!("requesting metrics for {url}");

        let client = reqwest::Client::new();
        let request = client.get(&url).header(
            "X-MONITORING-SECRET",
            token.as_ref().unwrap_or(&String::new()),
        );

        let response = request.send().await;
        let body = match response {
            Ok(body) => body,
            Err(e) => {
                error!("{e}");
                continue;
            }
        };

        let body = match body.text().await {
            Ok(body) => body,
            Err(e) => {
                error!("{e}");
                continue;
            }
        };

        let metrics = match serde_json::from_str::<ServerMetrics>(&body) {
            Ok(metrics) => metrics,
            Err(e) => {
                error!("{e}");
                continue;
            }
        };

        trace!("received server metrics for {url}: {metrics:?}");
    }
}
