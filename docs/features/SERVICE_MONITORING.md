# Service Uptime Monitoring

## Overview

This document describes the design and implementation of service uptime monitoring, which extends the current system from server resource monitoring to include endpoint health checks (HTTP/HTTPS) and network reachability (ICMP ping).

---

## Requirements

### Functional Requirements

1. **HTTP/HTTPS endpoint checks** with configurable intervals
2. **ICMP ping checks** for network reachability
3. **Response time tracking** (min/avg/max/p95)
4. **Status detection** (UP/DOWN/DEGRADED)
5. **SSL certificate expiration warnings**
6. **Custom validation rules** (status code, response body patterns, headers)
7. **Uptime percentage calculations** (24h, 7d, 30d)
8. **Alert on status changes** with grace periods

### Non-Functional Requirements

1. **Performance:** Check 100+ services every 10 seconds
2. **Reliability:** Minimal false positives (retry logic)
3. **Accuracy:** Sub-millisecond timing precision
4. **Scalability:** Support 1000+ services
5. **Extensibility:** Easy to add new check types

---

## Check Types

### 1. HTTP/HTTPS Checks

**Purpose:** Monitor web endpoints, APIs, and web services

**Features:**
- Support GET, POST, HEAD, PUT, DELETE methods
- Custom headers and body
- SSL/TLS verification
- Certificate expiration detection
- Response validation (status code, body regex, JSON path)
- Follow redirects (configurable)
- Timeout and retry logic

**Configuration:**

```json
{
  "services": [
    {
      "id": "api-prod",
      "name": "Production API",
      "type": "http",
      "url": "https://api.example.com/health",
      "method": "GET",
      "interval": 30,
      "timeout": 5,
      "retries": 2,
      "expected_status": [200, 204],
      "headers": {
        "User-Agent": "Guardia-Monitor/1.0",
        "Authorization": "Bearer ${API_TOKEN}"
      },
      "validation": {
        "body_contains": "\"status\":\"healthy\"",
        "json_path": "$.status",
        "json_value": "healthy"
      },
      "ssl": {
        "verify": true,
        "warn_days_before_expiry": 30
      },
      "alert": {
        "discord": {
          "url": "https://discord.com/api/webhooks/...",
          "user_id": "123456789"
        }
      }
    },
    {
      "id": "legacy-http",
      "name": "Legacy HTTP Service",
      "type": "http",
      "url": "http://10.0.1.50:8080/status",
      "interval": 60,
      "timeout": 10,
      "expected_status": [200]
    }
  ]
}
```

### 2. ICMP Ping Checks

**Purpose:** Monitor network reachability and latency

**Features:**
- Standard ICMP echo request/reply
- Configurable packet size and count
- Latency statistics (min/avg/max/stddev)
- Packet loss detection
- IPv4 and IPv6 support

**Configuration:**

```json
{
  "services": [
    {
      "id": "gateway",
      "name": "Network Gateway",
      "type": "ping",
      "host": "192.168.1.1",
      "interval": 10,
      "timeout": 5,
      "count": 3,
      "packet_size": 56,
      "max_loss_percent": 20,
      "max_avg_latency_ms": 100,
      "alert": {
        "webhook": {
          "url": "https://monitoring.example.com/webhook"
        }
      }
    }
  ]
}
```

---

## Implementation

### Library Selection

| Check Type | Library | Why |
|------------|---------|-----|
| HTTP/HTTPS | **reqwest** | Already used, async, excellent ecosystem |
| ICMP Ping | **surge-ping** | Async tokio-based, actively maintained |

### Dependencies

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
surge-ping = "0.8"
x509-parser = "0.16"  # For SSL cert parsing
regex = "1.10"
```

---

## Architecture

### ServiceMonitorActor

```rust
// src/actors/service_monitor.rs

use tokio::sync::{mpsc, broadcast};
use tokio::time::{interval, Duration, Instant};

pub struct ServiceMonitorActor {
    services: Vec<ServiceCheck>,
    command_rx: mpsc::Receiver<ServiceMonitorCommand>,
    result_tx: broadcast::Sender<ServiceCheckResult>,
    client: reqwest::Client,
    pinger: Arc<Pinger>,
}

