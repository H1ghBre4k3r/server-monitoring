# Server Monitoring Roadmap

This roadmap outlines the development plan to transform the current basic monitoring system into a comprehensive, production-ready monitoring platform with persistence, visualization, and service health checks.

## Current State (v0.5.0)

✅ **Implemented:**
- Agent-hub architecture for distributed monitoring
- Real-time CPU usage and temperature monitoring
- Configurable thresholds with grace periods
- Alert system (Discord webhooks, generic webhooks)
- Authentication via tokens
- Actor-based architecture with Tokio actors
- SQLite persistence with configurable retention
- Service health monitoring (HTTP/HTTPS)
- REST API + WebSocket streaming
- TUI dashboard with time-based charts
- Historical data loading and uptime tracking

✅ **Architecture:**
- Clean actor-based design (Collector, Storage, Alert, ServiceMonitor)
- Broadcast channels for event distribution
- Graceful shutdown and supervision
- Pluggable storage backends (SQLite, in-memory)

🎯 **Next Focus:**
- Performance optimization and benchmarking
- Production hardening and observability
- Documentation and deployment guides
- Release binaries and distribution

## Vision (v1.0.0)

A comprehensive monitoring platform featuring:
- 📊 Beautiful terminal UI with real-time graphs
- 💾 Time-series metric storage with configurable retention
- 🌐 Service uptime monitoring (HTTP/HTTPS + ICMP ping)
- 🔌 REST + WebSocket API for remote access
- 🏗️ Clean actor-based architecture
- 📈 Historical trend analysis and reporting

---

## Phase 1: Architecture Refactoring 🏗️ [✅ COMPLETE]

**Goal:** Modernize the codebase with a clean actor-based architecture

**Duration:** 1-2 weeks

**Status:** Complete - All actors integrated with graceful shutdown ✅

### 1.1 Actor Model Design
- [x] Design actor system with clear responsibilities ✅
  - `MetricCollectorActor` - polls agents and collects metrics ✅
  - `StorageActor` - handles all persistence operations ✅
  - `AlertActor` - evaluates rules and sends alerts ✅
  - `ServiceMonitorActor` - monitors service health ✅
- [x] Define message types and communication patterns ✅

### 1.2 Channel Architecture
- [x] Replace current loop-based polling with tokio channels ✅
- [x] Implement `mpsc` channels for actor commands ✅
- [x] Implement `broadcast` channels for metric events ✅
- [x] Add backpressure handling and buffering strategies ✅

### 1.3 Hub Refactoring
- [x] Refactor `hub.rs` to spawn actor tasks ✅
- [x] Implement graceful shutdown for all actors ✅
- [x] Create unified configuration system ✅

### 1.4 Testing & Migration
- [x] Add basic unit tests for actors (29 tests) ✅
- [x] Add integration tests for actor communication (43 tests) ✅
- [x] Ensure backward compatibility with existing configs ✅

**Dependencies:** None
**Deliverables:** Cleaner, more maintainable codebase with actor model
**Reference:** [docs/architecture/REFACTORING_PLAN.md](docs/architecture/REFACTORING_PLAN.md)

**Progress Notes:**
- **2025-01-15**: Created actor module structure (`src/actors/`)
  - Implemented `MetricCollectorActor` - replaces old `server_monitor` loop
  - Implemented `AlertActor` - maintains grace period state machine
  - Implemented `StorageActor` (in-memory stub for Phase 2)
  - Added message types (`MetricEvent`, commands for each actor)
  - Set up broadcast channel for metric distribution
  - All tests passing (5/5) ✅

---

## Phase 2: Metric Persistence 💾 [✅ COMPLETE]

**Goal:** Add time-series storage with flexible backend options

**Duration:** 1-2 weeks

**Status:** Complete - SQLite backend with batching and retention ✅

### 2.1 Storage Abstraction
- [x] Design storage trait with CRUD operations ✅
- [x] Define metric schema (timestamp, server_id, metric_type, value, metadata) ✅
- [x] Implement batch write operations ✅
- [x] Add query interface for time ranges and aggregations ✅

### 2.2 Backend Implementations
- [x] **SQLite backend** (default, embedded) ✅
  - [x] Schema design with indexes ✅
  - [x] Migration system (sqlx) ✅
  - [ ] Connection pooling (using single connection currently)
