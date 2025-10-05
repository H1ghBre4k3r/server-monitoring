# Server Monitoring Roadmap

This roadmap outlines the development plan to transform the current basic monitoring system into a comprehensive, production-ready monitoring platform with persistence, visualization, and service health checks.

## Current State (v0.1.0)

✅ **Implemented:**
- Agent-hub architecture for distributed monitoring
- Real-time CPU usage and temperature monitoring
- Configurable thresholds with grace periods
- Alert system (Discord webhooks, generic webhooks)
- Authentication via tokens

❌ **Limitations:**
- No metric persistence (all data is ephemeral)
- No historical data or trend analysis
- No visualization or dashboard
- Only resource monitoring (no service/endpoint checks)
- Thread-based architecture with tight coupling
- No API for external access

## Vision (v1.0.0)

A comprehensive monitoring platform featuring:
- 📊 Beautiful terminal UI with real-time graphs
- 💾 Time-series metric storage with configurable retention
- 🌐 Service uptime monitoring (HTTP/HTTPS + ICMP ping)
- 🔌 REST + WebSocket API for remote access
- 🏗️ Clean actor-based architecture
- 📈 Historical trend analysis and reporting

---

## Phase 1: Architecture Refactoring 🏗️ [IN PROGRESS]

**Goal:** Modernize the codebase with a clean actor-based architecture

**Duration:** 1-2 weeks

**Status:** Week 1 - Core actor infrastructure complete ✅

### 1.1 Actor Model Design
- [x] Design actor system with clear responsibilities
  - `MetricCollectorActor` - polls agents and collects metrics ✅
  - `StorageActor` - handles all persistence operations (stub) ✅
  - `AlertActor` - evaluates rules and sends alerts ✅
  - `ServiceMonitorActor` - monitors service health (Phase 3)
  - `ApiActor` - handles external API requests (Phase 4)
- [x] Define message types and communication patterns ✅
- [ ] Document actor lifecycle and supervision strategy

### 1.2 Channel Architecture
- [x] Replace current loop-based polling with tokio channels ✅
- [x] Implement `mpsc` channels for actor commands ✅
- [x] Implement `broadcast` channels for metric events ✅
- [x] Add backpressure handling and buffering strategies ✅

### 1.3 Hub Refactoring
- [ ] Refactor `hub.rs` to spawn actor tasks (NEXT STEP)
- [ ] Implement graceful shutdown for all actors
- [ ] Add actor health monitoring
- [ ] Create unified configuration system

### 1.4 Testing & Migration
- [x] Add basic unit tests for actors ✅
- [ ] Add integration tests for actor communication
- [ ] Ensure backward compatibility with existing configs
- [ ] Performance benchmarking vs current implementation
- [ ] Documentation for new architecture

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

## Phase 2: Metric Persistence 💾 [IN PROGRESS]

**Goal:** Add time-series storage with flexible backend options

**Duration:** 1-2 weeks

**Status:** Week 1 - SQLite backend implementation complete ✅

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
- [ ] Configurable retention policies per metric type → **Moved to Phase 4.0**
- [ ] Automatic data pruning/archival → **Moved to Phase 4.0**
- [ ] Downsampling for long-term storage (1min → 5min → 1hr) → **Future enhancement**
- [ ] Query optimization for large time ranges → **Future enhancement**

### 2.4 Integration
- [x] Update `StorageActor` to persist all metrics ✅
- [x] Add configuration for storage backend selection ✅
- [x] Add storage health checks ✅
- [ ] Implement metric replay on startup

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
  - All tests passing (60/60) ✅
  - Backward compatible: falls back to in-memory if no storage configured

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
  - All tests passing (75/75: 29 unit + 34 integration + 9 property + 3 doc) ✅
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

## Phase 4: Dashboard & API 📊

**Goal:** Build TUI dashboard and remote API access

**Duration:** 2-3 weeks

### 4.0 Retention & Cleanup (High Priority - Do First)
- [ ] Implement background task for automatic data pruning
- [ ] Add configurable retention policies per metric type
- [ ] Cleanup old metrics on hub startup
- [ ] Add retention statistics to health check endpoint
- [ ] Document storage space requirements and growth patterns
- [ ] Add metrics for cleanup operations (rows deleted, space reclaimed)

**Note:** Moved from Phase 2.3 - production necessity to prevent disk space issues

### 4.1 API Server (Axum)
- [ ] Design REST API specification
  - `GET /api/v1/servers` - list all monitored servers
  - `GET /api/v1/servers/{id}/metrics` - query metrics
  - `GET /api/v1/services` - list all monitored services
  - `GET /api/v1/alerts/history` - alert history