pub enum ServiceMonitorCommand {
    AddService(ServiceConfig),
    RemoveService(String),
    CheckNow { service_id: String, respond_to: oneshot::Sender<Result<ServiceCheckResult>> },
    GetStatus { service_id: String, respond_to: oneshot::Sender<Result<ServiceStatus>> },
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct ServiceCheckResult {
    pub service_id: String,
    pub timestamp: DateTime<Utc>,
    pub status: ServiceStatus,
    pub response_time_ms: Option<f64>,
    pub error: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Up,
    Down,
    Degraded,
    Unknown,
}

impl ServiceMonitorActor {
    pub fn new(
        services: Vec<ServiceConfig>,
        command_rx: mpsc::Receiver<ServiceMonitorCommand>,
        result_tx: broadcast::Sender<ServiceCheckResult>,
    ) -> Self {
        Self {
            services: services.into_iter().map(ServiceCheck::from).collect(),
            command_rx,
            result_tx,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            pinger: Arc::new(Pinger::new()?),
        }
    }

    pub async fn run(mut self) {
        // Each service gets its own task with its own interval timer
        let mut service_handles = Vec::new();

        for service in self.services {
            let result_tx = self.result_tx.clone();
            let client = self.client.clone();
            let pinger = self.pinger.clone();

            let handle = tokio::spawn(async move {
                service.run_loop(client, pinger, result_tx).await;
            });

            service_handles.push(handle);
        }

        // Handle commands
        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                ServiceMonitorCommand::CheckNow { service_id, respond_to } => {
                    // Trigger immediate check
                    // (implementation would send message to specific service task)
                }
                ServiceMonitorCommand::AddService(config) => {
                    // Spawn new service check task
                }
                ServiceMonitorCommand::RemoveService(id) => {
                    // Stop and remove service task
                }
                ServiceMonitorCommand::Shutdown => {
                    break;
                }
                _ => {}
            }
        }

        // Shutdown all service tasks
        for handle in service_handles {
            handle.abort();
        }
    }
}
```

### Service Check Implementations

```rust
// src/monitors/service_check.rs

pub struct ServiceCheck {
    config: ServiceConfig,
    state: ServiceState,
}

struct ServiceState {
    consecutive_failures: usize,
    consecutive_successes: usize,
    last_status: ServiceStatus,
    response_times: VecDeque<f64>,
    last_check: Option<DateTime<Utc>>,
    total_checks: u64,
    failed_checks: u64,
}

impl ServiceCheck {
    pub async fn run_loop(
        mut self,
        client: reqwest::Client,
        pinger: Arc<Pinger>,
        result_tx: broadcast::Sender<ServiceCheckResult>,
    ) {
        let mut ticker = interval(Duration::from_secs(self.config.interval as u64));

        loop {
            ticker.tick().await;

            let result = match &self.config.check_type {
                CheckType::Http(http_config) => {
                    self.check_http(&client, http_config).await
                }
                CheckType::Ping(ping_config) => {
                    self.check_ping(&pinger, ping_config).await
                }
            };

            // Update state
            self.update_state(&result);

            // Publish result
            let _ = result_tx.send(result);

            // Check if status changed and should alert
            if self.should_alert() {
                self.trigger_alert().await;
            }
        }
    }

