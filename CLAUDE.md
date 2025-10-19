# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based server monitoring solution that uses a hub-agent architecture to monitor server resources (CPU usage and temperature) and send alerts when thresholds are exceeded.

**Current Version:** v0.9.0 (Pre-Release - nearing v1.0.0)

**Architecture:**
- **Agent (`src/bin/agent.rs`)**: Runs on monitored servers, exposes metrics via Rocket HTTP server on `/metrics` endpoint
- **Hub (`src/bin/hub.rs`)**: Central monitoring service using actor-based architecture with graceful shutdown
- **Viewer (`src/bin/viewer.rs`)**: TUI dashboard for real-time visualization with time-based charts

**Key Components:**
- `ServerMetrics`: Core data structure containing system info, memory, CPU, and component temperature data
- `AlertManager`: Handles sending alerts via Discord webhooks or generic webhooks
- Configuration is JSON-based (see `config.example.json`)

### Actor-Based Architecture (✅ COMPLETE)

The hub uses an actor-based architecture for better scalability and maintainability:

**Actors (`src/actors/`):**
- **MetricCollectorActor** (`collector.rs`): Polls agent endpoints at configured intervals, publishes metrics to broadcast channel
- **ServiceMonitorActor** (`service_monitor.rs`): Performs HTTP/HTTPS service health checks, publishes service check events to broadcast channel
- **AlertActor** (`alert.rs`): Subscribes to metrics and service checks, maintains grace period state, triggers alerts when thresholds exceeded
- **StorageActor** (`storage.rs`): Subscribes to metrics and service checks, persists to storage with pluggable backends (SQLite, in-memory)

**Communication:**
- Each actor has an `mpsc` command channel for control messages (poll now, shutdown, etc.)
- Metrics flow through a `broadcast` channel from collectors to alert/storage actors
- Service checks flow through a separate `broadcast` channel from service monitors to alert/storage actors
- Handles provide typed API for sending commands to actors

**Message Types (`messages.rs`):**
- `MetricEvent`: Published when metrics collected from a server
- `ServiceCheckEvent`: Published when service health check completes (UP/DOWN/DEGRADED)
- `CollectorCommand`, `ServiceMonitorCommand`, `AlertCommand`, `StorageCommand`: Control messages for each actor type

**Why Actor Model:**
- Loose coupling - actors only communicate via messages
- Testable - actors can be tested in isolation with mock channels
- Scalable - easy to add new metric consumers (API, dashboard, etc.)
- Supervision - actors can be monitored and restarted independently
- Efficiency - HTTP client reused across requests (old system created new client each poll)

**Phase 1 Status (✅ COMPLETE - January 15, 2025):**
- ✅ Core actor infrastructure implemented
- ✅ All actor types created with command/event channels
- ✅ Unit tests passing
- ✅ Actors integrated into hub.rs
- ✅ Graceful shutdown on Ctrl+C
- ✅ Feature parity verified with old implementation

**Phase 2 Status (✅ COMPLETE - January 15, 2025):**
- ✅ StorageBackend trait for pluggable persistence
- ✅ SQLite backend with batching (dual flush triggers: 100 metrics OR 5 seconds)
- ✅ Hybrid schema: indexed aggregates + full metadata
- ✅ StorageActor extended with `Option<Box<dyn StorageBackend>>`
- ✅ Configuration support for storage backends
- ✅ Backward compatible (falls back to in-memory mode)

**Phase 3 Status (✅ COMPLETE - January 15, 2025):**
- ✅ HTTP/HTTPS service check monitoring
- ✅ ServiceMonitorActor with configurable check intervals
- ✅ Service check persistence (SQLite + in-memory)
- ✅ Service status tracking (UP/DOWN/DEGRADED)
- ✅ Grace periods for flapping detection
- ✅ Alert integration (Discord + Webhook)
- ✅ Uptime calculation with statistics
- ✅ Public query API in StorageHandle
  - `query_service_checks_range()` - time range queries
  - `query_latest_service_checks()` - latest N checks
  - `calculate_uptime()` - uptime statistics
- ✅ Integration tests (persistence, uptime, range queries)

**Phase 4 Status (✅ COMPLETE - January 16, 2025):**

