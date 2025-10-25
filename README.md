# Guardia

> A comprehensive Rust-based server monitoring solution with distributed agent architecture, real-time metrics collection, service health checks, and beautiful dashboards.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL%203.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)]()


## ✨ Features

- **📊 Real-time Monitoring**: CPU usage, temperature, memory, and component-level metrics
- **🌐 Service Health Checks**: HTTP/HTTPS endpoint monitoring with uptime tracking
- **🔔 Smart Alerting**: Discord and webhook alerts with grace periods to prevent flapping
- **💾 Time-Series Storage**: SQLite backend with configurable retention and automatic cleanup
- **🎯 Actor-Based Architecture**: Scalable, maintainable, and testable design using Tokio actors
- **🔌 REST + WebSocket API**: Remote access with real-time streaming capabilities
- **🌐 Web Dashboard**: Modern web interface with ECharts visualizations (React + TypeScript)
- **📺 TUI Dashboard**: Beautiful terminal UI with time-based charts, memory gauges, and sliding windows
- **📈 Advanced Visualization**: Time-based charts with HH:MM:SS labels, color-coded memory gauges, historical data loading
- **🔐 Security**: Token-based authentication for agents and API access
- **⚙️ Configurable**: JSON-based configuration with global alerts and defaults to reduce duplication

## 🏗️ Architecture

```
┌─────────────┐      HTTP polls     ┌───────────────┐
│   Agent 1   │◄────────────────────┤   Hub (Main)  │
│  (Server A) │                     │   ┌─────────┐ │
└─────────────┘                     │   │Collector│ │
                                    │   │ Actors  │ │
┌─────────────┐      HTTP polls     │   └────┬────┘ │
│   Agent 2   │◄────────────────────┤        │      │
│  (Server B) │                     │   ┌────▼────┐ │
└─────────────┘                     │   │ Storage │ │
                                    │   │  Actor  │ │
┌─────────────┐     HTTP/HTTPS      │   └────┬────┘ │
│  Service 1  │◄────────────────────┤        │      │
│ (API Check) │                     │   ┌────▼────┐ │
└─────────────┘                     │   │  Alert  │ │
                                    │   │  Actor  │ │
                                    │   └─────────┘ │
                                    │               │
                                    │   ┌─────────┐ │
                                    │   │   API   │ │
                                    │   │ Server  │ │
                                    │   └────┬────┘ │
                                    └────────┼──────┘
                                             │
                                      REST + WebSocket
                                             │
                                    ┌────────▼──────┐
                                    │  Web Dashboard │
                                    │  (Browser)     │
                                    └────────┬──────┘
                                             │
                                    ┌────────▼──────┐
                                    │  TUI Viewer   │
                                    │  (Dashboard)  │
                                    └───────────────┘
```

**Components:**
- **Agent** (`guardia-agent`): Runs on each monitored server, exposes metrics via HTTP
- **Hub** (`guardia-hub`): Central monitoring service with actor-based architecture + API server + Web Dashboard
- **Viewer** (`guardia-viewer`): TUI dashboard for real-time visualization (Ratatui)

## 🚀 Quick Start

### 1. Install

```bash
# Build from source
cargo build --release

# Install binaries
cargo install --path .

# Or download pre-built binaries (future release)
```

### 2. Start an Agent

On each server you want to monitor:

```bash
# Set environment variables
export AGENT_ADDR=0.0.0.0
export AGENT_PORT=3000
export AGENT_SECRET=your-secret-token

# Run the agent
guardia-agent
```

### 3. Configure the Hub

Create `config.json` using the new format with global alerts and defaults:

```json
{
  "_comment": "Guardia Configuration with Global Alerts",
  
  "alerts": {
    "prod-critical": {
      "discord": {
        "url": "https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN",
        "user_id": "123456789012345678"
      }
    },
    "dev-team": {
      "webhook": {
        "url": "https://monitoring.example.com/webhook"
      }
    }
  },

  "defaults": {
    "server": {
      "interval": 30,
      "limits": {
        "temperature": {
          "limit": 75,
          "grace": 3,
          "alert": "prod-critical"
        },
        "usage": {
          "limit": 80,
          "grace": 5,
          "alert": "prod-critical"
        }
      }
    },
    "service": {
      "interval": 60,
      "timeout": 10,
      "grace": 3,
      "alert": "prod-critical"
    }
  },

  "storage": {
    "backend": "sqlite",
    "path": "./metrics.db",
    "retention_days": 30,
    "cleanup_interval_hours": 24
  },

  "api": {
    "bind": "127.0.0.1",
    "port": 8080,
    "auth_token": "api-secret-token",
    "enable_cors": true
  },

  "servers": [
    {
      "ip": "192.168.1.100",
      "display": "Production Server",
      "port": 3000,
      "token": "your-secret-token"
      // Inherits all defaults
    },
    {
      "ip": "192.168.1.101",
      "display": "Development Server",
      "port": 3000,
      "limits": {
        "temperature": {
          "limit": 85,
          "alert": "dev-team"
        }
      }
    }
  ],

  "services": [
    {
      "name": "API Health",
      "url": "https://api.example.com/health"
      // Uses service defaults
    },
    {
      "name": "Website",
      "url": "https://example.com",
      "interval": 120,
      "alert": "dev-team"
    }
  ]
}
```