- [ ] **PostgreSQL backend** (optional, production - Phase 2.5)
  - TimescaleDB extension support
  - Hypertable configuration
  - Continuous aggregates
- [ ] **Parquet file backend** (optional, archival - Phase 2.5)
  - Columnar storage with compression
  - Partition by time (daily/hourly files)
  - Efficient range queries

### 2.3 Retention & Aggregation
- [x] Configurable retention policies (implemented in Phase 4.0) ✅
- [x] Automatic data pruning/archival (implemented in Phase 4.0) ✅
- [ ] Downsampling for long-term storage (1min → 5min → 1hr) → **Future enhancement**
- [ ] Query optimization for large time ranges → **Future enhancement**

### 2.4 Integration
- [x] Update `StorageActor` to persist all metrics ✅
- [x] Add configuration for storage backend selection ✅
- [x] Add storage health checks ✅

**Dependencies:** Phase 1
**Deliverables:** Persistent metric storage with multiple backend options
**Reference:** [docs/features/METRIC_PERSISTENCE.md](docs/features/METRIC_PERSISTENCE.md)

**Progress Notes:**
- **2025-01-15**: SQLite backend implementation complete
  - Created `StorageBackend` trait with async operations
  - Implemented `SqliteBackend` with WAL mode and optimized indexes
  - Designed hybrid schema: aggregate columns (indexed) + complete metadata (JSON)
  - Added batching strategy: dual flush triggers (100 metrics OR 5 seconds)
  - Extended `StorageActor` with `Option<Box<dyn StorageBackend>>` for persistence
  - Configured via `storage` section in config (SQLite or in-memory)
  - All tests passing (84/84) ✅
  - Backward compatible: falls back to in-memory if no storage configured
- **2025-01-16**: Retention and cleanup complete (Phase 4.0)
  - Added `retention_days` and `cleanup_interval_hours` configuration
  - Implemented background cleanup task in `StorageActor`
  - Cleanup runs on startup and at configured intervals
  - Statistics tracking: last cleanup time, metrics/checks deleted

---

## Phase 3: Service Monitoring 🌐 [✅ COMPLETE]

**Goal:** Add HTTP/HTTPS endpoint monitoring and ICMP ping support

**Duration:** 1 week

**Status:** Complete - Service monitoring with persistence and alerts ✅

### 3.1 HTTP/HTTPS Monitoring
- [x] Design service check configuration schema ✅
- [x] Implement HTTP client with timeout/retry logic ✅
- [x] Support multiple HTTP methods (GET, POST, HEAD) ✅
- [x] Validate response codes, headers, body patterns ✅
- [x] Measure response time and SSL cert expiration ✅
- [x] Track consecutive failures for alerting ✅
- [ ] ICMP Ping Monitoring (deferred to v1.1.0)

### 3.2 Service Status Tracking
- [x] Add service state machine (UP/DOWN/DEGRADED) ✅
- [x] Implement grace periods for flapping detection ✅
- [x] Store service check history (SQLite + in-memory) ✅
- [x] Generate uptime percentage calculations ✅

### 3.3 Alert Integration
- [x] Extend alert system for service failures ✅
- [x] Add service-specific alert templates (Discord + Webhook) ✅
- [x] Include error messages in alerts ✅
- [x] Support status transitions (down → recovery) ✅

### 3.4 Storage & Query API
- [x] ServiceCheckRow schema with persistence ✅
- [x] Public query API in StorageHandle ✅
  - `query_service_checks_range()` - time range queries
  - `query_latest_service_checks()` - latest N checks
  - `calculate_uptime()` - uptime statistics
- [x] Integration tests (persistence, uptime, range queries) ✅

**Dependencies:** Phase 1, Phase 2
**Deliverables:** Comprehensive service health monitoring
**Reference:** [docs/features/SERVICE_MONITORING.md](docs/features/SERVICE_MONITORING.md)

