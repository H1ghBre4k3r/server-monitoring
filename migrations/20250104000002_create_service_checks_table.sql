-- Service health check results table
-- Stores historical service check data for uptime tracking and analysis

CREATE TABLE IF NOT EXISTS service_checks (
    service_name TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    url TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('up', 'down', 'degraded')),
    response_time_ms INTEGER,
    http_status_code INTEGER,
    error_message TEXT,
    PRIMARY KEY (service_name, timestamp)
) STRICT;

-- Index for time-based queries (uptime calculations, retention cleanup)
CREATE INDEX IF NOT EXISTS idx_service_checks_timestamp
    ON service_checks(timestamp);

-- Index for status-based queries (downtime analysis)
CREATE INDEX IF NOT EXISTS idx_service_checks_status
    ON service_checks(status);

-- Composite index for uptime calculations (service + time range)
CREATE INDEX IF NOT EXISTS idx_service_checks_uptime
    ON service_checks(service_name, timestamp, status);
