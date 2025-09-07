# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based server monitoring system that consists of two main components:
- **Hub** (`src/bin/hub.rs`): Monitors multiple servers by polling metrics endpoints
- **Agent** (`src/bin/agent.rs`): Runs on monitored servers, exposing a `/metrics` endpoint via Rocket web framework

## Architecture

### Core Components

- **Hub Binary**: Reads config file, spawns monitor tasks for each server, polls metrics endpoints at configured intervals
- **Agent Binary**: Rocket web server that collects system metrics (CPU, memory, temperature) using `sysinfo` crate and exposes them via HTTP API
- **Resource Monitoring**: Async channel-based system with grace periods for alert threshold evaluation
- **Alert System**: Supports Discord webhooks with rich embeds and generic webhook notifications

### Key Modules

- `config.rs`: JSON configuration parsing with serde, defines server limits and alert configurations
- `monitors/server.rs`: HTTP client that polls agent endpoints, handles connection failures gracefully
- `monitors/resources.rs`: Grace period logic and threshold evaluation using async channels
- `alerts.rs`: Alert manager with Discord embed formatting and webhook notifications
- `util.rs`: Environment variable helpers for agent configuration (port, address, secret)

### Data Flow

1. Hub reads JSON config file specifying servers to monitor
2. For each server, Hub spawns a `server_monitor` task that polls `/metrics` endpoint
3. Metrics are sent through async channel to `resource_monitor` 
4. Resource monitor evaluates against limits with grace periods
5. When thresholds exceeded, AlertManager sends Discord/webhook notifications
6. Agent serves metrics from `/metrics` endpoint with optional token authentication

## Commands

### Build
```bash
cargo build --all
cargo build --all --release
```

### Testing
```bash
cargo test --workspace
```

### Linting and Formatting
```bash
cargo clippy --all-targets --workspace
cargo fmt -- --check
```

### Running

**Agent** (on monitored server):
```bash
cargo run --bin agent
# Environment variables:
# AGENT_PORT=51243 (default)
# AGENT_ADDR=0.0.0.0 (default)
# AGENT_SECRET=<optional auth token>
```

**Hub** (monitoring coordinator):
```bash
cargo run --bin hub -- -f config.json
```

## Configuration

The system uses a JSON config file with this structure:
- `servers[]`: Array of servers to monitor
  - `ip`, `port`: Agent endpoint location
  - `display`: Friendly name for alerts
  - `interval`: Polling frequency in seconds (default: 15)
  - `token`: Optional authentication for agent
  - `limits`: Thresholds for temperature/usage monitoring
    - `temperature`/`usage`: Each has `limit`, `grace` period, and optional `alert` config
    - `alert`: Can be Discord webhook or generic webhook

## Development Notes

- Uses `tracing` for structured logging with pretty formatting
- Async/await throughout with Tokio runtime
- Error handling with `anyhow` crate
- Authentication via `X-MONITORING-SECRET` header between hub and agents
- Grace periods prevent alert spam - alerts only fire after threshold exceeded for N consecutive checks
- Alert recovery notifications sent when values return to normal after exceeding grace period
- Discord alerts use rich embeds with color coding and progress bars