**Progress Notes:**
- **2025-01-15**: Service monitoring implementation complete
  - Created `ServiceMonitorActor` with configurable check intervals
  - Implemented HTTP/HTTPS health checks with method, body pattern, header validation
  - Added `ServiceCheckEvent` messages published to broadcast channel
  - Extended `StorageActor` to persist service checks to SQLite
  - Implemented `send_service_alert()` in AlertManager (Discord + Webhook)
  - Added uptime calculation with SQL aggregation (percentage, avg response time)
  - Created public query API in StorageHandle for dashboard/API access
  - All tests passing (84/84: 29 unit + 43 integration + 9 property + 3 doc) ✅
  - ICMP ping monitoring deferred to future release (requires elevated permissions)

---

## Phase 3.5: Alert Architecture Refactoring 🔔

**Goal:** Split metric and service alert managers for cleaner architecture

**Duration:** 3-5 days

**Priority:** Medium (after Phase 4.1 - do after retention cleanup and basic API)

### 3.5.1 Design Alert Abstraction
- [ ] Design `AlertSender` trait for shared Discord/Webhook delivery logic
- [ ] Define interface for `MetricAlertManager` (CPU, temp, disk, memory)
- [ ] Define interface for `ServiceAlertManager` (uptime, SSL, response times)
- [ ] Plan migration path from current `AlertManager`

### 3.5.2 Implementation
- [ ] Implement `AlertSender` trait with Discord and Webhook backends
- [ ] Extract `MetricAlertManager` from current `AlertManager`
- [ ] Extract `ServiceAlertManager` from current `AlertManager`
- [ ] Update `AlertActor` to use separate managers
- [ ] Remove old `AlertManager` once migration complete

### 3.5.3 Testing & Documentation
- [ ] Update unit tests for new architecture
- [ ] Update integration tests for alert flows
- [ ] Document alert manager selection logic
- [ ] Add examples for custom alert types

**Why This Refactoring:**
- Current `AlertManager` handles two conceptually different domains (metrics vs services)
- Different alert patterns: metrics use thresholds, services need SLA tracking
- Easier to add new alert types in the future (log alerts, security alerts)
- Better separation of concerns and testability

**Why Not Urgent:**
- Current implementation works without bugs or performance issues
- More critical features needed first (retention, dashboard)
- Will understand pain points better after real-world usage

**Dependencies:** Phase 3
**Deliverables:** Cleaner alert architecture ready for future extension

---

## Phase 4: Dashboard & API 📊 [✅ COMPLETE]

**Goal:** Build TUI dashboard and remote API access

**Duration:** 2-3 weeks

**Status:** Complete - Full API and TUI dashboard implemented ✅

### 4.0 Retention & Cleanup [✅ COMPLETE]
- [x] Implement background task for automatic data pruning ✅
- [x] Add configurable retention policies per metric type ✅
- [x] Cleanup old metrics on hub startup ✅
- [x] Add retention statistics to storage stats ✅
- [x] Add metrics for cleanup operations (rows deleted) ✅

### 4.1 API Server (Axum) [✅ COMPLETE]
- [x] Design REST API specification ✅
  - `GET /api/v1/health` - health check
  - `GET /api/v1/stats` - system statistics
  - `GET /api/v1/servers` - list all monitored servers with health status
  - `GET /api/v1/servers/{id}/metrics` - query metrics with time range
  - `GET /api/v1/servers/{id}/metrics/latest` - latest N metrics
  - `GET /api/v1/services` - list all monitored services with health status
  - `GET /api/v1/services/{name}/checks` - service check history
  - `GET /api/v1/services/{name}/uptime` - uptime statistics
- [x] Implement request authentication/authorization (Bearer token) ✅
- [x] WebSocket endpoint for real-time metric streaming (`/api/v1/stream`) ✅
- [x] CORS support for web dashboards ✅

### 4.2 WebSocket Streaming [✅ COMPLETE]
- [x] Implement `tokio-tungstenite` WebSocket handler ✅
- [x] Subscribe to metric broadcast channel ✅
- [x] Subscribe to service check broadcast channel ✅
- [x] Filter and serialize events for clients ✅
- [x] Handle client reconnection and buffering ✅

### 4.3 TUI Dashboard (Ratatui) [✅ COMPLETE]
- [x] Initialize Ratatui with Crossterm backend ✅
- [x] Implement tabbed interface layout ✅
  - **Servers Tab:** Server list + detailed metrics with time-based charts
  - **Services Tab:** Service health status with check history
  - **Alerts Tab:** Alert timeline with severity indicators