**✅ Phase 4.0: Retention & Cleanup COMPLETE**
- ✅ Startup cleanup (runs once on hub start)
- ✅ Configurable cleanup interval (default: 24 hours, range: 1-720 hours)
- ✅ Cleanup statistics tracking (`last_cleanup_time`, `total_metrics_deleted`, `total_service_checks_deleted`)
- ✅ Configuration validation (retention_days: 1-3650, cleanup_interval_hours: 1-720)
- ✅ Exposed in `StorageStats` via `GetStats` command
- ✅ Background task in StorageActor

**✅ Phase 4.1: API Server COMPLETE**
- ✅ REST API with Axum framework (`src/api/`)
- ✅ All core endpoints implemented:
  - `GET /api/v1/health` - Health check with timestamp
  - `GET /api/v1/stats` - System statistics (storage, actors)
  - `GET /api/v1/servers` - List servers with **real health status** (up/stale/unknown)
  - `GET /api/v1/servers/:id/metrics` - Query metrics with time range
  - `GET /api/v1/servers/:id/metrics/latest` - Latest N metrics
  - `GET /api/v1/services` - List services with **real health status** (up/down/degraded/stale/unknown)
  - `GET /api/v1/services/:name/checks` - Service check history
  - `GET /api/v1/services/:name/uptime` - Uptime statistics
  - `WS /api/v1/stream` - Real-time metric/service check streaming
- ✅ Bearer token authentication middleware
- ✅ CORS support for web dashboards
- ✅ WebSocket streaming with broadcast channel integration
- ✅ API configuration via JSON config file
- ✅ Feature flag: `api` (enabled by default)
- ✅ All tests passing (55/55)

**✅ Phase 4.2: TUI Dashboard COMPLETE**
- ✅ Ratatui-based terminal UI (`guardia-viewer` binary)
- ✅ WebSocket client for real-time metric/service check streaming
- ✅ Three-tab interface: Servers, Services, Alerts
- ✅ **Time-based charts with sliding window** (X-axis shows actual timestamps in HH:MM:SS format)
  - CPU usage chart with time-based X-axis and configurable window (default: 5 minutes)
  - Temperature chart with time-based X-axis
  - Historical data loading on startup (queries `/api/v1/servers/:id/metrics/latest`)
  - Automatic cleanup of metrics older than 2x time window
- ✅ **Enhanced memory visualization**
  - Color-coded memory gauge (green <70%, yellow <85%, red ≥85%)
  - Progress bars for RAM and Swap usage
  - Absolute values (GB) + percentages
- ✅ **Enhanced system information panel**
  - Hostname, OS, architecture
  - Quick metrics summary (CPU, temperature, memory)
- ✅ **Shared type architecture** (`src/api/types.rs`)
  - ServerInfo and ServiceInfo shared between API and viewer
  - Prevents serialization mismatches and type drift
- ✅ Server health status display (up/stale/unknown)
- ✅ Service health monitoring (up/down/degraded/stale/unknown)
- ✅ Alert timeline with severity indicators (Critical/Warning/Info)
- ✅ Keybindings: Tab navigation, arrow keys, space to pause, R to refresh, Q to quit
- ✅ TOML configuration support (`~/.config/guardia/viewer.toml`)
  - `api_url`, `api_token`, `refresh_interval`, `max_metrics`
  - `time_window_seconds` (default: 300 = 5 minutes)
- ✅ Automatic reconnection on WebSocket disconnect
- ✅ Time-based metric cleanup (removes metrics older than 2x window)
- ✅ Feature flag: `dashboard` (enabled by default)
- See "TUI Dashboard Architecture" section below for implementation details

**Testing Status (October 16, 2025):**
- ✅ **55 tests passing** (100% pass rate):
  - ~32 unit tests (actors, storage backends, service monitoring, utilities)
  - 43 integration tests (actor communication, persistence, API endpoints)
  - 9 property-based tests (grace period invariants)
  - 3 doc tests
- See **TESTING.md** for comprehensive test documentation with running instructions
- Run tests: `cargo test --workspace --all-features`
- Actor tests: `cargo test --lib` or `just test`
- Integration tests: `cargo test --test '*'`
- Property tests: `cargo test --test property_tests`
- All actors have comprehensive unit tests in their respective modules
- Integration tests for actor communication in `tests/integration/`

**Legacy Code Status:**
- ✅ Old `monitors/server.rs` and `monitors/resources.rs` cleaned up (Phase 1)
- ✅ `ResourceEvaluation` logic migrated to AlertActor
- All monitoring now uses actor-based architecture

### Metric Persistence (✅ Phase 2 - SQLite Complete)

The system now supports persistent metric storage through a pluggable backend architecture:

