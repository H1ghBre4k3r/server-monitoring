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

## Phase 2: Metric Persistence 💾

**Goal:** Add time-series storage with flexible backend options

**Duration:** 1-2 weeks

### 2.1 Storage Abstraction
- [ ] Design storage trait with CRUD operations
- [ ] Define metric schema (timestamp, server_id, metric_type, value, metadata)
- [ ] Implement batch write operations
- [ ] Add query interface for time ranges and aggregations

### 2.2 Backend Implementations
- [ ] **SQLite backend** (default, embedded)
  - Schema design with indexes
  - Connection pooling
  - Migration system
- [ ] **PostgreSQL backend** (optional, production)
  - TimescaleDB extension support
  - Hypertable configuration
  - Continuous aggregates
- [ ] **Parquet file backend** (optional, archival)
  - Columnar storage with compression
  - Partition by time (daily/hourly files)
  - Efficient range queries

### 2.3 Retention & Aggregation
- [ ] Configurable retention policies per metric type
- [ ] Automatic data pruning/archival
- [ ] Downsampling for long-term storage (1min → 5min → 1hr)
- [ ] Query optimization for large time ranges

### 2.4 Integration
- [ ] Update `StorageActor` to persist all metrics
- [ ] Add configuration for storage backend selection
- [ ] Implement metric replay on startup
- [ ] Add storage health checks

**Dependencies:** Phase 1
**Deliverables:** Persistent metric storage with multiple backend options
**Reference:** [docs/features/METRIC_PERSISTENCE.md](docs/features/METRIC_PERSISTENCE.md)

---

## Phase 3: Service Monitoring 🌐

**Goal:** Add HTTP/HTTPS endpoint monitoring and ICMP ping support

**Duration:** 1 week

### 3.1 HTTP/HTTPS Monitoring
- [ ] Design service check configuration schema
- [ ] Implement HTTP client with timeout/retry logic
- [ ] Support multiple HTTP methods (GET, POST, HEAD)
- [ ] Validate response codes, headers, body patterns
- [ ] Measure response time and SSL cert expiration
- [ ] Track consecutive failures for alerting

### 3.2 ICMP Ping Monitoring
- [ ] Integrate `surge-ping` library
- [ ] Implement ping with configurable packet size/count
- [ ] Measure latency (min/avg/max/stddev)
- [ ] Track packet loss percentage
- [ ] Handle ICMP permission requirements

### 3.3 Service Status Tracking
- [ ] Add service state machine (UP/DOWN/DEGRADED/UNKNOWN)
- [ ] Implement grace periods for flapping detection
- [ ] Store service check history
- [ ] Generate uptime percentage calculations

### 3.4 Alert Integration
- [ ] Extend alert system for service failures
- [ ] Add service-specific alert templates
- [ ] Include response time trends in alerts
- [ ] Support different severity levels

**Dependencies:** Phase 1, Phase 2
**Deliverables:** Comprehensive service health monitoring
**Reference:** [docs/features/SERVICE_MONITORING.md](docs/features/SERVICE_MONITORING.md)

---

## Phase 4: Dashboard & API 📊

**Goal:** Build TUI dashboard and remote API access

**Duration:** 2-3 weeks

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

| Phase | Duration | Start | End |
|-------|----------|-------|-----|
| Phase 1: Architecture | 1-2 weeks | Week 1 | Week 2-3 |
| Phase 2: Persistence | 1-2 weeks | Week 3 | Week 4-5 |
| Phase 3: Services | 1 week | Week 5 | Week 6 |
| Phase 4: Dashboard/API | 2-3 weeks | Week 6 | Week 8-9 |
| Phase 5: Polish | 1-2 weeks | Week 9 | Week 10-11 |

**Total Estimated Time:** 6-10 weeks

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
