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
- **StorageActor** (`storage.rs`): Subscribes to metrics, persists to storage with pluggable backends (SQLite, in-memory)

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

**Phase 1 Status (✅ COMPLETE):**
- ✅ Core actor infrastructure implemented
- ✅ All actor types created with command/event channels
- ✅ Unit tests passing (5/5)
- ✅ Actors integrated into hub.rs
- ✅ Graceful shutdown on Ctrl+C
- ✅ Feature parity verified with old implementation

**Phase 2 Status (✅ SQLite Backend COMPLETE):**
- ✅ StorageBackend trait for pluggable persistence
- ✅ SQLite backend with batching (dual flush triggers: 100 metrics OR 5 seconds)
- ✅ Hybrid schema: indexed aggregates + full metadata
- ✅ StorageActor extended with `Option<Box<dyn StorageBackend>>`
- ✅ Configuration support for storage backends
- ✅ Backward compatible (falls back to in-memory mode)
- ✅ All tests passing (60/60)
- ⏳ NEXT: Retention/cleanup background task, integration tests

**Legacy Code:**
- Old `monitors/server.rs` and `monitors/resources.rs` are kept for reference
- The `ResourceEvaluation` logic in `monitors/resources.rs` is still used by AlertActor
- Will be cleaned up after Phase 2

**Testing:**
- Run actor tests: `cargo test --lib`
- Run all tests: `cargo test --workspace --all-features`
- All actors have unit tests in their respective modules
- Integration tests for actor communication in `tests/integration/`

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
    "retention_days": 30
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

**Backward Compatibility:**
- `storage` config section is optional - defaults to in-memory if omitted
- Feature flag `storage-sqlite` (enabled by default) - can be disabled at compile time
- StorageActor gracefully falls back to in-memory mode if backend init fails

**Feature Flags:**
- `storage-sqlite`: Enables SQLite backend (requires sqlx dependency)
- Build without persistence: `cargo build --no-default-features`

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

- **Storage** (optional): Backend for metric persistence (`sqlite` or `none`/omitted for in-memory)
- **Grace periods**: Number of consecutive exceeded measurements before alerting
- **Limits**: Separate thresholds for `temperature` (°C) and `usage` (CPU %)
- **Alerts**: Support for Discord webhooks (with optional user mentions) and generic webhooks
- **Tokens**: Optional `X-MONITORING-SECRET` header for agent authentication

See `config.example.json` for a complete configuration example.

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

The project produces two binaries from `src/bin/`:
- `agent`: The monitoring agent (also called `guardia-agent` when installed)
- `hub`: The central monitoring hub

Both share common code from `src/lib.rs` and its modules.

## Resource Evaluation States

- `Ok`: Below limit
- `Exceeding`: Above limit but within grace period
- `StartsToExceed`: Grace period exhausted, triggers alert
- `BackToOk`: Returned to normal after exceeding, triggers recovery alert