**Storage Architecture (`src/storage/`):**
- **StorageBackend trait** (`backend.rs`): Async trait for all storage operations
  - `insert_batch()`: Batch write metrics (optimized for throughput)
  - `query_range()`: Query metrics within time range
  - `query_latest()`: Get most recent N metrics for a server
  - `cleanup_old_metrics()`: Prune data older than retention period
  - `health_check()`: Backend health status
- **SqliteBackend** (`sqlite.rs`): SQLite implementation with WAL mode
- **MemoryBackend** (`memory.rs`): In-memory fallback (no persistence)
- **MetricRow** (`schema.rs`): Flattened metric schema for storage

**Schema Design:**
- **Hybrid approach**: Aggregate columns (indexed) + complete metadata (JSON)
- Indexed fields: `server_id`, `timestamp`, `metric_type`, `cpu_avg`, `temp_avg`
- Full `ServerMetrics` struct stored in `metadata` column for complete data access
- Primary key: `(server_id, timestamp)` for efficient time-series queries

**Batching Strategy:**
- **Size trigger**: Flush after 100 metrics (prevents unbounded memory growth)
- **Time trigger**: Flush after 5 seconds (ensures data freshness)
- Whichever trigger fires first initiates the batch write
- Final flush on shutdown to prevent data loss

**Configuration:**
```json
{
  "storage": {
    "backend": "sqlite",
    "path": "./metrics.db",
    "retention_days": 30,
    "cleanup_interval_hours": 24
  }
}
```

Or use in-memory mode (no persistence):
```json
{
  "storage": {
    "backend": "none"
  }
}
```

**Retention & Cleanup (Phase 4):**
- `retention_days`: How long to keep metrics (1-3650 days, default: 30)
- `cleanup_interval_hours`: How often to run cleanup (1-720 hours, default: 24)
- Cleanup runs automatically on startup and at configured intervals
- Statistics tracked: `last_cleanup_time`, `total_metrics_deleted`, `total_service_checks_deleted`

**Backward Compatibility:**
- `storage` config section is optional - defaults to in-memory if omitted
- Feature flag `storage-sqlite` (enabled by default) - can be disabled at compile time
- StorageActor gracefully falls back to in-memory mode if backend init fails

**Feature Flags:**
- `storage-sqlite`: Enables SQLite backend (requires sqlx dependency)
- `api`: Enables REST API and WebSocket server (requires axum, tower, tower-http)
- `dashboard`: Enables TUI dashboard viewer (requires ratatui, crossterm)
- Build minimal hub: `cargo build --bin hub --no-default-features`

### API Server (✅ Phase 4.1 - Complete)

The system includes a REST API server with WebSocket streaming for remote access and dashboards.

**API Module Structure (`src/api/`):**
- `mod.rs` - API server initialization and routing setup
- `types.rs` - **Shared response types** used by both API and viewer (ServerInfo, ServiceInfo, etc.)
- `state.rs` - API application state and handler context
- `error.rs` - Error types and response formatting
- `utils.rs` - Utility functions for response building and health status calculation
- `websocket.rs` - WebSocket connection handling and broadcast integration
- `routes/` - Endpoint implementations organized by resource type
  - `health.rs` - Health check endpoint
  - `stats.rs` - Statistics endpoint
  - `servers.rs` - Server list and metrics endpoints
  - `services.rs` - Service list and check endpoints
- `middleware/` - Request/response middleware
  - `auth.rs` - Bearer token authentication
  - `cors.rs` - CORS configuration

**API Endpoints:**
- `GET /api/v1/health` - Health check with timestamp
- `GET /api/v1/stats` - System statistics (storage stats, actor counts)
- `GET /api/v1/servers` - List all servers with health status (up/stale/unknown)
- `GET /api/v1/servers/:id/metrics?start=&end=&limit=` - Query metrics within time range
- `GET /api/v1/servers/:id/metrics/latest?limit=` - Get latest N metrics
- `GET /api/v1/services` - List all services with health status (up/down/degraded/stale/unknown)
- `GET /api/v1/services/:name/checks?start=&end=` - Service check history
- `GET /api/v1/services/:name/uptime?since=` - Uptime statistics
- `WS /api/v1/stream` - Real-time metric and service check streaming

**Health Status Values:**
- **Servers**: `"up"` (recent metrics), `"stale"` (>5min old), `"unknown"` (no metrics)
- **Services**: `"up"`, `"down"`, `"degraded"`, `"stale"` (>5min old), `"unknown"` (no checks)

