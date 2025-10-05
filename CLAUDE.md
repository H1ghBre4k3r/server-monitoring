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
- ✅ COMPLETE - Moving to Phase 4 (retention cleanup)

**Phase 3 Status (✅ Service Monitoring COMPLETE):**
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
- ✅ All tests passing (75/75: 29 unit + 34 integration + 9 property + 3 doc)

**Phase 4 Status (✅ Retention & Cleanup COMPLETE):**
- ✅ Startup cleanup (runs once on hub start)
- ✅ Configurable cleanup interval (default: 24 hours, range: 1-720 hours)
- ✅ Cleanup statistics tracking (`last_cleanup_time`, `total_metrics_deleted`, `total_service_checks_deleted`)
- ✅ Configuration validation (retention_days: 1-3650, cleanup_interval_hours: 1-720)
- ✅ Exposed in `StorageStats` via `GetStats` command
- ✅ All tests passing (75/75)
- 📋 NEXT: Phase 4.1 - API endpoints and dashboard

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
- **Priority**: Medium - after Phase 4.1 (retention cleanup and basic API)
- **Reason**: Current implementation works without bugs or performance issues
- **Timing**: 3-5 days after retention/API features are complete
- **Before**: Adding more alert types (would compound the technical debt)

See [ROADMAP.md Phase 3.5](ROADMAP.md#phase-35-alert-architecture-refactoring-) for full plan.