    async fn check_http(
        &self,
        client: &reqwest::Client,
        config: &HttpCheckConfig,
    ) -> ServiceCheckResult {
        let start = Instant::now();

        // Build request
        let mut request = match config.method.as_str() {
            "GET" => client.get(&config.url),
            "POST" => client.post(&config.url),
            "HEAD" => client.head(&config.url),
            "PUT" => client.put(&config.url),
            "DELETE" => client.delete(&config.url),
            _ => client.get(&config.url),
        };

        // Add headers
        for (key, value) in &config.headers {
            request = request.header(key, value);
        }

        // Add body if present
        if let Some(body) = &config.body {
            request = request.body(body.clone());
        }

        // Set timeout
        request = request.timeout(Duration::from_secs(config.timeout as u64));

        // Execute request (with retries)
        let mut last_error = None;
        for attempt in 0..=config.retries {
            match request.try_clone().unwrap().send().await {
                Ok(response) => {
                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    let status_code = response.status().as_u16();

                    // Check status code
                    let status = if config.expected_status.contains(&status_code) {
                        ServiceStatus::Up
                    } else {
                        ServiceStatus::Down
                    };

                    // Validate response body if configured
                    let body_text = response.text().await.unwrap_or_default();
                    let validated = self.validate_response(config, &body_text);

                    let final_status = if !validated {
                        ServiceStatus::Down
                    } else {
                        status
                    };

                    // Check SSL certificate if HTTPS
                    let ssl_metadata = if config.url.starts_with("https") {
                        self.check_ssl_cert(&config.url).await
                    } else {
                        serde_json::Value::Null
                    };

                    return ServiceCheckResult {
                        service_id: self.config.id.clone(),
                        timestamp: Utc::now(),
                        status: final_status,
                        response_time_ms: Some(elapsed),
                        error: None,
                        metadata: json!({
                            "status_code": status_code,
                            "attempt": attempt + 1,
                            "ssl": ssl_metadata,
                        }),
                    };
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    if attempt < config.retries {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }

        // All retries failed
        ServiceCheckResult {
            service_id: self.config.id.clone(),
            timestamp: Utc::now(),
            status: ServiceStatus::Down,
            response_time_ms: None,
            error: last_error,
            metadata: json!({
                "retries": config.retries,
            }),
        }
    }

    fn validate_response(&self, config: &HttpCheckConfig, body: &str) -> bool {
        // Check body contains
        if let Some(pattern) = &config.validation.body_contains {
            if !body.contains(pattern) {
                return false;
            }
        }

        // Check JSON path
        if let Some(json_path) = &config.validation.json_path {
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body) {
                // Simple JSON path evaluation (for production, use jsonpath crate)
                // ...
            }
        }

        true
    }

    async fn check_ssl_cert(&self, url: &str) -> serde_json::Value {
        // Parse URL to get host
        let host = url.split("://").nth(1).and_then(|s| s.split('/').next());

        if let Some(host) = host {
            // Connect to host and get cert
            // Use x509-parser to parse certificate
            // Extract expiration date
            // Return metadata
            json!({
                "expires_in_days": 45,
                "issuer": "Let's Encrypt",
            })
        } else {
            serde_json::Value::Null
        }
    }

    async fn check_ping(
        &self,
        pinger: &Arc<Pinger>,
        config: &PingCheckConfig,
    ) -> ServiceCheckResult {
        use surge_ping::PingIdentifier;

        let payload = vec![0; config.packet_size];
        let mut results = Vec::new();
        let mut packet_loss = 0;

        for seq in 0..config.count {
            let start = Instant::now();

            match pinger.ping(
                PingIdentifier(rand::random()),
                config.host.parse().unwrap(),
            ).await {
                Ok((_packet, duration)) => {
                    results.push(duration.as_secs_f64() * 1000.0);
                }
                Err(_) => {
                    packet_loss += 1;
                }
            }
        }

        let loss_percent = (packet_loss as f64 / config.count as f64) * 100.0;

        let status = if loss_percent > config.max_loss_percent {
            ServiceStatus::Down
        } else if !results.is_empty() {
            let avg_latency = results.iter().sum::<f64>() / results.len() as f64;
            if avg_latency > config.max_avg_latency_ms {
                ServiceStatus::Degraded
            } else {
                ServiceStatus::Up
            }
        } else {
            ServiceStatus::Down
        };

        let stats = if !results.is_empty() {
            let min = results.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = results.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let avg = results.iter().sum::<f64>() / results.len() as f64;

            // Calculate stddev
            let variance = results.iter()
                .map(|x| (x - avg).powi(2))
                .sum::<f64>() / results.len() as f64;
            let stddev = variance.sqrt();

            json!({
                "min_ms": min,
                "max_ms": max,
                "avg_ms": avg,
                "stddev_ms": stddev,
                "packet_loss_percent": loss_percent,
            })
        } else {
            json!({
                "packet_loss_percent": 100.0,
            })
        };

        ServiceCheckResult {
            service_id: self.config.id.clone(),
            timestamp: Utc::now(),
            status,
            response_time_ms: results.first().copied(),
            error: if status == ServiceStatus::Down {
                Some(format!("{}% packet loss", loss_percent))
            } else {
                None
            },
            metadata: stats,
        }
    }

    fn update_state(&mut self, result: &ServiceCheckResult) {
        self.state.last_check = Some(result.timestamp);
        self.state.total_checks += 1;

        if result.status == ServiceStatus::Down {
            self.state.consecutive_failures += 1;
            self.state.consecutive_successes = 0;
            self.state.failed_checks += 1;
        } else {
            self.state.consecutive_successes += 1;
            self.state.consecutive_failures = 0;
        }

        // Update response time history
        if let Some(rt) = result.response_time_ms {
            self.state.response_times.push_back(rt);
            if self.state.response_times.len() > 100 {
                self.state.response_times.pop_front();
            }
        }

        // Update status with grace period
        let grace = self.config.alert.as_ref().and_then(|a| a.grace).unwrap_or(0);

        if self.state.consecutive_failures > grace && self.state.last_status != ServiceStatus::Down {
            self.state.last_status = ServiceStatus::Down;
        } else if self.state.consecutive_successes > grace && self.state.last_status != ServiceStatus::Up {
            self.state.last_status = ServiceStatus::Up;
        }
    }

    fn should_alert(&self) -> bool {
        let grace = self.config.alert.as_ref().and_then(|a| a.grace).unwrap_or(0);

        // Alert when status just changed after grace period
        (self.state.consecutive_failures == grace + 1) ||
        (self.state.consecutive_successes == grace + 1 && self.state.last_status == ServiceStatus::Down)
    }

    async fn trigger_alert(&self) {
        // Use existing AlertManager to send alerts
        // ...
    }
}
```

---

## Configuration Schema

```rust
// src/config.rs additions

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CheckType {
    Http(HttpCheckConfig),
    Ping(PingCheckConfig),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub check_type: CheckType,
    pub alert: Option<Alert>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpCheckConfig {
    pub url: String,
    #[serde(default = "default_http_method")]
    pub method: String,
    #[serde(default = "default_interval")]
    pub interval: usize,
    #[serde(default = "default_timeout")]
    pub timeout: usize,
    #[serde(default)]
    pub retries: usize,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    #[serde(default = "default_expected_status")]
    pub expected_status: Vec<u16>,
    #[serde(default)]
    pub validation: HttpValidation,
    #[serde(default)]
    pub ssl: SslConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HttpValidation {
    pub body_contains: Option<String>,
    pub json_path: Option<String>,
    pub json_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SslConfig {
    #[serde(default = "default_true")]
    pub verify: bool,
    #[serde(default = "default_ssl_warn_days")]
    pub warn_days_before_expiry: u32,
}

impl Default for SslConfig {
    fn default() -> Self {
        Self {
            verify: true,
            warn_days_before_expiry: 30,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PingCheckConfig {
    pub host: String,
    #[serde(default = "default_interval")]
    pub interval: usize,
    #[serde(default = "default_timeout")]
    pub timeout: usize,
    #[serde(default = "default_ping_count")]
    pub count: usize,
    #[serde(default = "default_packet_size")]
    pub packet_size: usize,
    #[serde(default = "default_max_loss")]
    pub max_loss_percent: f64,
    #[serde(default = "default_max_latency")]
    pub max_avg_latency_ms: f64,
}

fn default_http_method() -> String { "GET".to_string() }
fn default_interval() -> usize { 30 }
fn default_timeout() -> usize { 5 }
fn default_expected_status() -> Vec<u16> { vec![200] }
fn default_true() -> bool { true }
fn default_ssl_warn_days() -> u32 { 30 }
fn default_ping_count() -> usize { 3 }
fn default_packet_size() -> usize { 56 }
fn default_max_loss() -> f64 { 20.0 }
fn default_max_latency() -> f64 { 100.0 }
```

---

## Uptime Calculation

```rust
// src/monitors/uptime.rs

pub struct UptimeCalculator {
    check_history: VecDeque<ServiceCheckResult>,
}

impl UptimeCalculator {
    pub fn calculate_uptime(&self, duration: Duration) -> f64 {
        let cutoff = Utc::now() - duration;

        let total = self.check_history
            .iter()
            .filter(|r| r.timestamp > cutoff)
            .count();

        let successful = self.check_history
            .iter()
            .filter(|r| r.timestamp > cutoff && r.status == ServiceStatus::Up)
            .count();

        if total == 0 {
            return 100.0;
        }

        (successful as f64 / total as f64) * 100.0
    }

    pub fn get_stats(&self, duration: Duration) -> UptimeStats {
        let cutoff = Utc::now() - duration;
        let relevant_checks: Vec<_> = self.check_history
            .iter()
            .filter(|r| r.timestamp > cutoff)
            .collect();

        let response_times: Vec<f64> = relevant_checks
            .iter()
            .filter_map(|r| r.response_time_ms)
            .collect();

        let avg_response_time = if !response_times.is_empty() {
            response_times.iter().sum::<f64>() / response_times.len() as f64
        } else {
            0.0
        };

        let p95 = calculate_percentile(&response_times, 0.95);
        let p99 = calculate_percentile(&response_times, 0.99);

        UptimeStats {
            uptime_percent: self.calculate_uptime(duration),
            total_checks: relevant_checks.len(),
            failed_checks: relevant_checks.iter().filter(|r| r.status == ServiceStatus::Down).count(),
            avg_response_time_ms: avg_response_time,
            p95_response_time_ms: p95,
            p99_response_time_ms: p99,
        }
    }
}

pub struct UptimeStats {
    pub uptime_percent: f64,
    pub total_checks: usize,
    pub failed_checks: usize,
    pub avg_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
}

fn calculate_percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let index = ((sorted.len() as f64) * percentile).floor() as usize;
    sorted[index.min(sorted.len() - 1)]
}
```

---

## Storage Integration

Service check results should be persisted alongside server metrics:

```sql
CREATE TABLE IF NOT EXISTS service_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    status TEXT NOT NULL,  -- 'up', 'down', 'degraded', 'unknown'
    response_time_ms REAL,
    error TEXT,
    metadata TEXT  -- JSON
);

CREATE INDEX idx_service_checks_id_time ON service_checks(service_id, timestamp DESC);
```

---

## Alert Integration

Service status changes should trigger alerts using the existing `AlertActor`:

```rust
// Extend AlertActor to handle ServiceCheckResult
impl AlertActor {
    pub async fn handle_service_check(&mut self, result: ServiceCheckResult) {
        // Look up alert configuration for this service
        let alert_config = self.get_service_alert_config(&result.service_id);

        // Check if status changed
        if self.service_status_changed(&result) {
            self.send_service_alert(&result, alert_config).await;
        }
    }

    fn format_service_alert(&self, result: &ServiceCheckResult) -> String {
        match result.status {
            ServiceStatus::Down => {
                format!(
                    "🔴 **Service Down**: {} is unreachable\n\nError: {}",
                    result.service_id,
                    result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                )
            }
            ServiceStatus::Up => {
                format!(
                    "🟢 **Service Recovered**: {} is back online",
                    result.service_id
                )
            }
            ServiceStatus::Degraded => {
                format!(
                    "🟡 **Service Degraded**: {} is responding slowly",
                    result.service_id
                )
            }
            _ => String::new(),
        }
    }
}
```

---

## Dashboard Integration

Service status will be displayed in the TUI dashboard's Services tab (see DASHBOARD.md).

---

## Testing Strategy

### Unit Tests
- HTTP check with mock responses
- Ping check with simulated packets
- Uptime calculation accuracy
- SSL cert parsing

### Integration Tests
- Real HTTP endpoints (httpbin.org)
- Local test server
- Simulated network failures

### ICMP Requirements
**Important:** ICMP ping requires elevated privileges. Options:

1. **Run as root** (not recommended for production)
2. **Set capabilities:**
   ```bash
   sudo setcap cap_net_raw=eip /path/to/guardia-hub
   ```
3. **Use unprivileged ICMP** (Linux 3.x+):
   ```bash
   sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"
   ```

---

## Performance Considerations

1. **Connection pooling:** Reuse HTTP client
2. **Concurrent checks:** Use tokio tasks for parallel execution
3. **Rate limiting:** Prevent overwhelming target services
4. **Timeout handling:** Ensure checks don't hang indefinitely

---

## Security Considerations

1. **Secrets management:** Store API tokens securely (not in config)
2. **SSL verification:** Default to verifying certificates
3. **Request signing:** Support HMAC/JWT for authenticated health checks
4. **DoS prevention:** Rate limit checks to avoid being flagged

---

## Future Enhancements

- TCP port checks (generic port monitoring)
- DNS resolution checks
- gRPC health checks
- Database connection checks (MySQL, PostgreSQL, Redis)
- Custom script execution (run arbitrary health check commands)
- Multi-region checks (check from different locations)

---

## Success Criteria

- [ ] Check 100+ services every 10 seconds
- [ ] Sub-50ms overhead per check
- [ ] <0.1% false positive rate
- [ ] Uptime calculations are accurate
- [ ] SSL warnings 30 days before expiry
- [ ] Graceful handling of network failures
