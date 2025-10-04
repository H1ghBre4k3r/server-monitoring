use clap::Parser;
use server_monitoring::{
    actors::{
        alert::AlertHandle, collector::CollectorHandle, service_monitor::ServiceHandle,
        storage::StorageHandle,
    },
    config::{Config, StorageConfig, read_config_file},
};
use tokio::sync::broadcast;
use tracing::{error, info, level_filters::LevelFilter, trace, warn};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "storage-sqlite")]
use server_monitoring::storage::{StorageBackend, sqlite::SqliteBackend};

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

    // Create broadcast channel for service check events
    let (service_tx, _service_rx) = broadcast::channel(256);

    // Initialize storage backend based on config
    #[cfg(feature = "storage-sqlite")]
    let (backend, retention_days) = initialize_storage_backend(&config.storage).await;

    // Spawn storage actor with optional persistent backend
    #[cfg(feature = "storage-sqlite")]
    let storage_handle =
        StorageHandle::spawn_with_backend(metric_tx.subscribe(), backend, retention_days);

    #[cfg(not(feature = "storage-sqlite"))]
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

    // Spawn service monitor actor for each configured service
    let mut service_handles = Vec::new();
    if let Some(services) = config.services {
        for service_config in services {
            let service_name = service_config.name.clone();

            let handle = ServiceHandle::spawn(service_config, service_tx.clone());
            info!("service monitor actor started for {service_name}");
            service_handles.push(handle);
        }
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

    info!("shutting down service monitors...");
    for handle in service_handles {
        handle.shutdown().await;
    }

    info!("shutting down alert actor...");
    alert_handle.shutdown().await;

    info!("shutting down storage actor...");
    storage_handle.shutdown().await;

    info!("all actors stopped, exiting");

    Ok(())
}

/// Initialize storage backend based on configuration
/// Returns (backend, retention_days)
#[cfg(feature = "storage-sqlite")]
async fn initialize_storage_backend(
    storage_config: &Option<StorageConfig>,
) -> (Option<Box<dyn StorageBackend>>, Option<u32>) {
    match storage_config {
        Some(StorageConfig::Sqlite {
            path,
            retention_days,
        }) => {
            info!(
                "initializing SQLite backend at: {:?} (retention: {} days)",
                path, retention_days
            );
            match SqliteBackend::new(path).await {
                Ok(backend) => {
                    info!("SQLite backend initialized successfully");
                    (
                        Some(Box::new(backend) as Box<dyn StorageBackend>),
                        Some(*retention_days),
                    )
                }
                Err(e) => {
                    error!("failed to initialize SQLite backend: {}", e);
                    warn!("falling back to in-memory storage");
                    (None, None)
                }
            }
        }
        Some(StorageConfig::None) | None => {
            info!("using in-memory storage (no persistence)");
            (None, None)
        }
    }
}