### 4. Start the Hub

```bash
guardia-hub -f config.json
```

### 5. Access the Dashboards

**Option A: Web Dashboard (Recommended)**

Access the modern web dashboard in your browser:

```
http://localhost:8080
```

Features:
- Modern, responsive UI built with React and TypeScript
- Beautiful ECharts visualizations with per-core CPU charts
- Real-time updates via WebSocket
- Works on desktop and tablet
- Dark theme by default
- Interactive charts with legends and tooltips

**Option B: TUI Dashboard (Terminal)**

```bash
# Create viewer config
mkdir -p ~/.config/guardia
cat > ~/.config/guardia/viewer.toml <<EOF
api_url = "http://localhost:8080"
api_token = "api-secret-token"
refresh_interval = 5
max_metrics = 100
time_window_seconds = 300  # 5 minute sliding window for charts
EOF

# Run the viewer
guardia-viewer

# Or with CLI args
guardia-viewer --url http://localhost:8080 --token api-secret-token
```

**TUI Dashboard Features:**
- **Time-based Charts**: CPU and temperature charts with actual timestamps (HH:MM:SS format)
  - Configurable sliding window (default: 5 minutes)
  - Automatic historical data loading on startup
  - Real-time updates via WebSocket
- **Enhanced Memory Visualization**: Color-coded gauges for RAM and Swap
  - Green (<70%), Yellow (<85%), Red (≥85%)
  - Progress bars with absolute values (GB) and percentages
- **Server Details**: Hostname, OS, architecture, quick metrics summary
- **Three-Tab Interface**: Servers, Services, Alerts
- **Health Status Indicators**: Color-coded status for all monitored resources

**Keybindings:**
- `Tab` / `←` `→` - Navigate between tabs (Servers, Services, Alerts)
- `↑` `↓` / `j` `k` - Select items in lists
- `Space` - Pause/resume real-time updates
- `r` - Refresh data from API
- `q` / `Esc` - Quit

### 6. Access the API (Optional)

```bash
# Health check
curl http://localhost:8080/api/v1/health

# List servers with health status
curl -H "Authorization: Bearer api-secret-token" \
  http://localhost:8080/api/v1/servers

# Get latest metrics
curl -H "Authorization: Bearer api-secret-token" \
  "http://localhost:8080/api/v1/servers/192.168.1.100:3000/metrics/latest?limit=10"
```

## 🐳 Docker Deployment

### Quick Start with Docker Compose

1. **Create configuration:**
```bash
mkdir -p config
cp config.example.json config/config.json
# Edit config/config.json with your settings
# Important: Set storage.path to "/app/data/metrics.db"
```

2. **Start the hub:**
```bash
docker compose up -d
```

3. **View logs:**
```bash
docker compose logs -f hub
```

### Configuration for Docker

When using Docker, ensure your `config.json` uses container-appropriate paths:

```json
{
  "storage": {
    "backend": "sqlite",
    "path": "/app/data/metrics.db",
    "retention_days": 30
  }
}
```

## 📖 Configuration

### New Configuration Format (v0.5.0+)

The configuration now supports **global alerts** and **defaults** to reduce duplication:

- **`alerts`**: Define reusable named alert configurations (Discord, webhook)
- **`defaults`**: Set default values for servers and services (intervals, limits, alerts)
- **Alert References**: Servers and services reference alerts by name instead of inline definitions
- **Inheritance**: Servers/services inherit defaults unless overridden

### Storage Options

**SQLite (default):**
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

**In-Memory (no persistence):**
```json
{
  "storage": {
    "backend": "none"
  }
}
```

### Alert Configuration

**Discord with user mentions:**
```json
{
  "alerts": {
    "prod-critical": {
      "discord": {
        "url": "https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN",
        "user_id": "123456789012345678"
      }
    }
  }
}
```

**Generic webhook:**
```json
{
  "alerts": {
    "webhook-monitoring": {
      "webhook": {
        "url": "https://monitoring.example.com/webhook"
      }
    }
  }
}
```

### Grace Periods

Grace periods prevent alert spam from temporary spikes:

```json
{
  "limits": {
    "temperature": {
      "limit": 75,
      "grace": 3  // Alert after 3 consecutive violations
    }
  }
}
```

See [config.example.json](config.example.json) for a complete configuration example.

## 🔌 API Reference

### REST Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/health` | GET | Health check with timestamp |
| `/api/v1/stats` | GET | System statistics (storage, actors) |
| `/api/v1/servers` | GET | List all servers with health status |
| `/api/v1/servers/:id/metrics` | GET | Query metrics (supports `?start=&end=&limit=`) |
| `/api/v1/servers/:id/metrics/latest` | GET | Get latest N metrics (`?limit=100`) |
| `/api/v1/services` | GET | List all services with health status |
| `/api/v1/services/:name/checks` | GET | Service check history (`?start=&end=`) |
| `/api/v1/services/:name/uptime` | GET | Uptime statistics (`?since=`) |

