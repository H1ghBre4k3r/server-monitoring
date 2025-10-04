# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based server monitoring solution that uses a hub-agent architecture to monitor server resources (CPU usage and temperature) and send alerts when thresholds are exceeded.

**Architecture:**
- **Agent (`src/bin/agent.rs`)**: Runs on monitored servers, exposes metrics via Rocket HTTP server on `/metrics` endpoint
- **Hub (`src/bin/hub.rs`)**: Central monitoring service using actor-based architecture (Phase 1 refactoring in progress)

**Key Components:**
- `ServerMetrics`: Core data structure containing system info, memory, CPU, and component temperature data
- `AlertManager`: Handles sending alerts via Discord webhooks or generic webhooks
- Configuration is JSON-based (see `config.example.json`)

### Actor-Based Architecture (✅ Phase 1 COMPLETE)

The hub now uses an actor-based architecture for better scalability and maintainability:

**Actors (`src/actors/`):**
- **MetricCollectorActor** (`collector.rs`): Polls agent endpoints at configured intervals, publishes metrics to broadcast channel
- **AlertActor** (`alert.rs`): Subscribes to metrics, maintains grace period state, triggers alerts when thresholds exceeded
- **StorageActor** (`storage.rs`): Subscribes to metrics, persists to storage (currently in-memory stub, Phase 2 will add SQLite/Postgres)

**Communication:**
- Each actor has an `mpsc` command channel for control messages (poll now, shutdown, etc.)
- Metrics flow through a `broadcast` channel from collectors to alert/storage actors
- Handles provide typed API for sending commands to actors

**Message Types (`messages.rs`):**
- `MetricEvent`: Published when metrics collected from a server
- `CollectorCommand`, `AlertCommand`, `StorageCommand`: Control messages for each actor type

**Why Actor Model:**
- Loose coupling - actors only communicate via messages
- Testable - actors can be tested in isolation with mock channels
- Scalable - easy to add new metric consumers (API, dashboard, etc.)
- Supervision - actors can be monitored and restarted independently
- Efficiency - HTTP client reused across requests (old system created new client each poll)

**Phase 1 Status (COMPLETE):**
- ✅ Core actor infrastructure implemented
- ✅ All actor types created with command/event channels
- ✅ Unit tests passing (5/5)
- ✅ Actors integrated into hub.rs
- ✅ Graceful shutdown on Ctrl+C
- ✅ Feature parity verified with old implementation
- ⏳ NEXT: Phase 2 - Metric Persistence (SQLite/PostgreSQL backends)

**Legacy Code:**
- Old `monitors/server.rs` and `monitors/resources.rs` are kept for reference
- The `ResourceEvaluation` logic in `monitors/resources.rs` is still used by AlertActor
- Will be cleaned up after Phase 2

**Testing:**
- Run actor tests: `cargo test --lib`
- All actors have basic unit tests in their respective modules
- Integration tests planned for Phase 2

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

## Alert Flow (Actor-Based)

1. **CollectorActor** polls agent HTTP endpoint at configured intervals
2. Metrics published to broadcast channel as `MetricEvent`
3. **AlertActor** receives events and evaluates against thresholds using grace period state machine
4. On `StartsToExceed` (grace period exhausted) or `BackToOk` (recovered), `AlertManager` sends alerts
5. Alerts formatted and sent via Discord (with embeds) or webhook (JSON payload)
6. **StorageActor** also receives events (currently logs only, persistence in Phase 2)

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