**Configuration:**
```json
{
  "api": {
    "bind": "127.0.0.1",
    "port": 8080,
    "auth_token": "your-secret-token",
    "enable_cors": true
  }
}
```

**Authentication:**
- Bearer token authentication via `Authorization: Bearer <token>` header
- Optional - if `auth_token` not configured, API is unauthenticated
- Returns `401 Unauthorized` if token missing, `403 Forbidden` if invalid

**Starting the API:**
```bash
# Add API configuration to config.json
cargo run --bin hub -- -f config.json
# API will start automatically: "API server started on http://127.0.0.1:8080"
```

### TUI Dashboard Architecture (✅ Phase 4.2 - COMPLETE)

A beautiful terminal dashboard for monitoring servers and services in real-time.

**Binary:** `src/bin/viewer.rs` → `guardia-viewer`

**Implementation (`src/viewer/`):**
- `app.rs` - Main application loop, event handling, WebSocket integration
  - Historical metrics loading on startup
  - Periodic API refresh
  - WebSocket event handling
- `config.rs` - TOML configuration loading from `~/.config/guardia/viewer.toml`
- `state.rs` - Application state management
  - Time-based metric ring buffers with automatic cleanup
  - Tab navigation and selection state
  - Server/service/alert lists
- `websocket.rs` - WebSocket client with automatic reconnection
- `ui/` - Ratatui-based UI modules

**Shared Types (`src/api/types.rs`):**
- `ServerInfo` - Shared server response type (used by API and viewer)
- `ServiceInfo` - Shared service response type (used by API and viewer)
- **Purpose**: Prevents serialization mismatches and type drift between API and viewer
- **Benefit**: Single source of truth for API response structures
- **Location**: Centralized in `src/api/types.rs` for both API routes and viewer consumption

**Architecture:**
- Connects to API server (local or remote) via HTTP + WebSocket
- Real-time updates via `/api/v1/stream` WebSocket endpoint
- Initial data fetch via REST endpoints (`/api/v1/servers`, `/api/v1/services`)
- Periodic refresh (configurable interval, default 5s)
- Configuration file: `~/.config/guardia/viewer.toml` or via CLI args

**UI Design (Ratatui + Crossterm):**

**Tab 1: Servers** - Server monitoring
- **Left panel**: Server list with health status indicators (●)
  - Color coding: green (up), yellow (stale), gray (unknown)
  - Shows display name and status badge
  - Arrow keys to select servers
- **Right panel**: Selected server details
  - **Enhanced server info panel**: hostname, OS, architecture, quick metrics summary
  - **Color-coded memory gauge**: RAM and Swap with progress bars (█/░), color-coded by usage
  - **Time-based CPU chart** (line graph with Braille markers)
    - X-axis shows actual timestamps (HH:MM:SS format)
    - Sliding window (default 5 minutes, configurable)
    - Displays only metrics within time window
  - **Time-based temperature chart** (line graph with Braille markers)
    - X-axis shows actual timestamps (HH:MM:SS format)
    - Same sliding window behavior as CPU chart
  - Charts auto-update as WebSocket events arrive
  - **Historical data loading**: queries past metrics on startup for immediate visualization

**Tab 2: Services** - Service health monitoring
- **Table view**: Service list with columns
  - Status indicator (●), Service Name, URL, Last Check
  - Color-coded: green (up), red (down), yellow (degraded), magenta (stale), gray (unknown)
  - Arrow keys to navigate
- **Detail panel**: Selected service details
  - Service name, URL, status, monitoring status, last check, last status

**Tab 3: Alerts** - Alert timeline
- **Scrollable list** of alerts (newest first, max 500 in memory)
- Shows: timestamp, severity icon (⚠/⚡/ℹ), server/service ID, alert type, message
- Color-coded by severity: red (critical), yellow (warning), blue (info)
- Currently captures service DOWN events automatically

**Implemented Widgets (`src/viewer/ui/`):**
- `layout.rs` - Main dashboard layout with header (tabs), content area, footer (keybindings/status)
- `servers.rs` - Server list + enhanced metrics display (system info + memory gauge + time-based charts)
- `services.rs` - Service table + detail panel
- `alerts.rs` - Alert timeline with severity indicators
- `widgets.rs` - Reusable chart widgets
  - `render_cpu_chart` - Time-based CPU chart with sliding window and HH:MM:SS labels
  - `render_temp_chart` - Time-based temperature chart with sliding window
  - `render_memory_gauge` - Color-coded memory/swap gauge with progress bars