- [x] Create chart components with time-based X-axis (HH:MM:SS) ✅
- [x] Enhanced system info panel (hostname, OS, architecture) ✅
- [x] Color-coded memory gauges with progress bars ✅
- [x] Implement real-time updates via WebSocket ✅
- [x] Add interactive controls (pause, refresh, navigation) ✅
- [x] Sliding time window for charts (configurable, default 5 minutes) ✅
- [x] Historical data loading on startup ✅

### 4.4 CLI Binary (`guardia-viewer`) [✅ COMPLETE]
- [x] Create new binary in `src/bin/viewer.rs` ✅
- [x] Support connection to local or remote hub ✅
- [x] Configuration file for API endpoint and auth (`~/.config/guardia/viewer.toml`) ✅
- [x] Graceful error handling and automatic reconnection ✅
- [x] Help text and keybindings display ✅
- [x] CLI arguments for URL and token override ✅

**Dependencies:** Phase 1, Phase 2, Phase 3
**Deliverables:** Beautiful TUI dashboard and flexible API ✅
**Reference:** See CLAUDE.md for detailed architecture documentation

**Progress Notes:**
- **2025-01-16**: Phase 4.0 (Retention & Cleanup) complete
  - Background task for automatic metric/service check pruning
  - Configurable retention policies and cleanup intervals
  - Cleanup statistics tracking in StorageActor
- **2025-01-16**: Phase 4.1 (API Server) complete
  - Full REST API with Axum framework
  - All endpoints implemented (health, stats, servers, services)
  - Bearer token authentication and CORS support
  - WebSocket streaming for real-time updates
- **2025-01-16**: Phase 4.2 (TUI Dashboard) complete
  - Three-tab interface (Servers, Services, Alerts)
  - Time-based charts with sliding window (HH:MM:SS labels)
  - Enhanced memory visualization with color-coded gauges
  - Historical data loading on startup
  - WebSocket integration with automatic reconnection
  - TOML configuration support with CLI overrides
  - All tests passing (84/84) ✅

---

## Phase 5: Polish & Production Readiness 🚀 [🎯 IN PROGRESS]

**Goal:** Optimize, document, and prepare for production deployment

**Duration:** 1-2 weeks

**Status:** In progress - focus on v1.0.0 release ✨

### 5.1 Performance Optimization
- [ ] Profile CPU and memory usage under load
- [ ] Optimize database queries and indexes
- [ ] Implement connection pooling for SQLite
- [ ] Add caching layer for frequent queries (server list, service status)
- [ ] Benchmark metric throughput (target: 10k metrics/sec)
- [ ] Load testing with multiple agents and services

### 5.2 Observability
- [ ] Add structured logging with log levels (tracing/serde_json)
- [ ] Implement metrics about the monitoring system itself (meta-monitoring)
  - Actor health and message queue depths
  - Storage backend performance metrics
  - API request/response times
- [ ] Enhanced health check endpoints (storage, actors, connectivity)
- [ ] Add distributed tracing support (optional - opentelemetry)

### 5.3 Documentation
- [ ] Complete API documentation (OpenAPI/Swagger spec)
- [ ] Write deployment guides (systemd, Docker, Kubernetes)
- [ ] Create troubleshooting guide (common issues, debugging)
- [ ] Add example configurations (production, development, minimal)
- [ ] Record demo videos/screenshots for TUI dashboard
- [ ] Architecture diagrams (actor communication, data flow)

### 5.4 Distribution
- [ ] Create release binaries for major platforms (Linux, macOS, Windows)
- [ ] Docker images with multi-stage builds
  - Hub image
  - Agent image
  - All-in-one demo image
- [ ] Installation scripts (curl | bash installer)
- [ ] Homebrew formula (macOS)
- [ ] Package for apt/yum (Linux distributions)
- [ ] GitHub Actions for automated releases

### 5.5 Testing & Quality
- [x] Good unit test coverage (29 unit tests) ✅
- [x] Integration tests for actor communication (43 tests) ✅
- [x] Property-based tests (9 tests) ✅
- [ ] Performance regression tests
- [ ] Chaos testing (network failures, high load, disk full)
- [ ] End-to-end tests (agent → hub → dashboard)
- [ ] Security audit (dependency scanning, SAST)

