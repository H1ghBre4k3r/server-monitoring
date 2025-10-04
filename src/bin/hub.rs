use clap::Parser;
use server_monitoring::{
    actors::{alert::AlertHandle, collector::CollectorHandle, storage::StorageHandle},
    config::{Config, read_config_file},
};
use tokio::sync::broadcast;
use tracing::{error, info, level_filters::LevelFilter, trace, warn};
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
                .pretty()
                .with_writer(std::io::stderr)
                .compact(),
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

    // Run the actor-based monitoring system
    run_monitoring(config).await?;

    Ok(())
}

/// Run the actor-based monitoring system
async fn run_monitoring(config: Config) -> anyhow::Result<()> {
    let Some(servers) = config.servers else {
        warn!("no servers configured");
        return Ok(());
    };

    info!(
        "starting actor-based monitoring for {} servers",
        servers.len()
    );

    // Create broadcast channel for metric events
    // Capacity of 256 allows some buffering for slow consumers
    let (metric_tx, _metric_rx) = broadcast::channel(256);

    // Spawn storage actor (currently a stub)
    let storage_handle = StorageHandle::spawn(metric_tx.subscribe());
    info!("storage actor started");

    // Spawn alert actor with all server configs
    let alert_handle = AlertHandle::spawn(servers.clone(), metric_tx.subscribe());
    info!("alert actor started");

    // Spawn collector actor for each server
    let mut collector_handles = Vec::new();
    for server_config in servers {
        let display_name = server_config
            .display
            .clone()
            .unwrap_or_else(|| format!("{}:{}", server_config.ip, server_config.port));

        let handle = CollectorHandle::spawn(server_config.clone(), metric_tx.clone());
        info!("collector actor started for {display_name}");
        collector_handles.push(handle);
    }

    info!("all actors started, monitoring active");
    info!("press Ctrl+C to shutdown gracefully");

    // Wait for shutdown signal
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            info!("received shutdown signal, stopping actors...");
        }
        Err(err) => {
            error!("unable to listen for shutdown signal: {err}");
        }
    }

    // Graceful shutdown: stop all actors
    info!("shutting down collectors...");
    for handle in collector_handles {
        if let Err(e) = handle.shutdown().await {
            warn!("error shutting down collector: {e}");
        }
    }

    info!("shutting down alert actor...");
    alert_handle.shutdown().await;

    info!("shutting down storage actor...");
    storage_handle.shutdown().await;

    info!("all actors stopped, exiting");

    Ok(())
}
