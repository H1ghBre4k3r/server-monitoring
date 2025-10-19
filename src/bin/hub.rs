use axum::routing::trace;
use clap::Parser;
use server_monitoring::{
    actors::{
        alert::AlertHandle, collector::CollectorHandle, service_monitor::ServiceHandle,
        storage::StorageHandle,
    },
    config::{ResolvedConfig, StorageConfig, read_config_file},
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
        ("guardia_hub", LevelFilter::TRACE),
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

    // Validate storage configuration
    if let Some(ref storage_config) = config.storage
        && let Err(e) = storage_config.validate()
    {
        error!("Invalid storage configuration: {}", e);
        return Err(anyhow::anyhow!("Configuration validation failed: {}", e));
    }

    // Resolve configuration (merge defaults and resolve alert references)
    let resolved_config = config.resolve()?;
    info!("resolved configuration: {resolved_config:#?}");
    info!(
        "configuration resolved: {} servers, {} services",
        resolved_config.servers.len(),
        resolved_config.services.len()
    );

    // Run the actor-based monitoring system
    run_monitoring(resolved_config).await?;

    Ok(())
}

/// Run the actor-based monitoring system
async fn run_monitoring(resolved_config: ResolvedConfig) -> anyhow::Result<()> {
    if resolved_config.servers.is_empty() {
        warn!("no servers configured");
        return Ok(());
    }

    info!(
        "starting actor-based monitoring for {} servers",
        resolved_config.servers.len()
    );

    // Create broadcast channel for metric events
    // Capacity of 256 allows some buffering for slow consumers
    let (metric_tx, _metric_rx) = broadcast::channel(256);

    // Create broadcast channel for service check events
    let (service_tx, _service_rx) = broadcast::channel(256);

    // Create broadcast channel for polling status events
    let (polling_tx, _polling_rx) = broadcast::channel(256);

    // Clone servers and services for actor spawning
    let servers = resolved_config.servers.clone();
    let services = resolved_config.services.clone();

    // Initialize storage backend based on config
    #[cfg(feature = "storage-sqlite")]
    let (backend, retention_days, cleanup_interval_hours) =
        initialize_storage_backend(&resolved_config.storage).await;

    // Spawn storage actor with optional persistent backend
    #[cfg(feature = "storage-sqlite")]
    let storage_handle = StorageHandle::spawn_with_backend(
        metric_tx.subscribe(),
        service_tx.subscribe(),
        backend,
        retention_days,
        cleanup_interval_hours,
    );

    #[cfg(not(feature = "storage-sqlite"))]
    let storage_handle = StorageHandle::spawn(metric_tx.subscribe(), service_tx.subscribe());

    info!("storage actor started");

    // Spawn alert actor with all server and service configs
    let alert_handle = AlertHandle::spawn(
        servers.clone(),
        services.clone(),
        metric_tx.subscribe(),
        service_tx.subscribe(),
    );
    info!("alert actor started");

    // Spawn collector actor for each server
    let mut collector_handles = Vec::new();
    for server_config in servers {
        let display_name = server_config
            .display
            .clone()
            .unwrap_or_else(|| format!("{}:{}", server_config.ip, server_config.port));

        let handle =
            CollectorHandle::spawn(server_config.clone(), metric_tx.clone(), polling_tx.clone());
        info!("collector actor started for {display_name}");
        collector_handles.push(handle);
    }

    // Spawn service monitor actor for each configured service
    let mut service_handles = Vec::new();
    for service_config in services {
        let service_name = service_config.name.clone();

        let handle = ServiceHandle::spawn(service_config, service_tx.clone());
        info!("service monitor actor started for {service_name}");
        service_handles.push(handle);
    }

    info!("all actors started, monitoring active");

    // Spawn API server if configured
    #[cfg(feature = "api")]
    if let Some(api_config) = resolved_config.api {
        use server_monitoring::api::{ApiConfig, ApiState, spawn_api_server};
        use std::net::SocketAddr;

        let bind_addr: SocketAddr = format!("{}:{}", api_config.bind, api_config.port)
            .parse()
            .expect("Invalid API bind address");

        let api_state = ApiState::new(
            storage_handle.clone(),
            alert_handle.clone(),
            collector_handles.clone(),
            service_handles.clone(),
            metric_tx.clone(),
            service_tx.clone(),
        );

        let api_config = ApiConfig {
            bind_addr,
            auth_token: api_config.auth_token,
            enable_cors: api_config.enable_cors,
        };

        // Spawn polling status tracking task
        let polling_store_for_tracker = api_state.polling_store.clone();
        let mut polling_rx = polling_tx.subscribe();
        tokio::spawn(async move {
            while let Ok(polling_event) = polling_rx.recv().await {
                polling_store_for_tracker.handle_event(&polling_event).await;
            }
        });

        match spawn_api_server(api_config, api_state).await {
            Ok(addr) => {
                info!("API server started on http://{}", addr);
            }
            Err(e) => {
                error!("Failed to start API server: {}", e);
            }
        }
    } else {
        info!("API server disabled (not configured)");
    }

    #[cfg(not(feature = "api"))]
    info!("API server disabled (feature not enabled)");

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
/// Returns (backend, retention_days, cleanup_interval_hours)
#[cfg(feature = "storage-sqlite")]
async fn initialize_storage_backend(
    storage_config: &Option<StorageConfig>,
) -> (Option<Box<dyn StorageBackend>>, Option<u32>, Option<u32>) {
    match storage_config {
        Some(StorageConfig::Sqlite {
            path,
            retention_days,
            cleanup_interval_hours,
        }) => {
            info!(
                "initializing SQLite backend at: {:?} (retention: {} days, cleanup: every {} hours)",
                path, retention_days, cleanup_interval_hours
            );
            match SqliteBackend::new(path).await {
                Ok(backend) => {
                    info!("SQLite backend initialized successfully");
                    (
                        Some(Box::new(backend) as Box<dyn StorageBackend>),
                        Some(*retention_days),
                        Some(*cleanup_interval_hours),
                    )
                }
                Err(e) => {
                    error!("failed to initialize SQLite backend: {}", e);
                    warn!("falling back to in-memory storage");
                    (None, None, None)
                }
            }
        }
        Some(StorageConfig::None) | None => {
            info!("using in-memory storage (no persistence)");
            (None, None, None)
        }
    }
}
