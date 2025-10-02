# Terminal Dashboard (TUI)

## Overview

This document describes the design and implementation of a beautiful terminal user interface (TUI) for visualizing server monitoring metrics in real-time. The dashboard will display historical trends, current status, and alerts using graphs, tables, and sparklines.

---

## Requirements

### Functional Requirements

1. **Real-time updates** from hub via WebSocket
2. **Multi-server view** with overview and per-server tabs
3. **Time-series charts** with CPU, temperature, memory
4. **Threshold visualization** (horizontal lines on graphs)
5. **Service status display** (uptime checks)
6. **Alert history** with severity indicators
7. **Interactive controls** (pause, zoom, time range selection)
8. **Works locally and remotely** (connect to hub API)

### Non-Functional Requirements

1. **Responsive:** Sub-100ms UI updates
2. **Efficient:** Low CPU usage (<5% idle)
3. **Resilient:** Reconnect on network failure
4. **Accessible:** Clear text, good contrast
5. **Cross-platform:** Works on macOS, Linux, Windows

---

## Technology Stack

### Core Libraries

| Library | Purpose | Why |
|---------|---------|-----|
| **Ratatui** | TUI framework | Modern, actively maintained, excellent docs |
| **Crossterm** | Terminal backend | Cross-platform, supports all major terminals |
| **Tokio-tungstenite** | WebSocket client | Async, integrates with tokio runtime |
| **Serde** | JSON (de)serialization | Standard Rust serialization |

```toml
# Cargo.toml additions
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
tokio-tungstenite = "0.24"
tokio = { version = "1.47", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
```

---

## UI Layout

### Tab Structure

```
┌─────────────────────────────────────────────────────────────────┐
│ Guardia Monitor │ [Overview] [Server-A] [Server-B] [Services]  │  ← Tab bar
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Main Content Area                                              │  ← Current tab content
│  (charts, tables, status)                                       │
│                                                                 │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ Status: ● Connected to hub | ⌚ 2025-01-15 14:32:05 | ? Help  │  ← Footer
└─────────────────────────────────────────────────────────────────┘
```

### Tab: Overview

```
┌─────────────────────────────────────────────────────────────────┐
│ Server Summary                                                  │
├────────────┬──────────┬─────────────┬──────────┬────────────────┤
│ Server     │ Status   │ CPU         │ Temp     │ Last Seen      │
├────────────┼──────────┼─────────────┼──────────┼────────────────┤
│ Server-A   │ 🟢 UP    │ ▂▃▅▇█▅▃▂ 75%│ 68.2°C  │ 2s ago         │
│ Server-B   │ 🟢 UP    │ ▂▂▃▃▄▄▅▅ 45%│ 55.1°C  │ 5s ago         │
│ Server-C   │ 🔴 DOWN  │ --          │ --      │ 2m ago         │
├────────────┴──────────┴─────────────┴──────────┴────────────────┤
│                                                                 │
│ 🔥 Recent Alerts (3)                                            │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│ 🔴 14:31 Server-A temperature exceeded 75°C (76.8°C)           │
│ 🟠 14:25 Server-B CPU usage high: 85%                          │
│ 🟢 14:20 Server-A temperature recovered: 68.2°C                │
└─────────────────────────────────────────────────────────────────┘
```

### Tab: Server Detail

```
┌─────────────────────────────────────────────────────────────────┐
│ Server-A (192.168.1.100:3000)                    Last: 2s ago   │
├─────────────────────────────────────────────────────────────────┤
│ CPU Usage (%)                                [Last 5 minutes]   │
│ 100 ┤                                    ╭─╮                    │
│  80 ┤                         ╭──╮      │ ╰╮                   │
│  60 ┤              ╭──╮      │  ╰╮  ╭─╯   ╰─╮   ← Threshold    │
│  40 ┤         ╭───╯  ╰──────╯   ╰──╯        ╰─╮                │
│  20 ┤    ╭────╯                               ╰────             │
│   0 ┼────╯                                                      │
│     └─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────    │
│         14:27  14:28  14:29  14:30  14:31  14:32               │
├─────────────────────────────────────────────────────────────────┤
│ Temperature (°C)                             [Last 5 minutes]   │
│  80 ┤                                                           │
│  70 ┤                                    ╭────  ← Threshold     │
│  60 ┤              ╭──╮      ╭──╮  ╭────╯                      │
│  50 ┤         ╭───╯  ╰──╮  │  ╰╮─╯                             │
│  40 ┤    ╭────╯         ╰──╯   ╰                               │
│  30 ┼────╯                                                      │
│     └─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────    │
├─────────────────────────────────────────────────────────────────┤
│ System Info                                                     │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│ OS: Ubuntu 22.04 LTS       │ Memory: 15.2 / 32.0 GB (47%)     │
│ Kernel: 5.15.0-91          │ Swap: 0.0 / 8.0 GB (0%)          │
│ CPU: AMD Ryzen 9 5950X     │ Uptime: 15d 7h 23m               │
└─────────────────────────────────────────────────────────────────┘
```