### WebSocket Streaming

Connect to `/api/v1/stream` for real-time events:

```javascript
const ws = new WebSocket('ws://localhost:8080/api/v1/stream');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);

  if (data.type === 'metric') {
    console.log('Metric:', data.server_id, data.metrics);
  } else if (data.type === 'service_check') {
    console.log('Service Check:', data.service_name, data.status);
  }
};
```

### Authentication

Use Bearer token authentication:

```bash
curl -H "Authorization: Bearer your-api-token" \
  http://localhost:8080/api/v1/stats
```

## 🛠️ Development

### Quick Reference with Justfile

This project includes a `justfile` for common development tasks:

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

# Build specific binary
cargo build --bin hub
cargo build --bin agent
```

### Testing

```bash
# Run all tests (84 tests: 29 unit + 43 integration + 9 property + 3 doc)
cargo test --workspace --all-features
# or
just test

# Run specific test suite
cargo test --lib                    # Unit tests
cargo test --test '*'               # Integration tests
cargo test --doc                    # Doc tests
```

### Building the Web Dashboard

The web dashboard is built automatically when building the hub with the `web-dashboard` feature enabled (default).

```bash
# Build web dashboard
cd web-dashboard
npm install
npm run build

# Dashboard will be available at http://localhost:8080 when hub runs
```

For development:

```bash
# Development server with hot reload
cd web-dashboard
npm install
npm run dev

# In another terminal, run hub API server
cargo run --bin hub -- -f config.json
```

The dev server proxies API requests to `http://localhost:8080`.

### Feature Flags

```bash
# Build with all features (default - includes web-dashboard)
cargo build --all-features

# Build without web dashboard
cargo build --no-default-features --features "storage-sqlite,api,dashboard"

# Build minimal hub (no storage, no API, no dashboards)
cargo build --bin hub --no-default-features
```

Available features:
- `storage-sqlite` (default): SQLite backend for persistence
- `api` (default): REST API and WebSocket server
- `dashboard` (default): TUI viewer dependencies
- `web-dashboard` (default): Web dashboard served by hub

### Development Commands

```bash
# Watch mode (auto-rebuild on changes)
cargo watch -x "build --bins"

# Run with config
cargo run --bin hub -- -f config.json

# Run agent with environment variables
AGENT_PORT=3000 AGENT_SECRET=test cargo run --bin agent
```

## 📊 Monitoring Metrics

### Server Metrics

- **CPU**: Per-core usage, average usage, architecture
- **Memory**: Total, used, swap usage
- **Temperature**: Per-component temperatures, average
- **System**: Kernel version, OS version, hostname

### Service Metrics

- **Status**: UP, DOWN, DEGRADED
- **Response Time**: Milliseconds
- **HTTP Status Code**: For HTTP/HTTPS checks
- **Uptime Percentage**: Calculated over time ranges

## 🤝 Contributing

Contributions are welcome! Please see [ROADMAP.md](ROADMAP.md) for planned features.

### Development Setup

1. Clone the repository
2. Install Rust (1.70+)
3. Run tests: `cargo test --workspace --all-features`
4. Build: `cargo build`

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Run clippy: `cargo clippy -- -D warnings`
- Add tests for new features
- Update documentation

## 📝 License

This project is licensed under the GPL-3.0 License - see the [LICENSE](LICENSE) file for details.

## 🗺️ Roadmap

**Current Version: v0.5.0**

**Completed:**
- ✅ Phase 1: Actor-based architecture with graceful shutdown
- ✅ Phase 2: SQLite persistence with batching and hybrid schema
- ✅ Phase 3: Service health monitoring (HTTP/HTTPS with uptime tracking)
- ✅ Phase 4.0: Automatic retention cleanup with configurable policies
- ✅ Phase 4.1: REST API + WebSocket streaming
- ✅ Phase 4.2: TUI Dashboard with time-based charts and historical data loading
- ✅ Phase 5: Web Dashboard with modern React UI and ECharts visualizations

**Current Focus:**
- 🎯 Production hardening and performance optimization (target: v1.0.0)

**Future Plans:**
- 📋 Phase 3.5: Alert architecture refactoring (split metric/service alerts)
- 📋 v1.1.0+: Mobile app, additional alert channels, anomaly detection

See [ROADMAP.md](ROADMAP.md) for detailed plans.

## 📚 Documentation

- [CLAUDE.md](CLAUDE.md) - Detailed technical documentation for AI assistants
- [ROADMAP.md](ROADMAP.md) - Development roadmap and feature plans
- [TESTING.md](TESTING.md) - Comprehensive testing documentation
- [config.example.json](config.example.json) - Complete configuration example with new format
- API Documentation - Coming soon (OpenAPI/Swagger)

## 🙏 Acknowledgments

Built with:
- [Tokio](https://tokio.rs/) - Async runtime
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - SQL toolkit
- [Ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [React](https://reactjs.org/) - Web UI framework
- [Apache ECharts](https://echarts.apache.org/) - Data visualization
- [Sysinfo](https://github.com/GuillaumeGomez/sysinfo) - System information

---

**Made with ❤️ in Rust**
