# Architecture Refactoring Plan

## Executive Summary

This document outlines the plan to refactor the current hub architecture from a thread-based polling system to a clean actor-based architecture using Tokio channels. The refactoring will improve maintainability, testability, and prepare the codebase for advanced features like metric persistence, service monitoring, and API access.

---

## Current Architecture Analysis

### Code Structure (as of v0.1.0)

```
src/
├── bin/
│   ├── agent.rs      - Rocket HTTP server exposing /metrics
│   └── hub.rs        - Main monitoring coordinator
├── monitors/
│   ├── server.rs     - Per-server polling loop
│   └── resources.rs  - Resource evaluation with grace periods
├── alerts.rs         - Alert sending logic
├── discord.rs        - Discord webhook formatting
├── config.rs         - Configuration structs
├── lib.rs           - Shared types (ServerMetrics, etc.)
└── util.rs          - Environment variable helpers
```

### Current Flow

1. **Hub startup** (`hub.rs:main`)
   - Reads JSON config file
   - Calls `dispatch_servers()`
   - Spawns one `tokio::task` per server via `spawn(server_monitor(config))`

2. **Server Monitor** (`monitors/server.rs:server_monitor`)
   - Runs infinite loop with `tokio::time::sleep(interval)`
   - Creates `AlertManager` and `ResourceMonitor`
   - Creates unbounded channel for metrics
   - Polls agent HTTP endpoint
   - Sends metrics to channel

3. **Resource Monitor** (`monitors/resources.rs`)
   - Receives metrics from channel in its own spawned task
   - Evaluates CPU usage and temperature against limits
   - Tracks grace period state
   - Calls handler closures on threshold violations
   - Handlers spawn new tasks to send alerts

4. **Alert Manager** (`alerts.rs`)
   - Formats alert messages
   - Sends HTTP requests to Discord/webhooks

### Problems with Current Architecture

#### 1. **Tight Coupling**
- `server_monitor` creates its own `AlertManager` and `ResourceMonitor`
- No way to inject alternative implementations for testing
- Difficult to add new metric consumers (e.g., storage, API)

#### 2. **Unstructured Concurrency**
- Each server spawns 2+ tasks (server loop + resource monitor + alert senders)
- No centralized task supervision or error handling
- Can't easily coordinate cross-server operations

#### 3. **No Shared State Management**
- Each monitor maintains its own grace period state
- No global view of system health
- Cannot implement features like "alert if >50% of servers down"

#### 4. **Limited Extensibility**
- Adding new metric sources requires duplicating the monitor pattern
- No clean way to broadcast metrics to multiple consumers
- Hard to add features like metric persistence or API access

#### 5. **Resource Inefficiency**
- Creates new `reqwest::Client` on every request (line 54 in server.rs)
- Unbounded channels risk memory exhaustion under load
- No connection pooling or batching

---

## Target Architecture: Actor Model

### Why Actor Model?

The actor model addresses all current issues:
- **Loose Coupling:** Actors communicate only via messages
- **Supervision:** Parent actors can monitor and restart failed children
- **Testability:** Actors can be tested in isolation with mock channels
- **Scalability:** Easy to add new actors or distribute across machines
- **Backpressure:** Bounded channels provide natural flow control

### Core Concepts

An **actor** is:
- An async task running in a loop
- Receives messages via a Tokio channel
- Maintains private state
- Communicates with other actors via messages
- Can spawn child actors

### Reference Implementation

Alice Ryhl's pattern (https://ryhl.io/blog/actors-with-tokio/):

```rust
pub struct MyActor {
    receiver: mpsc::Receiver<ActorMessage>,
    // private state
}

pub enum ActorMessage {
    DoSomething { respond_to: oneshot::Sender<Response> },
    // other message types
}

impl MyActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        // initialize
    }

    async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }

    async fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::DoSomething { respond_to } => {
                let result = self.do_something().await;
                let _ = respond_to.send(result);
            }
        }
    }
}

pub struct MyActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

impl MyActorHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let actor = MyActor::new(receiver);
        tokio::spawn(actor.run());
        Self { sender }
    }

    pub async fn do_something(&self) -> Result<Response> {
        let (send, recv) = oneshot::channel();
        self.sender.send(ActorMessage::DoSomething { respond_to: send }).await?;
        recv.await?
    }
}
```