### 5.6 Configuration & UX
- [ ] Configuration validation with helpful error messages
- [ ] Migration tool for config format changes
- [ ] Environment variable support for sensitive values
- [ ] Wizard/interactive setup for first-time users
- [ ] Better CLI help and examples

**Dependencies:** Phase 1-4 ✅
**Deliverables:** Production-ready v1.0.0 release with binaries and documentation
**Target:** Q1 2025

---

## Future Enhancements (v1.1.0+)

### Possible Features
- 📱 Mobile app for monitoring on-the-go
- 🔔 Additional alert channels (Slack, PagerDuty, email)
- 📊 Web UI (alternative to TUI)
- 🤖 Anomaly detection with ML
- 📝 Custom metric plugins via WASM
- 🌍 Geo-distributed monitoring
- 📈 Custom dashboards and reports
- 🔐 Multi-tenancy support
- 🔄 Configuration management UI
- 🎯 SLA tracking and reporting

---

## Timeline Summary

| Phase | Duration | Status | Completion Date | Notes |
|-------|----------|--------|-----------------|-------|
| Phase 1: Architecture | 1-2 weeks | ✅ COMPLETE | 2025-01-15 | Actor-based architecture with graceful shutdown |
| Phase 2: Persistence | 1-2 weeks | ✅ COMPLETE | 2025-01-15 | SQLite backend with batching and hybrid schema |
| Phase 3: Services | 1 week | ✅ COMPLETE | 2025-01-15 | HTTP/HTTPS monitoring with alerts and uptime |
| Phase 4.0: Retention | 2-3 days | ✅ COMPLETE | 2025-01-16 | Automatic cleanup with configurable policies |
| Phase 4.1: API Server | 1 week | ✅ COMPLETE | 2025-01-16 | REST API + WebSocket streaming |
| Phase 4.2: TUI Dashboard | 1 week | ✅ COMPLETE | 2025-01-16 | Ratatui dashboard with time-based charts |
| Phase 5: Polish | 1-2 weeks | 🎯 IN PROGRESS | Target: Q1 2025 | Production readiness, optimization |
| Phase 3.5: Alert Refactoring | 3-5 days | 📋 PLANNED | After v1.0.0 | Medium priority - split metric/service alerts |

**Progress (as of 2025-01-16):**
- ✅ Core features complete: All of Phases 1-4 (100%)
- ✅ Test coverage: 84 tests passing (29 unit + 43 integration + 9 property + 3 doc)
- 🎯 Current: Phase 5 (Production hardening and optimization)
- 📋 Target: v1.0.0 release in Q1 2025
- 📋 Post-release: Phase 3.5 (Alert architecture refactoring)

---

## Success Metrics

**v1.0.0 Goals:**
- ✅ Zero-downtime metric collection (actor-based architecture implemented)
- ✅ Storage: Persistent SQLite backend with configurable retention
- ✅ Dashboard: Sub-second TUI responsiveness with real-time updates achieved
- ✅ API: WebSocket streaming with Bearer token authentication
- ✅ Services: HTTP/HTTPS health checks with uptime tracking
- ✅ Test Coverage: 84 tests (29 unit + 43 integration + 9 property + 3 doc)
- 🎯 Reliability: 99.9% uptime for monitoring itself (needs production validation)
- 🎯 Performance: 10k metrics/sec throughput (needs benchmarking in Phase 5)
- 🎯 Documentation: Complete deployment guides and API docs (in progress)
- 🎯 Distribution: Release binaries for Linux, macOS, Windows (planned)

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Performance degradation with actor model | High | Thorough benchmarking in Phase 1 |
| Database scalability limits | Medium | Design for horizontal sharding from start |
| TUI complexity and bugs | Medium | Incremental UI development with testing |
| ICMP permission issues | Low | Clear documentation for capability setup |
| Backward compatibility breaks | Medium | Maintain config migration path |

---

## Contributing

This is a living document. As development progresses:
1. Check off completed items with ✅
2. Add notes about implementation decisions
3. Update timelines based on actual progress
4. Document any deviations from the plan

For detailed technical specifications, see the linked documents in each phase.