**Keybindings:**
- `Tab` / `→` - Next tab
- `Shift+Tab` / `←` - Previous tab
- `↑` / `k` - Select previous item (in lists/tables)
- `↓` / `j` - Select next item (in lists/tables)
- `Space` - Pause/resume real-time updates
- `r` / `R` - Force refresh (re-query API)
- `c` - Clear error message
- `q` / `Q` / `Esc` - Quit

**Data Flow:**
1. Viewer connects to API server on startup
2. Fetches initial state via REST endpoints (`/api/v1/servers`, `/api/v1/services`)
3. **Loads historical metrics** for each server (`/api/v1/servers/:id/metrics/latest?limit=N`)
   - N calculated based on `time_window_seconds` (e.g., 5 min window = ~30 points at 10s intervals)
   - Populates charts immediately with historical data
4. Opens WebSocket to `/api/v1/stream` (automatic reconnection on disconnect)
5. Receives `MetricEvent` and `ServiceCheckEvent` messages from WebSocket
6. Updates in-memory ring buffers with **time-based cleanup**
   - Metrics older than 2x `time_window_seconds` are automatically removed
   - Safety limit: max 1000 metrics per server, 500 alerts total
7. Renders UI on state changes or keyboard events (event-driven, ~10 FPS polling)
   - Charts filter to only show metrics within `time_window_seconds` window
   - X-axis displays actual timestamps (HH:MM:SS format)
8. Periodic refresh (default 5s) re-fetches server/service lists from API
9. Paused mode freezes updates but maintains WebSocket connection

**Configuration (`~/.config/guardia/viewer.toml`):**
```toml
# API server URL
api_url = "http://localhost:8080"

# Optional API authentication token
api_token = "your-api-token-here"

# Refresh interval in seconds (default: 5)
refresh_interval = 5

# Maximum metrics to keep in memory per server (default: 100)
max_metrics = 100

# Chart time window in seconds (default: 300 = 5 minutes)
# Determines how much historical data to display in charts
# Metrics older than 2x this value are automatically cleaned up
time_window_seconds = 300

# Enable debug mode (default: false)
debug = false
```

**Using the Template:**
- Copy `viewer.example.toml` from repository as a starting point
- Save to `~/.config/guardia/viewer.toml` (viewer looks here by default)
- Or specify custom location with `--config` flag

**Running the Viewer:**
```bash
# With default config (~/.config/guardia/viewer.toml)
guardia-viewer

# With custom config file
guardia-viewer --config viewer.toml

# Override API URL and token via CLI
guardia-viewer --url http://remote-server:8080 --token secret123
```

**Dependencies:**
- `ratatui` (0.29) - Terminal UI framework
- `crossterm` (0.28) - Terminal manipulation
- `tokio-tungstenite` (0.24) - WebSocket client
- `toml` (0.8) - Config file parsing
- `dirs` (6.0) - Cross-platform config directory

**Implementation Notes:**
- WebSocket client runs in background tokio task with automatic reconnection
- Events sent to app via `mpsc::unbounded_channel`
- State updates are non-blocking (try_recv in event loop)
- Charts use Ratatui's `Chart` widget with Braille markers for smooth rendering
- Ring buffers (VecDeque) for metric history prevent unbounded memory growth

### Web Dashboard (✅ Phase 5 - Modern Browser UI - NEW)

A modern, responsive web dashboard built with React, TypeScript, and Vite. Complements the TUI viewer with a browser-based interface.

**Binary/Files:** `web-dashboard/` directory with React + TypeScript project

**Key Features:**
- Modern dark-themed UI with Tailwind CSS
- Apache ECharts for professional visualizations
- Real-time updates via WebSocket
- Responsive design (desktop and tablet)
- Per-core CPU charts with legend
- Temperature monitoring with component breakdown
- Memory gauges with color-coded usage levels
- Service health dashboard
- Alert timeline
- Built-in to hub binary serving via Axum

**Technology Stack (`web-dashboard/src/`):**
- **React 18** + TypeScript - UI framework
- **Vite** - Build tool and dev server
- **Apache ECharts** - Beautiful charts (`components/servers/CpuChart.tsx`, `TemperatureChart.tsx`)
- **Tailwind CSS** - Styling with custom dark theme
- **Zustand** - Lightweight state management (`stores/monitoringStore.ts`)
- **Lucide React** - Icons