---

## Proposed Actor System

### Actor Types

#### 1. **SupervisorActor**
- **Role:** Root actor, spawns and supervises all other actors
- **State:** Map of actor handles, health status
- **Messages:**
  - `GetSystemStatus` → Returns overall health
  - `RestartActor(ActorId)` → Restarts failed actor
  - `Shutdown` → Graceful shutdown of all actors

#### 2. **MetricCollectorActor** (one per server)
- **Role:** Polls agent endpoint at configured intervals
- **State:** Server config, HTTP client, last poll time
- **Messages:**
  - `PollNow` → Trigger immediate poll (for testing)
  - `UpdateConfig(ServerConfig)` → Change poll interval/URL
  - Publishes: `MetricCollected` event
- **Behavior:** Infinite loop with interval timer, publishes metrics to broadcast channel

#### 3. **StorageActor**
- **Role:** Persists metrics to database/files
- **State:** Database connection pool, write buffer
- **Messages:**
  - `StoreMetrics(Vec<ServerMetrics>)` → Batch write
  - `QueryMetrics(TimeRange, ServerId)` → Read historical data
  - `GetStorageStats` → Returns DB size, write rate
- **Behavior:** Subscribes to metric broadcast channel, batches writes

#### 4. **AlertActor**
- **Role:** Evaluates metrics against thresholds, sends alerts
- **State:** Per-server grace period counters, HTTP client for webhooks
- **Messages:**
  - `EvaluateMetric(ServerMetrics)` → Check thresholds
  - `GetAlertHistory` → Returns recent alerts
  - `MuteAlerts(Duration)` → Temporarily disable
- **Behavior:** Subscribes to metric broadcast, maintains grace state

#### 5. **ServiceMonitorActor**
- **Role:** Monitors HTTP endpoints and ICMP ping
- **State:** Service configs, HTTP client, ping state
- **Messages:**
  - `CheckService(ServiceId)` → Immediate check
  - `AddService(ServiceConfig)` → Dynamic registration
  - Publishes: `ServiceCheckResult` event
- **Behavior:** Manages check intervals per service

#### 6. **ApiActor**
- **Role:** Handles external API requests and WebSocket connections
- **State:** Connected clients, subscription filters
- **Messages:**
  - `HandleHttpRequest(Request)` → Process API call
  - `BroadcastToWebSockets(Event)` → Send to all WS clients
  - `ClientConnected/Disconnected` → Manage subscriptions
- **Behavior:** Subscribes to all events, filters and forwards to clients

### Channel Architecture

```
                    ┌─────────────────┐
                    │ SupervisorActor │
                    └────────┬────────┘
                             │ spawns
                ┌────────────┼────────────┐
                │            │            │
        ┌───────▼───────┐    │    ┌───────▼───────┐
        │ Collector-1   │    │    │ Collector-N   │
        │ (Server A)    │    │    │ (Server N)    │
        └───────┬───────┘    │    └───────┬───────┘
                │            │            │
                └────────────┼────────────┘
                             │
                   ┌─────────▼──────────┐
                   │  Broadcast Channel │ (metrics)
                   │  (MPMC)            │
                   └──────────┬─────────┘
                              │ subscribe
              ┌───────────────┼───────────────┐
              │               │               │
      ┌───────▼───────┐  ┌────▼────┐  ┌──────▼──────┐
      │ StorageActor  │  │AlertActor│  │  ApiActor   │
      └───────────────┘  └──────────┘  └──────┬──────┘
                                               │
                                      ┌────────▼────────┐
                                      │ WebSocket Clients│
                                      └─────────────────┘
```

**Channel Types:**

1. **Command Channels** (mpsc, bounded)
   - Each actor has its own command channel
   - Size: 32 messages (small buffer for backpressure)
   - Used for: Configuration changes, queries, control commands

2. **Event Broadcast** (broadcast, capacity 256)
   - Published by: All collector actors
   - Subscribed by: Storage, Alert, API actors
   - Contains: `MetricCollected(ServerId, ServerMetrics, Timestamp)`
   - Lagging subscribers are dropped (they can resync from storage)

