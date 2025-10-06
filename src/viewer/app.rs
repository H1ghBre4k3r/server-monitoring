//! Main application logic

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tokio::sync::mpsc;

use crate::api::types::WsEvent;

use super::{
    config::Config,
    state::{AlertEntry, AlertSeverity, AppState},
    ui,
    websocket::WebSocketClient,
};

/// Main TUI application
pub struct App {
    config: Config,
    state: AppState,
    ws_rx: mpsc::UnboundedReceiver<WsEvent>,
    /// Reusable HTTP client for API requests
    http_client: reqwest::Client,
}

impl App {
    /// Create a new application instance
    pub fn new(config: Config) -> Result<Self> {
        // Create reusable HTTP client
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        // Create WebSocket client
        let ws_client = WebSocketClient::new(&config.api_url, config.api_token.clone());

        // Connect to WebSocket - handle connection errors gracefully
        let ws_rx = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(ws_client.connect())
        })
        .map_err(|e| {
            // Return a more user-friendly error
            anyhow::anyhow!("Failed to connect to API at {}: {}", config.api_url, e)
        })?;

        let mut app = Self {
            state: AppState::new(config.time_window_seconds),
            config,
            ws_rx,
            http_client,
        };

        // Mark as not connected initially - will be updated when first event arrives
        app.state.connected = false;

        Ok(app)
    }

    /// Build an authenticated GET request to the API
    fn build_authenticated_request(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.config.api_url, path);
        let mut request = self.http_client.get(url);

        if let Some(token) = &self.config.api_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        request
    }

    /// Run the application
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Calculate data limit based on terminal width
        let terminal_size = terminal.size()?;
        let chart_width = (terminal_size.width as f64 * 0.7) as usize; // 70% is chart area
        let data_limit = (chart_width * 2).clamp(20, 500); // 2 points per char, min 20, max 500

        tracing::debug!(
            "Terminal size: {}x{}, chart width: {}, data limit: {}",
            terminal_size.width,
            terminal_size.height,
            chart_width,
            data_limit
        );

        // Update state with calculated data limit
        self.state.data_limit = data_limit;

        // Initial data fetch - load server/service lists and historical metrics
        // This provides immediate visualization instead of waiting for WebSocket data
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
    ///
    /// Loads server/service lists and historical metrics for immediate visualization.
    /// After this, the WebSocket provides real-time updates.
    async fn fetch_initial_data(&mut self) -> Result<()> {
        // Fetch servers
        let request = self.build_authenticated_request("/api/v1/servers");

        match request.send().await {
            Ok(response) if response.status().is_success() => {
                match response.json::<crate::api::ServersResponse>().await {
                    Ok(servers_response) => {
                        self.state.update_servers(servers_response.servers);
                        self.state.connected = true;

                        // Fetch historical metrics for each server
                        self.fetch_historical_metrics().await;
                    }
                    Err(e) => {
                        self.state.error_message = Some(format!("Failed to parse servers: {}", e));
                    }
                }
            }
            Ok(response) => {
                self.state.error_message = Some(format!("API error: {}", response.status()));
            }
            Err(e) => {
                self.state.error_message = Some(format!("Connection failed: {}", e));
            }
        }

        // Fetch services
        let request = self.build_authenticated_request("/api/v1/services");

        if let Ok(response) = request.send().await
            && response.status().is_success()
            && let Ok(services_response) = response.json::<crate::api::ServicesResponse>().await
        {
            self.state.update_services(services_response.services);
        }

        Ok(())
    }

    /// Fetch historical metrics for all servers
    async fn fetch_historical_metrics(&mut self) {
        // Use terminal-width-aware data limit for optimal visualization
        let limit = self.state.data_limit;

        let servers = self.state.servers.clone(); // Clone to avoid borrow issues

        for server in servers {
            let mut request = self.http_client.get(format!(
                "{}/api/v1/servers/{}/metrics/latest?limit={}",
                self.config.api_url, server.server_id, limit
            ));

            if let Some(token) = &self.config.api_token {
                request = request.header("Authorization", format!("Bearer {}", token));
            }

            match request.send().await {
                Ok(response) if response.status().is_success() => {
                    // Direct deserialization - no double parsing!
                    match response.json::<crate::api::LatestMetricsResponse>().await {
                        Ok(metrics_response) => {
                            // Reverse so oldest metrics are first (API returns newest first)
                            let mut metrics = metrics_response.metrics;
                            metrics.reverse();

                            // Add to history in chronological order
                            for metric_row in metrics {
                                self.state.add_metric(
                                    metric_row.server_id,
                                    metric_row.metadata,
                                    metric_row.timestamp,
                                );
                            }

                            tracing::debug!(
                                "Loaded {} historical metrics for {}",
                                metrics_response.count,
                                server.server_id
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to parse metrics response for {}: {}",
                                server.server_id,
                                e
                            );
                        }
                    }
                }
                Ok(response) => {
                    tracing::error!(
                        "HTTP error fetching historical metrics for {}: {}",
                        server.server_id,
                        response.status()
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to fetch historical metrics for {}: {}",
                        server.server_id,
                        e
                    );
                }
            }
        }
    }

    /// Main event loop
    async fn run_event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        let mut last_refresh = std::time::Instant::now();

        loop {
            // Render UI
            terminal.draw(|f| ui::render(f, &self.state))?;

            // Handle WebSocket events (non-blocking)
            while let Ok(event) = self.ws_rx.try_recv() {
                self.handle_ws_event(event);
            }

            // Handle keyboard events (with timeout)
            if event::poll(std::time::Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && self.handle_key_event(key.code).await?
            {
                break; // Quit
            }

            // Periodic refresh - only update server/service lists
            // WebSocket provides real-time metric updates, so no need to refetch historical data
            if last_refresh.elapsed().as_secs() >= self.config.refresh_interval {
                self.refresh_server_list().await.ok(); // Ignore errors
                last_refresh = std::time::Instant::now();
            }

            // Check connection timeout (every 5 seconds)
            if !self.state.paused {
                self.state.check_connection_timeout(30); // 30 second timeout
            }
        }

        Ok(())
    }

    /// Refresh server and service lists (without refetching historical metrics)
    ///
    /// This lightweight refresh only updates the list of monitored servers/services.
    /// Metric data comes from WebSocket, so we don't need to refetch it periodically.
    async fn refresh_server_list(&mut self) -> Result<()> {
        // Fetch servers
        let request = self.build_authenticated_request("/api/v1/servers");

        if let Ok(response) = request.send().await
            && response.status().is_success()
            && let Ok(servers_response) = response.json::<crate::api::ServersResponse>().await
        {
            self.state.update_servers(servers_response.servers);
        }

        // Fetch services
        let request = self.build_authenticated_request("/api/v1/services");

        if let Ok(response) = request.send().await
            && response.status().is_success()
            && let Ok(services_response) = response.json::<crate::api::ServicesResponse>().await
        {
            self.state.update_services(services_response.services);
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
                // Clear error message when connectivity is restored via metric data
                if !self.state.connected {
                    self.state.error_message = None;
                }
                self.state.connected = true;
            }
            WsEvent::ServiceCheck {
                service_name,
                status,
                timestamp,
                error_message,
                ..
            } => {
                // Special handling for connection state changes
                if service_name == "Connection" {
                    match status {
                        crate::actors::messages::ServiceStatus::Up => {
                            self.state.connected = true;
                            self.state.error_message = None; // Clear any previous error
                        }
                        crate::actors::messages::ServiceStatus::Down => {
                            self.state.connected = false;
                            self.state.error_message = error_message;
                        }
                        _ => {}
                    }
                    return; // Don't treat connection events as service alerts
                }

                // Clear error message when connectivity is restored via service data
                // (for non-connection service events)
                if !self.state.connected {
                    self.state.error_message = None;
                }

                // Add to alerts if DOWN (but not for connection events)
                if matches!(status, crate::actors::messages::ServiceStatus::Down) {
                    self.state.add_alert(AlertEntry {
                        timestamp,
                        server_id: service_name.clone(),
                        alert_type: "Service Down".to_string(),
                        message: error_message
                            .unwrap_or_else(|| "Service check failed".to_string()),
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