- [ ] Implement request authentication/authorization
- [ ] Add rate limiting and request validation
- [ ] WebSocket endpoint for real-time metric streaming
- [ ] API documentation (OpenAPI/Swagger)

### 4.2 WebSocket Streaming
- [ ] Implement `tokio-tungstenite` WebSocket handler
- [ ] Subscribe to metric broadcast channel
- [ ] Filter and serialize metrics for clients
- [ ] Handle client reconnection and buffering
- [ ] Add compression for bandwidth efficiency

### 4.3 TUI Dashboard (Ratatui)
- [ ] Initialize Ratatui with Crossterm backend
- [ ] Implement tabbed interface layout
  - **Overview Tab:** All servers at a glance
  - **Server Detail Tabs:** Per-server graphs
  - **Services Tab:** Service health status
  - **Alerts Tab:** Recent alerts and history
- [ ] Create chart components with threshold lines
- [ ] Add sparklines for compact metric display
- [ ] Implement real-time updates via WebSocket
- [ ] Add interactive controls (pause, zoom, time range)

### 4.4 CLI Binary (`guardia-viewer`)
- [ ] Create new binary in `src/bin/viewer.rs`
- [ ] Support connection to local or remote hub
- [ ] Configuration file for API endpoint and auth
- [ ] Graceful error handling and reconnection
- [ ] Help text and keybindings display

**Dependencies:** Phase 1, Phase 2, Phase 3
**Deliverables:** Beautiful TUI dashboard and flexible API
**Reference:** [docs/features/DASHBOARD.md](docs/features/DASHBOARD.md), [docs/api/API_DESIGN.md](docs/api/API_DESIGN.md)

---

## Phase 5: Polish & Production Readiness 🚀

**Goal:** Optimize, document, and prepare for production deployment

**Duration:** 1-2 weeks

### 5.1 Performance Optimization
- [ ] Profile CPU and memory usage under load
- [ ] Optimize database queries and indexes
- [ ] Implement connection pooling
- [ ] Add caching layer for frequent queries
- [ ] Benchmark metric throughput (targets: 10k metrics/sec)

### 5.2 Observability
- [ ] Add structured logging throughout
- [ ] Implement metrics about the monitoring system itself (meta-monitoring)
- [ ] Create health check endpoints
- [ ] Add distributed tracing support (optional)

### 5.3 Documentation
- [ ] Complete API documentation
- [ ] Write deployment guides
- [ ] Create troubleshooting guide
- [ ] Add example configurations
- [ ] Record demo videos/screenshots

### 5.4 Distribution
- [ ] Create release binaries for major platforms
- [ ] Docker images with examples
- [ ] Installation scripts
- [ ] Homebrew formula (macOS)
- [ ] Package for apt/yum (Linux)

### 5.5 Testing
- [ ] Expand unit test coverage (target: 80%)
- [ ] Add integration tests
- [ ] Performance regression tests
- [ ] Chaos testing (network failures, high load)

**Dependencies:** Phase 1-4
**Deliverables:** Production-ready v1.0.0 release

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

| Phase | Duration | Status | Notes |
|-------|----------|--------|-------|
| Phase 1: Architecture | 1-2 weeks | ✅ COMPLETE | Actor-based architecture implemented |
| Phase 2: Persistence | 1-2 weeks | ✅ COMPLETE | SQLite backend with batching |
| Phase 3: Services | 1 week | ✅ COMPLETE | HTTP/HTTPS monitoring with alerts |
| Phase 3.5: Alert Refactoring | 3-5 days | 📋 PLANNED | Do after Phase 4.1 (medium priority) |
| Phase 4: Dashboard/API | 2-3 weeks | 🎯 NEXT | Start with retention cleanup |
| Phase 5: Polish | 1-2 weeks | 📋 PLANNED | Production readiness |

**Progress:**
- ✅ Phases 1-3 complete (3 weeks)
- 🎯 Next: Phase 4.0 (Retention cleanup)
- 📋 Remaining: ~3-5 weeks to v1.0.0

---

## Success Metrics

**v1.0.0 Goals:**
- ✅ Zero-downtime metric collection
- ✅ Storage: 1M+ metrics without performance degradation
- ✅ Dashboard: Sub-second UI responsiveness
- ✅ API: 1000+ concurrent WebSocket connections
- ✅ Services: Check 100+ endpoints at 10s intervals
- ✅ Reliability: 99.9% uptime for monitoring itself

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