3. **Oneshot Channels**
   - Used for request/response patterns
   - Example: `QueryMetrics` returns data via oneshot

---

## Migration Strategy

### Phase 1: Introduce Actor Infrastructure (Week 1)

**Goal:** Add actor primitives without breaking existing code

**Steps:**
1. Create `src/actors/mod.rs` module
2. Implement `SupervisorActor` with basic lifecycle
3. Add broadcast channel for metrics (initially unused)
4. Keep existing `server_monitor` running in parallel
5. Test: Verify supervisor can start/stop without affecting current flow

**Deliverables:**
- `src/actors/supervisor.rs`
- `src/actors/messages.rs` - Common message types
- Integration test proving coexistence

### Phase 2: Extract MetricCollectorActor (Week 1-2)

**Goal:** Replace `server_monitor` loop with actor

**Steps:**
1. Create `src/actors/collector.rs`
2. Implement actor with same polling logic
3. Publish metrics to broadcast channel
4. Update `hub.rs` to spawn collectors via supervisor
5. Remove old `server_monitor` code
6. Test: Verify metrics still collected, alerts still sent

**Deliverables:**
- `src/actors/collector.rs`
- Updated `src/bin/hub.rs`
- Delete old loop-based code

### Phase 3: Extract AlertActor (Week 2)

**Goal:** Separate alert logic into dedicated actor

**Steps:**
1. Create `src/actors/alert.rs`
2. Move `ResourceMonitor` logic into actor
3. Subscribe to metric broadcast channel
4. Maintain grace period state in actor
5. Send alerts via existing `AlertManager`
6. Test: Verify alert timing and grace periods preserved

**Deliverables:**
- `src/actors/alert.rs`
- Refactored `src/alerts.rs` (just formatting, no state)

### Phase 4: Add StorageActor Stub (Week 2)

**Goal:** Prepare for persistence without implementing backends

**Steps:**
1. Create `src/actors/storage.rs`
2. Define storage trait: `StorageBackend`
3. Implement in-memory backend (for testing)
4. Subscribe to metric broadcast
5. Expose query interface (returns empty for now)
6. Test: Verify metrics are received, no panics

**Deliverables:**
- `src/actors/storage.rs`
- `src/storage/mod.rs` - Backend trait
- `src/storage/memory.rs` - In-memory implementation

### Phase 5: Validation & Cleanup (Week 2)

**Goal:** Ensure refactoring didn't break anything

**Steps:**
1. End-to-end testing with real agents
2. Performance benchmarking vs v0.1.0
3. Fix any regressions
4. Update documentation
5. Remove dead code

**Acceptance Criteria:**
- ✅ All existing features work identically
- ✅ Performance within 5% of baseline
- ✅ No memory leaks under 24hr run
- ✅ Clean `cargo clippy` output
- ✅ Updated CLAUDE.md with new architecture

---

## Code Examples

### Simplified MetricCollectorActor

