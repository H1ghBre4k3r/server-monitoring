-- Create metrics table for storing server monitoring data
--
-- This table uses a hybrid schema:
-- - Aggregate metrics (cpu_avg, memory_used, etc.) as columns for fast queries
-- - Detailed metrics (per-core CPU, per-component temps) as JSON for flexibility
--
-- Performance optimizations:
-- - Primary key on (server_id, timestamp) for efficient range scans
-- - Index on timestamp for time-based queries
-- - STRICT mode for better type safety

CREATE TABLE IF NOT EXISTS metrics (
    -- Primary identification
    server_id TEXT NOT NULL,              -- Server identifier (e.g., "192.168.1.100:3000")
    timestamp INTEGER NOT NULL,            -- Unix timestamp in milliseconds (UTC)
    display_name TEXT NOT NULL,            -- Human-readable server name

    -- Metric type and categorization
    metric_type TEXT NOT NULL DEFAULT 'resource',  -- 'resource', 'system', or 'custom'

    -- Aggregate metrics (frequently queried, indexed)
    cpu_avg REAL,                          -- Average CPU usage (0-100%)
    memory_used INTEGER,                   -- Memory used (bytes)
    memory_total INTEGER,                  -- Total memory (bytes)
    temp_avg REAL,                         -- Average temperature (Celsius)

    -- Detailed metrics (JSON for flexibility)
    metadata TEXT NOT NULL,                -- JSON blob with per-core, per-component data

    -- Primary key for efficient queries by server and time
    PRIMARY KEY (server_id, timestamp)
) STRICT;

-- Index for time-based queries (e.g., "show last hour")
CREATE INDEX IF NOT EXISTS idx_metrics_timestamp
ON metrics(timestamp);

-- Index for metric type filtering (e.g., retention policies per type)
CREATE INDEX IF NOT EXISTS idx_metrics_type
ON metrics(metric_type);

-- Composite index for common query pattern: server + time range
-- Note: Not needed because PRIMARY KEY already provides this
-- CREATE INDEX idx_metrics_server_time ON metrics(server_id, timestamp);