### Tab: Services

```
┌─────────────────────────────────────────────────────────────────┐
│ Service Health Checks                                           │
├────────────────┬─────────┬─────────────┬──────────┬────────────┤
│ Service        │ Status  │ Resp. Time  │ Uptime   │ Last Check │
├────────────────┼─────────┼─────────────┼──────────┼────────────┤
│ api.example.c…│ 🟢 UP   │ 45ms        │ 99.95%   │ 5s ago     │
│ db.example.com│ 🟢 UP   │ 12ms        │ 99.99%   │ 3s ago     │
│ cdn.example.c…│ 🟡 SLOW │ 250ms       │ 99.80%   │ 8s ago     │
│ old.example.c…│ 🔴 DOWN │ timeout     │ 45.20%   │ 15s ago    │
├────────────────┴─────────┴─────────────┴──────────┴────────────┤
│                                                                 │
│ Response Time (api.example.com)              [Last 10 minutes] │
│ 200ms ┤                         ╭╮                             │
│ 150ms ┤           ╭╮           │╰╮                            │
│ 100ms ┤    ╭╮    │╰╮  ╭╮     ╭╯ ╰╮     ╭╮                     │
│  50ms ┤────╯╰────╯ ╰──╯╰─────╯   ╰─────╯╰──────              │
│   0ms ┼─────────────────────────────────────────────           │
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Architecture

### Binary Structure

```rust
// src/bin/viewer.rs

use ratatui::prelude::*;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args
    let args = ViewerArgs::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(&args.hub_url).await?;

    // Run event loop
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
```

### App State

```rust
// src/viewer/app.rs

pub struct App {
    // Connection
    ws_client: WebSocketClient,
    connection_status: ConnectionStatus,

    // Data
    servers: HashMap<String, ServerState>,
    services: Vec<ServiceState>,
    alerts: Vec<Alert>,

    // UI State
    current_tab: Tab,
    tabs: Vec<Tab>,
    selected_time_range: TimeRange,
    paused: bool,

    // Events
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
}

pub enum Tab {
    Overview,
    ServerDetail(String),
    Services,
    Alerts,
}

pub enum AppEvent {
    MetricReceived(MetricEvent),
    ServiceCheckReceived(ServiceCheckResult),
    AlertReceived(Alert),
    ConnectionStatusChanged(ConnectionStatus),
    KeyPress(KeyCode),
    Tick,
}

impl App {
    pub async fn new(hub_url: &str) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Connect to hub WebSocket
        let ws_client = WebSocketClient::connect(hub_url, event_tx.clone()).await?;