```rust
// src/actors/collector.rs

use tokio::sync::{mpsc, broadcast};
use tokio::time::{interval, Duration};
use reqwest::Client;
use crate::config::ServerConfig;

pub struct MetricCollectorActor {
    config: ServerConfig,
    client: Client,
    command_rx: mpsc::Receiver<CollectorCommand>,
    metric_tx: broadcast::Sender<MetricEvent>,
}

pub enum CollectorCommand {
    UpdateInterval(Duration),
    PollNow { respond_to: oneshot::Sender<Result<()>> },
    Shutdown,
}

pub struct MetricEvent {
    pub server_id: String,
    pub metrics: ServerMetrics,
    pub timestamp: DateTime<Utc>,
}

impl MetricCollectorActor {
    pub fn new(
        config: ServerConfig,
        command_rx: mpsc::Receiver<CollectorCommand>,
        metric_tx: broadcast::Sender<MetricEvent>,
    ) -> Self {
        Self {
            config,
            client: Client::new(),
            command_rx,
            metric_tx,
        }
    }

    pub async fn run(mut self) {
        let mut ticker = interval(Duration::from_secs(self.config.interval as u64));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.poll_metrics().await;
                }
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        CollectorCommand::PollNow { respond_to } => {
                            let result = self.poll_metrics().await;
                            let _ = respond_to.send(result);
                        }
                        CollectorCommand::UpdateInterval(dur) => {
                            ticker = interval(dur);
                        }
                        CollectorCommand::Shutdown => break,
                    }
                }
            }
        }
    }

    async fn poll_metrics(&self) -> Result<()> {
        let url = format!("http://{}:{}/metrics", self.config.ip, self.config.port);

        let response = self.client
            .get(&url)
            .header("X-MONITORING-SECRET", self.config.token.as_ref().unwrap_or(&String::new()))
            .send()
            .await?;

        let metrics: ServerMetrics = response.json().await?;

        let event = MetricEvent {
            server_id: format!("{}:{}", self.config.ip, self.config.port),
            metrics,
            timestamp: Utc::now(),
        };

        // Ignore send errors (no subscribers is OK)
        let _ = self.metric_tx.send(event);

        Ok(())
    }
}

// Handle for external control
pub struct CollectorHandle {
    sender: mpsc::Sender<CollectorCommand>,
}

impl CollectorHandle {
    pub fn new(config: ServerConfig, metric_tx: broadcast::Sender<MetricEvent>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let actor = MetricCollectorActor::new(config, cmd_rx, metric_tx);
        tokio::spawn(actor.run());
        Self { sender: cmd_tx }
    }

    pub async fn poll_now(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(CollectorCommand::PollNow { respond_to: tx }).await?;
        rx.await?
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.sender.send(CollectorCommand::Shutdown).await?;
        Ok(())
    }
}
```

### Updated Hub

```rust
// src/bin/hub.rs (simplified)

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let args = Args::parse();
    let config = read_config_file(&args.file)?;

    // Create broadcast channel for metrics
    let (metric_tx, _) = broadcast::channel(256);

    // Spawn storage actor
    let storage_handle = StorageActor::new(metric_tx.subscribe());

    // Spawn alert actor
    let alert_handle = AlertActor::new(metric_tx.subscribe());

    // Spawn collector for each server
    let mut collectors = vec![];
    if let Some(servers) = config.servers {
        for server_config in servers {
            let handle = CollectorHandle::new(server_config, metric_tx.clone());
            collectors.push(handle);
        }
    }

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    // Graceful shutdown
    for collector in collectors {
        collector.shutdown().await?;
    }
    storage_handle.shutdown().await?;
    alert_handle.shutdown().await?;

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests
- Test each actor in isolation with mock channels
- Verify state transitions (e.g., grace period logic)
- Test error handling (network failures, channel closed)

### Integration Tests
- Spawn full actor system with test config
- Inject fake agent responses
- Verify metrics flow through entire pipeline
- Test shutdown and restart scenarios

### Performance Tests
- Benchmark actor message throughput
- Measure latency from metric collection to alert
- Test with 100+ servers to ensure scalability
- Monitor memory usage over 24hr period

---

## Rollback Plan

If the refactoring introduces critical issues:

1. **Immediate:** Revert to tagged `v0.1.0` release
2. **Short-term:** Run old and new systems in parallel, compare outputs
3. **Investigation:** Use profiling and logging to identify root cause
4. **Fix-forward:** Apply targeted fixes rather than abandoning refactor

---

## Dependencies

**New Crates:**
- None! This refactoring uses only existing Tokio primitives

**Breaking Changes:**
- Internal only - no config file format changes
- Binary names unchanged (agent, hub)
- Alert behavior identical

---

## Success Criteria

- [ ] All 12 existing integration tests pass
- [ ] New architecture code coverage >80%
- [ ] Performance within 5% of v0.1.0 baseline
- [ ] Memory usage stable over 24hr run
- [ ] Clean shutdown in <5 seconds
- [ ] Code complexity reduced (measured by cyclomatic complexity)
- [ ] Team consensus that new code is more maintainable

---

## References

- [Tokio Tutorial: Channels](https://tokio.rs/tokio/tutorial/channels)
- [Alice Ryhl: Actors with Tokio](https://ryhl.io/blog/actors-with-tokio/)
- [Building an Asynchronous Actor Model in Rust](https://medium.com/@p4524888/leveraging-rusts-tokio-library-for-asynchronous-actor-model-cf6d477afb19)
- Current codebase: [src/monitors/](../../src/monitors/)