**Architecture (`web-dashboard/src/`):**
- `api/client.ts` - HTTP + WebSocket API client (connects to `/api/v1/*`)
- `api/types.ts` - TypeScript mirrors of Rust API types
- `stores/monitoringStore.ts` - Central state (servers, services, metrics, alerts)
- `hooks/useWebSocket.ts` - WebSocket connection with auto-reconnection
- `hooks/useMonitoring.ts` - Initial data loading and periodic refresh
- `components/layout/` - Header, sidebar, main layout
- `components/servers/` - Server list, detail, CPU/temp/memory charts
- `components/services/` - Service list and details
- `components/alerts/` - Alert timeline

**UI Components:**

**Layout:**
- `Header`: Connection status indicator, refresh button, app title
- `Sidebar`: Tab navigation (Servers, Services, Alerts)
- `Layout`: Main layout wrapper with responsive grid

**Servers Tab:**
- **Server List** (left panel): Card-based list with health status badges
  - Color coding: green (up), yellow (stale), gray (unknown)
  - Click to select and view details
- **Server Detail** (right panel):
  - System info panel: hostname, OS, arch, kernel
  - CPU breakdown: per-core with progress bars
  - Memory: RAM and Swap with color-coded gauges
  - Temperatures: Component list with current values
  - **CPU Chart**: ECharts line chart with:
    - Per-core breakdown (multi-line)
    - Average overlay
    - Time-based X-axis (HH:MM:SS)
    - Sliding window (default 5 min, configurable)
    - Interactive legend
  - **Temperature Chart**: ECharts line chart with:
    - Per-component tracking
    - Color-coded lines by temperature range
    - Time-based X-axis
    - Tooltip with values

**Services Tab:**
- **Service Table**: Columns for name, URL, status, response time, last check
- Color-coded rows: green (up), red (down), yellow (degraded), gray (stale/unknown)
- Click for additional details

**Alerts Tab:**
- **Timeline**: Reverse-chronological alert list
- Severity indicators: ⚡ (critical), ⚠️ (warning), ℹ️ (info)
- Color-coded by severity
- Shows timestamp, service/server ID, alert message

**Real-time Integration:**
- WebSocket connects to `/api/v1/stream` on hub
- Auto-reconnection with exponential backoff (max 10 attempts)
- Handles `MetricEvent` and `ServiceCheckEvent` messages
- Updates charts in real-time
- Historical data loaded on server selection (150 points = ~5 min at 2s intervals)

**Configuration (`web-dashboard/vite.config.ts`):**
- Dev server proxies `/api` to `http://localhost:8080`
- Production build to `dist/` directory
- Code splitting for ECharts library
- Sourcemaps disabled for production

**Build & Deployment:**
```bash
# Development
cd web-dashboard
npm install
npm run dev          # http://localhost:5173, proxies to hub on 8080

# Production
npm run build        # Outputs to dist/
```

The hub serves the dashboard:
- Static files from `web-dashboard/dist/` when available
- Served at `/` (root) - API routes at `/api/v1/*`
- SPA routing fallback to `index.html`
- Automatic gzip/brotli compression

**Feature Flag:** `web-dashboard` (enabled by default, depends on `api` feature)

**Development Notes:**
- Types kept in sync with Rust API via `src/api/types.ts`
- Zustand store manages all state
- WebSocket auto-reconnection handles hub restarts gracefully
- Ring buffers (VecDeque) limit memory per server: 1000 metrics, 500 alerts total
- Time-based cleanup: metrics older than 2x time window removed automatically
- No external API documentation needed - types ensure consistency

## Development Commands

### Quick Reference with Justfile

This project includes a `justfile` for common development tasks. All commands below can be shortened using `just`:

```bash
just build          # cargo build
just build-release  # cargo build --release
just test           # cargo test --workspace
just watch          # cargo watch -x "build --bins"
just install        # cargo install --path .
just bins           # cargo build --bins
just bins-release   # cargo build --bins --release
```

### Building
```bash
# Development build
cargo build
# or
just build

# Release build (optimized)
cargo build --release
# or
just build-release

# Build only binaries (agent + hub + viewer)
cargo build --bins
# or
just bins
```

### Testing

See **[TESTING.md](TESTING.md)** for comprehensive test documentation.

```bash
# Run all tests
cargo test --workspace --all-features
# or
just test

# Run specific test types:
cargo test --lib                    # Unit tests only
cargo test --test property_tests    # Property-based tests
cargo test --test integration_tests # Integration tests
cargo test --doc                    # Documentation tests

# Run a specific test
cargo test test_grace_period_temperature_increments_until_alert

# Run with output (no capture)
cargo test -- --nocapture
```