        // Spawn tick timer
        let tick_tx = event_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(250));
            loop {
                interval.tick().await;
                let _ = tick_tx.send(AppEvent::Tick);
            }
        });

        // Spawn keyboard listener
        let key_tx = event_tx.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(Event::Key(key)) = event::read() {
                    let _ = key_tx.send(AppEvent::KeyPress(key.code));
                }
            }
        });

        Ok(Self {
            ws_client,
            connection_status: ConnectionStatus::Connected,
            servers: HashMap::new(),
            services: Vec::new(),
            alerts: Vec::new(),
            current_tab: Tab::Overview,
            tabs: vec![Tab::Overview],
            selected_time_range: TimeRange::Last5Minutes,
            paused: false,
            event_rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        while let Some(event) = self.event_rx.recv().await {
            self.handle_event(event).await?;
        }
        Ok(())
    }

    async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::MetricReceived(metric) => {
                self.update_server_metrics(metric);
            }
            AppEvent::ServiceCheckReceived(check) => {
                self.update_service_status(check);
            }
            AppEvent::AlertReceived(alert) => {
                self.alerts.insert(0, alert);
                if self.alerts.len() > 100 {
                    self.alerts.truncate(100);
                }
            }
            AppEvent::KeyPress(key) => {
                self.handle_keypress(key).await?;
            }
            AppEvent::Tick => {
                // Trigger UI refresh
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_keypress(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                // Quit
                std::process::exit(0);
            }
            KeyCode::Tab => {
                // Next tab
                self.next_tab();
            }
            KeyCode::BackTab => {
                // Previous tab
                self.previous_tab();
            }
            KeyCode::Char('p') => {
                // Pause/unpause
                self.paused = !self.paused;
            }
            KeyCode::Char('1'..='9') => {
                // Switch to tab by number
                let idx = key.to_string().parse::<usize>().unwrap() - 1;
                if idx < self.tabs.len() {
                    self.current_tab = self.tabs[idx].clone();
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

### WebSocket Client

```rust
// src/viewer/websocket.rs

use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{StreamExt, SinkExt};

pub struct WebSocketClient {
    // Internal state
}

impl WebSocketClient {
    pub async fn connect(url: &str, event_tx: mpsc::UnboundedSender<AppEvent>) -> Result<Self> {
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Spawn reader task
        tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Ok(event) = serde_json::from_str::<ServerEvent>(&text) {
                            match event {
                                ServerEvent::MetricCollected(m) => {
                                    let _ = event_tx.send(AppEvent::MetricReceived(m));
                                }
                                ServerEvent::ServiceCheck(s) => {
                                    let _ = event_tx.send(AppEvent::ServiceCheckReceived(s));
                                }
                                ServerEvent::Alert(a) => {
                                    let _ = event_tx.send(AppEvent::AlertReceived(a));
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        let _ = event_tx.send(AppEvent::ConnectionStatusChanged(
                            ConnectionStatus::Disconnected,
                        ));
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Attempt reconnection
            // ...
        });

        Ok(Self {})
    }
}
```

### Rendering Components

```rust
// src/viewer/ui/overview.rs

use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_overview(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Server table
            Constraint::Min(10),    // Alerts
        ])
        .split(area);

    render_server_table(frame, app, chunks[0]);
    render_alerts(frame, app, chunks[1]);
}

fn render_server_table(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec!["Server", "Status", "CPU", "Temp", "Last Seen"])
        .style(Style::default().fg(Color::Yellow).bold());

    let rows: Vec<Row> = app.servers.values().map(|server| {
        let status_icon = match server.status {
            ServerStatus::Up => "🟢 UP",
            ServerStatus::Down => "🔴 DOWN",
            ServerStatus::Unknown => "⚪ UNKNOWN",
        };

        let cpu_sparkline = render_sparkline(&server.cpu_history);

        Row::new(vec![
            Cell::from(server.display_name.clone()),
            Cell::from(status_icon),
            Cell::from(format!("{} {:.1}%", cpu_sparkline, server.current_cpu)),
            Cell::from(format!("{:.1}°C", server.current_temp)),
            Cell::from(format_duration(server.last_seen)),
        ])
    }).collect();

    let table = Table::new(rows, vec![
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(20),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Server Summary"));

    frame.render_widget(table, area);
}

fn render_sparkline(data: &VecDeque<f64>) -> String {
    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = data.iter().fold(0.0, |a, &b| a.max(b));

    data.iter()
        .map(|&val| {
            let idx = ((val / max) * (chars.len() - 1) as f64) as usize;
            chars[idx]
        })
        .collect()
}

// src/viewer/ui/server_detail.rs

pub fn render_server_detail(frame: &mut Frame, app: &App, server_id: &str, area: Rect) {
    let server = match app.servers.get(server_id) {
        Some(s) => s,
        None => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // CPU chart
            Constraint::Percentage(40), // Temp chart
            Constraint::Percentage(20), // System info
        ])
        .split(area);

    render_cpu_chart(frame, server, chunks[0]);
    render_temp_chart(frame, server, chunks[1]);
    render_system_info(frame, server, chunks[2]);
}

fn render_cpu_chart(frame: &mut Frame, server: &ServerState, area: Rect) {
    let data: Vec<(f64, f64)> = server
        .cpu_history
        .iter()
        .enumerate()
        .map(|(i, &val)| (i as f64, val))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name("CPU Usage")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&data),
        // Threshold line
        Dataset::default()
            .name("Threshold")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Red))
            .data(&vec![(0.0, 80.0), (data.len() as f64, 80.0)]),
    ];

    let x_max = data.len() as f64;
    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("CPU Usage (%) - Last {}", format_time_range(app.selected_time_range))),
        )
        .x_axis(
            Axis::default()
                .title("Time")
                .bounds([0.0, x_max])
                .labels(vec![
                    format_time(server.oldest_timestamp),
                    format_time(server.newest_timestamp),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Usage %")
                .bounds([0.0, 100.0])
                .labels(vec!["0", "20", "40", "60", "80", "100"]),
        );

    frame.render_widget(chart, area);
}
```

### Data Structures

```rust
// src/viewer/state.rs

use std::collections::VecDeque;

pub struct ServerState {
    pub id: String,
    pub display_name: String,
    pub ip: String,
    pub port: u16,
    pub status: ServerStatus,

    // Current values
    pub current_cpu: f64,
    pub current_temp: f64,
    pub current_memory_pct: f64,

    // History (ring buffers, max 300 points = 5 min at 1sec interval)
    pub cpu_history: VecDeque<f64>,
    pub temp_history: VecDeque<f64>,
    pub memory_history: VecDeque<f64>,

    pub last_seen: DateTime<Utc>,
    pub oldest_timestamp: DateTime<Utc>,
    pub newest_timestamp: DateTime<Utc>,
}

impl ServerState {
    pub fn new(id: String) -> Self {
        Self {
            id,
            display_name: String::new(),
            ip: String::new(),
            port: 0,
            status: ServerStatus::Unknown,
            current_cpu: 0.0,
            current_temp: 0.0,
            current_memory_pct: 0.0,
            cpu_history: VecDeque::with_capacity(300),
            temp_history: VecDeque::with_capacity(300),
            memory_history: VecDeque::with_capacity(300),
            last_seen: Utc::now(),
            oldest_timestamp: Utc::now(),
            newest_timestamp: Utc::now(),
        }
    }

    pub fn update_metrics(&mut self, metrics: &ServerMetrics) {
        self.current_cpu = metrics.cpus.average_usage as f64;
        self.current_temp = metrics.components.average_temperature.unwrap_or(0.0) as f64;
        self.current_memory_pct =
            (metrics.memory.used as f64 / metrics.memory.total as f64) * 100.0;

        // Add to history
        self.cpu_history.push_back(self.current_cpu);
        self.temp_history.push_back(self.current_temp);
        self.memory_history.push_back(self.current_memory_pct);

        // Trim old data
        if self.cpu_history.len() > 300 {
            self.cpu_history.pop_front();
            self.temp_history.pop_front();
            self.memory_history.pop_front();
        }

        self.last_seen = Utc::now();
        self.newest_timestamp = Utc::now();

        if self.cpu_history.len() == 1 {
            self.oldest_timestamp = Utc::now();
        }
    }
}

pub enum ServerStatus {
    Up,
    Down,
    Unknown,
}
```

---

## Configuration

```json
{
  "viewer": {
    "hub_url": "ws://localhost:8080/ws",
    "reconnect_interval_secs": 5,
    "theme": "dark",
    "refresh_rate_ms": 250,
    "history_points": 300
  }
}
```

---

## Features Roadmap

### Phase 1: Basic Display
- [x] Connect to WebSocket
- [x] Display server list
- [x] Show current metrics
- [x] Tab navigation

### Phase 2: Charts
- [ ] Implement Chart widget for CPU
- [ ] Add temperature chart
- [ ] Add memory chart
- [ ] Draw threshold lines

### Phase 3: Interactivity
- [ ] Pause/resume updates
- [ ] Time range selection (5m, 30m, 1h, 6h, 24h)
- [ ] Zoom in/out on charts
- [ ] Server filtering

### Phase 4: Services
- [ ] Service status table
- [ ] Response time charts
- [ ] Uptime percentages

### Phase 5: Polish
- [ ] Color themes
- [ ] Help screen
- [ ] Configuration file
- [ ] Status bar with stats

---

## Keybindings

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `1-9` | Jump to tab by number |
| `p` | Pause/resume updates |
| `r` | Refresh (force reload) |
| `t` | Cycle time range (5m → 30m → 1h → ...) |
| `+` / `-` | Zoom in/out |
| `?` / `F1` | Show help |
| `Arrow Keys` | Navigate within tab |
| `Enter` | Select item |

---

## Testing Strategy

1. **Visual Testing:** Manual inspection of layouts
2. **Unit Tests:** Test data structures and state updates
3. **Integration Tests:** Test WebSocket message handling
4. **Stress Tests:** 100+ servers updating every second

---

## Performance Considerations

1. **Limit history:** Max 300 points per metric (5 min at 1 sec intervals)
2. **Lazy rendering:** Only render visible tabs
3. **Throttle updates:** Max 4 FPS (250ms refresh)
4. **Efficient data structures:** Use VecDeque for ring buffers
5. **Minimize allocations:** Reuse buffers where possible

---

## Accessibility

1. Use ASCII fallback characters if Unicode not supported
2. High contrast colors
3. Clear labels and titles
4. Status indicators with both color and text

---

## Future Enhancements

- Export current view as PNG/SVG
- Record and replay sessions
- Multiple dashboard views (save/load layouts)
- Custom metric expressions (e.g., "cpu + io_wait")
- Alert annotations on charts
- Correlation view (overlay multiple servers)

---

## Success Criteria

- [ ] Displays 10+ servers smoothly
- [ ] Sub-100ms UI responsiveness
- [ ] Reconnects automatically on network failure
- [ ] Charts are readable and informative
- [ ] Works in 80x24 terminal minimum
- [ ] CPU usage <5% when idle
