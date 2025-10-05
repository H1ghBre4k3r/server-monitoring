//! Main application logic

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use tokio::sync::mpsc;

use super::{
    config::Config,
    state::{AlertEntry, AlertSeverity, AppState},
    ui,
    websocket::{WebSocketClient, WsEvent},
};

use crate::api::{ServerInfo, ServiceInfo};

/// Main TUI application
pub struct App {
    config: Config,
    state: AppState,
    ws_rx: mpsc::UnboundedReceiver<WsEvent>,
}

impl App {
    /// Create a new application instance
    pub fn new(config: Config) -> Result<Self> {
        // Create WebSocket client
        let ws_client = WebSocketClient::new(&config.api_url, config.api_token.clone());

        // Connect to WebSocket
        let ws_rx = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(ws_client.connect())
        })?;

        Ok(Self {
            state: AppState::new(config.time_window_seconds),
            config,
            ws_rx,
        })
    }

    /// Run the application
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Initial data fetch
        self.fetch_initial_data().await?;

        // Run event loop
        let result = self.run_event_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Fetch initial data from API
    async fn fetch_initial_data(&mut self) -> Result<()> {
        let client = reqwest::Client::new();
        let base_url = self.config.api_url.clone();

        // Build request with optional auth
        let mut request = client.get(format!("{}/api/v1/servers", &base_url));
        if let Some(token) = &self.config.api_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Fetch servers
        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let json: serde_json::Value = response.json().await?;
                    if let Some(servers) = json["servers"].as_array() {
                        let servers: Vec<ServerInfo> = serde_json::from_value(serde_json::Value::Array(servers.clone()))?;
                        self.state.update_servers(servers);
                        self.state.connected = true;

                        // Fetch historical metrics for each server
                        self.fetch_historical_metrics(&client).await;
                    }
                } else {
                    self.state.error_message = Some(format!("API error: {}", response.status()));
                }
            }
            Err(e) => {
                self.state.error_message = Some(format!("Connection failed: {}", e));
            }
        }

        // Fetch services
        let mut request = client.get(format!("{}/api/v1/services", &base_url));
        if let Some(token) = &self.config.api_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let json: serde_json::Value = response.json().await?;
                    if let Some(services) = json["services"].as_array() {
                        let services: Vec<ServiceInfo> = serde_json::from_value(serde_json::Value::Array(services.clone()))?;
                        self.state.update_services(services);
                    }
                }
            }
            Err(_) => {
                // Ignore service fetch errors
            }
        }

        Ok(())
    }

    /// Fetch historical metrics for all servers
    async fn fetch_historical_metrics(&mut self, client: &reqwest::Client) {
        // Calculate how many metrics to fetch based on time window
        // Assume metrics arrive every ~10 seconds, so fetch time_window / 10 points
        let limit = (self.state.time_window_seconds / 10).max(10);

        let base_url = self.config.api_url.clone();
        let api_token = self.config.api_token.clone();
        let servers = self.state.servers.clone(); // Clone to avoid borrow issues

        for server in servers {
            let mut request = client.get(format!(
                "{}/api/v1/servers/{}/metrics?limit={}",
                base_url, server.server_id, limit
            ));

            if let Some(token) = &api_token {
                request = request.header("Authorization", format!("Bearer {}", token));
            }

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(json) = response.json::<serde_json::Value>().await {
                            if let Some(metrics) = json["metrics"].as_array() {
                                // Parse and add each metric to history
                                for metric_value in metrics {
                                    if let Ok(metric_row) = serde_json::from_value::<crate::storage::schema::MetricRow>(metric_value.clone()) {
                                        self.state.add_metric(
                                            metric_row.server_id,
                                            metric_row.metadata,
                                            metric_row.timestamp,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Silently ignore errors for historical data
                }
            }
        }
    }

    /// Main event loop
    async fn run_event_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let mut last_refresh = std::time::Instant::now();

        loop {
            // Render UI
            terminal.draw(|f| ui::render(f, &self.state))?;

            // Handle WebSocket events (non-blocking)
            while let Ok(event) = self.ws_rx.try_recv() {
                self.handle_ws_event(event);
            }

            // Handle keyboard events (with timeout)
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if self.handle_key_event(key.code).await? {
                            break; // Quit
                        }
                    }
                }
            }

            // Periodic refresh
            if last_refresh.elapsed().as_secs() >= self.config.refresh_interval {
                self.fetch_initial_data().await?;
                last_refresh = std::time::Instant::now();
            }
        }

        Ok(())
    }

    /// Handle WebSocket event
    fn handle_ws_event(&mut self, event: WsEvent) {
        if self.state.paused {
            return;
        }

        match event {
            WsEvent::Metric {
                server_id,
                metrics,
                timestamp,
                ..
            } => {
                self.state.add_metric(server_id, metrics, timestamp);
                self.state.connected = true;
            }
            WsEvent::ServiceCheck {
                service_name,
                status,
                timestamp,
                error_message,
                ..
            } => {
                // Add to alerts if DOWN
                if matches!(status, crate::actors::messages::ServiceStatus::Down) {
                    self.state.add_alert(AlertEntry {
                        timestamp,
                        server_id: service_name.clone(),
                        alert_type: "Service Down".to_string(),
                        message: error_message.unwrap_or_else(|| "Service check failed".to_string()),
                        severity: AlertSeverity::Critical,
                    });
                }
            }
        }
    }

    /// Handle keyboard event
    async fn handle_key_event(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                return Ok(true); // Quit
            }
            KeyCode::Tab | KeyCode::Right => {
                self.state.current_tab = self.state.current_tab.next();
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.state.current_tab = self.state.current_tab.previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_previous();
            }
            KeyCode::Char(' ') => {
                self.state.toggle_pause();
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.fetch_initial_data().await?;
            }
            KeyCode::Char('c') => {
                self.state.clear_error();
            }
            _ => {}
        }

        Ok(false)
    }
}