### Running
```bash
# Watch mode (rebuilds binaries on changes)
cargo watch -x "build --bins"
# or
just watch

# Run hub with config file
cargo run --bin hub -- -f config.json

# Run agent (uses environment variables)
cargo run --bin agent

# Run viewer
cargo run --bin viewer -- --url http://localhost:8080
```

### Installing
```bash
# Install binaries to cargo bin
cargo install --path .
# or
just install

# Install agent as systemd service (Linux only, requires root)
sudo ./install.sh
```

## Feature Flags & Build Variants

The project uses feature flags to customize builds. All features are enabled by default.

**Available Features:**
- `storage-sqlite`: SQLite backend for metric persistence (default: enabled)
- `api`: REST API and WebSocket server (default: enabled)
- `dashboard`: TUI viewer dependencies (default: enabled)

**Build Examples:**

```bash
# Default build (all features: storage + api + dashboard)
cargo build

# Build without dashboard (API + storage only)
cargo build --no-default-features --features "storage-sqlite,api"

# Build without storage (API + dashboard only, in-memory metrics)
cargo build --no-default-features --features "api,dashboard"

# Minimal hub (storage only, no API or dashboard)
cargo build --bin hub --no-default-features --features "storage-sqlite"

# API-only hub (no storage, no dashboard)
cargo build --bin hub --no-default-features --features "api"

# Viewer-only build (requires API feature for shared types)
cargo build --bin viewer --no-default-features --features "dashboard"
```

## Configuration

The hub requires a JSON config file specifying servers to monitor. The configuration system supports **global alert definitions** and **defaults** to reduce duplication.

### Key Configuration Concepts

- **Alert Registry** (`alerts`): Named, reusable alert configurations (Discord, Webhook)
- **Defaults** (`defaults`): Default settings for servers and services (limits, intervals, alerts)
- **Alert References**: Servers and services reference alerts by name instead of inline definitions
- **Storage** (optional): Backend for metric persistence (`sqlite` or `none`/omitted for in-memory)
- **Grace periods**: Number of consecutive exceeded measurements before alerting
- **Limits**: Separate thresholds for `temperature` (°C) and `usage` (CPU %)
- **Tokens**: Optional `X-MONITORING-SECRET` header for agent authentication

### Configuration Structure

```json
{
  "alerts": {
    "prod-critical": {
      "discord": {
        "url": "https://discord.com/api/webhooks/...",
        "user_id": "123456789"
      }
    },
    "dev-team": {
      "discord": { "url": "..." }
    }
  },
  "defaults": {
    "server": {
      "interval": 30,
      "limits": {
        "temperature": { "limit": 75, "grace": 3, "alert": "prod-critical" },
        "usage": { "limit": 80, "grace": 5, "alert": "prod-critical" }
      }
    },
    "service": {
      "interval": 60,
      "timeout": 10,
      "grace": 3,
      "alert": "prod-critical"
    }
  },
  "servers": [
    {
      "ip": "192.168.1.100",
      "display": "Production Server",
      "port": 3000
      // Inherits all defaults
    },
    {
      "ip": "192.168.1.101",
      "limits": {
        "temperature": {
          "limit": 85,         // Override specific value
          "alert": "dev-team"  // Override alert
        }
      }
    }
  ],
  "services": [
    { "name": "API", "url": "https://api.example.com" },  // Uses defaults
    { "name": "Website", "url": "https://example.com", "alert": "dev-team" }
  ]
}
```

### Configuration Resolution

The hub resolves the configuration on startup by:
1. Loading named alerts from the `alerts` registry
2. Merging `defaults` with server/service specific overrides
3. Replacing alert name references with actual `Alert` objects
4. Validating all alert references exist

This happens via `Config::resolve()` → `ResolvedConfig` before spawning actors.

See `config.example.json` for a complete configuration example with detailed comments.

Agent configuration via environment variables:
- `AGENT_ADDR`: Bind address (default: 0.0.0.0)
- `AGENT_PORT`: HTTP port (default: 3000)
- `AGENT_SECRET`: Optional authentication token

## Alert Flow (Actor-Based)

