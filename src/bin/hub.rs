use clap::Parser;
use server_monitoring::{
    config::{Config, read_config_file},
    monitors::server::server_monitor,
};
use tokio::{join, spawn};
use tracing::{error, level_filters::LevelFilter, trace};
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
