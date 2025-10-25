//! TUI Dashboard Viewer
//!
//! Interactive terminal dashboard for monitoring servers and services in real-time.
//! Connects to the hub's API via WebSocket for live metric streaming.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[cfg(feature = "dashboard")]
use guardia::viewer::App;

#[derive(Parser, Debug)]
#[command(name = "guardia-viewer")]
#[command(about = "Terminal UI dashboard for server monitoring", long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// API server URL (overrides config file)
    #[arg(short, long, value_name = "URL")]
    url: Option<String>,

    /// API authentication token (overrides config file)
    #[arg(short, long, value_name = "TOKEN")]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing - redirect logs to file when in TUI mode to avoid console output
    let log_path = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("dev.lome.guardia")
        .join("viewer.log");

    // Create directory if it doesn't exist
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path);

    match log_file {
        Ok(file) => {
            tracing_subscriber::fmt()
                .with_target(false)
                .with_level(true)
                .with_writer(file)
                .init();
        }
        Err(_) => {
            // If we can't create a log file, use a minimal stderr logger that only shows errors
            tracing_subscriber::fmt()
                .with_target(false)
                .with_level(true)
                .with_max_level(tracing::Level::ERROR)
                .init();
        }
    }

    #[cfg(feature = "dashboard")]
    {
        let args = Args::parse();

        // Load configuration
        let config = guardia::viewer::Config::load(args.config.as_deref())?;

        // Override with CLI args if provided
        let config = guardia::viewer::Config {
            api_url: args.url.unwrap_or(config.api_url),
            api_token: args.token.or(config.api_token),
            ..config
        };

        // Create and run the app
        let mut app = App::new(config)?;
        app.run().await?;
    }

    #[cfg(not(feature = "dashboard"))]
    {
        eprintln!("Error: This binary was compiled without dashboard support.");
        eprintln!("Please rebuild with: cargo build --features dashboard");
        std::process::exit(1);
    }

    Ok(())
}