1. **CollectorActor** polls agent HTTP endpoint at configured intervals
2. Metrics published to broadcast channel as `MetricEvent`
3. **AlertActor** receives events and evaluates against thresholds using grace period state machine
4. On `StartsToExceed` (grace period exhausted) or `BackToOk` (recovered), `AlertManager` sends alerts
5. Alerts formatted and sent via Discord (with embeds) or webhook (JSON payload)
6. **StorageActor** receives events and persists to storage backend (SQLite with batching, or in-memory)

## Binary Structure

The project produces three binaries from `src/bin/`:
- `guardia-agent`: The monitoring agent (runs on monitored servers)
- `guardia-hub`: The central monitoring hub (processes metrics and sends alerts)
- `guardia-viewer`: The TUI dashboard (displays real-time metrics)

All three binaries share common code from `src/lib.rs` and its modules.

## Resource Evaluation States

- `Ok`: Below limit
- `Exceeding`: Above limit but within grace period
- `StartsToExceed`: Grace period exhausted, triggers alert
- `BackToOk`: Returned to normal after exceeding, triggers recovery alert

## Known Technical Debt

### Alert Architecture (Phase 3.5 - Medium Priority)

**Current State:**
- `AlertManager` (`src/alerts.rs`) handles both metric alerts (CPU, temp) and service alerts (uptime)
- Single manager with methods for different alert types (`send_temperature_alert`, `send_usage_alert`, `send_service_alert`)
- See TODO comment in `src/alerts.rs:197` acknowledging this should be split

**Why Split:**
- Different concerns: metrics (resource monitoring) vs services (availability monitoring)
- Different alert patterns: metrics use threshold percentages, services need SLA tracking
- Future extensibility: easier to add new alert types (log alerts, security alerts, disk space)
- Better separation of concerns and testability

**Proposed Refactoring:**
```rust
// Shared alert delivery abstraction
trait AlertSender {
    async fn send_discord(&self, discord: &Discord, message: &Message);
    async fn send_webhook(&self, webhook: &Webhook, payload: &Value);
}

// Metric-specific alerts (CPU, temp, disk, memory)
struct MetricAlertManager {
    sender: Box<dyn AlertSender>,
    server_config: ServerConfig,
}

// Service-specific alerts (uptime, SSL expiry, response times)
struct ServiceAlertManager {
    sender: Box<dyn AlertSender>,
    server_config: ServerConfig,
}
```

**When to Do:**
- **Priority**: Medium - after v1.0.0 release
- **Reason**: Current implementation works without bugs or performance issues
- **Timing**: Post-release refactoring (v1.1.0 cycle)
- **Before**: Adding more alert types (would compound the technical debt)

See [ROADMAP.md Phase 3.5](ROADMAP.md#phase-35-alert-architecture-refactoring-) for full plan.

## Current Development Focus (Phase 5: Production Readiness)

**Status:** v0.9.0 (Pre-Release) → Target: v1.0.0 in Q1 2025

**What's Complete:**
- ✅ All core features implemented (Phases 1-4 complete)
- ✅ Actor-based architecture with graceful shutdown
- ✅ SQLite persistence with configurable retention
- ✅ Service health monitoring with uptime tracking
- ✅ REST API + WebSocket streaming
- ✅ Beautiful TUI dashboard with time-based charts
- ✅ 55 tests passing (32 unit + 43 integration + 9 property + 3 doc)

**Next Steps (Phase 5):**
1. **Performance Optimization:**
   - Profile CPU and memory usage under load
   - Optimize database queries and add connection pooling
   - Benchmark metric throughput (target: 10k metrics/sec)
   - Load testing with multiple agents and services

2. **Production Hardening:**
   - Add structured logging with tracing
   - Implement meta-monitoring (monitor the monitoring system)
   - Enhanced health checks for all components
   - Graceful degradation strategies

3. **Documentation:**
   - Complete API documentation (OpenAPI/Swagger)
   - Write deployment guides (systemd, Docker, Kubernetes)
   - Create troubleshooting guide
   - Add architecture diagrams

4. **Distribution:**
   - Create release binaries (Linux, macOS, Windows)
   - Docker images with multi-stage builds
   - Installation scripts and package managers
   - GitHub Actions for automated releases

5. **Quality Assurance:**
   - Performance regression tests
   - Chaos testing (network failures, high load)
   - End-to-end tests
   - Security audit

**Contributing:**
When working on this project, focus on:
- Maintaining backward compatibility with existing configs
- Adding tests for new features
- Following the actor pattern for new components
- Documenting API changes
- Updating ROADMAP.md with progress notes

See [ROADMAP.md](ROADMAP.md) for detailed development plans and [README.md](README.md) for user-facing documentation.
