# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based server monitoring solution that uses a hub-agent architecture to monitor server resources (CPU usage and temperature) and send alerts when thresholds are exceeded.

**Architecture:**
- **Agent (`src/bin/agent.rs`)**: Runs on monitored servers, exposes metrics via Rocket HTTP server on `/metrics` endpoint
- **Hub (`src/bin/hub.rs`)**: Central monitoring service that polls multiple agents, evaluates metrics against configured limits, and sends alerts

**Key Components:**
- `ServerMetrics`: Core data structure containing system info, memory, CPU, and component temperature data
- `AlertManager`: Handles sending alerts via Discord webhooks or generic webhooks
- `ResourceMonitor`: Evaluates metrics against limits with grace period support
- Configuration is JSON-based (see `config.example.json`)

## Development Commands

### Building
```bash
# Development build
cargo build
# or
just build

# Release build
cargo build --release
# or
just build-release

# Build only binaries (agent + hub)
cargo build --bins
# or
just bins
```

### Testing
```bash
cargo test --workspace
# or
just test
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

## Configuration

The hub requires a JSON config file specifying servers to monitor. Key concepts:

- **Grace periods**: Number of consecutive exceeded measurements before alerting
- **Limits**: Separate thresholds for `temperature` (°C) and `usage` (CPU %)
- **Alerts**: Support for Discord webhooks (with optional user mentions) and generic webhooks
- **Tokens**: Optional `X-MONITORING-SECRET` header for agent authentication

Agent configuration via environment variables:
- `AGENT_ADDR`: Bind address (default: 0.0.0.0)
- `AGENT_PORT`: HTTP port (default: 3000)
- `AGENT_SECRET`: Optional authentication token

## Alert Flow

1. Hub polls agent at configured intervals
2. `ResourceMonitor` evaluates metrics against limits with grace period tracking
3. On `StartsToExceed` (grace period reached) or `BackToOk` (recovered), `AlertManager` triggers
4. Alerts formatted and sent via Discord (with embeds) or webhook (JSON payload)

## Binary Structure

The project produces two binaries from `src/bin/`:
- `agent`: The monitoring agent (also called `guardia-agent` when installed)
- `hub`: The central monitoring hub

Both share common code from `src/lib.rs` and its modules.

## Resource Evaluation States

- `Ok`: Below limit
- `Exceeding`: Above limit but within grace period
- `StartsToExceed`: Grace period exhausted, triggers alert
- `BackToOk`: Returned to normal after exceeding, triggers recovery alert
