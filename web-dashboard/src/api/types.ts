/**
 * API Types - TypeScript mirrors of Rust API types
 * These keep the web dashboard in sync with the backend API
 */

// Status enums
export enum MonitoringStatus {
  Active = 'active',
  Paused = 'paused',
  Disabled = 'disabled',
}

export enum ServerHealthStatus {
  Up = 'up',
  Down = 'down',
  Stale = 'stale',
  Unknown = 'unknown',
}

export enum ServiceHealthStatus {
  Up = 'up',
  Down = 'down',
  Degraded = 'degraded',
  Stale = 'stale',
  Unknown = 'unknown',
}

// CPU info
export interface CpuInfo {
  usage: number
}

export interface CpuMetrics {
  cpus: CpuInfo[]
}

// Memory info
export interface MemoryMetrics {
  total: number
  used: number
  available: number
  swap_total: number
  swap_used: number
}

// Temperature info
export interface TemperatureMetrics {
  temperatures: Array<{
    name: string
    current: number
  }>
}

// Main metrics struct
export interface ServerMetrics {
  cpus: CpuMetrics
  memory: MemoryMetrics
  temperatures: TemperatureMetrics
  hostname: string
  os: string
  arch: string
  kernel_version: string
}

// Server info
export interface ServerInfo {
  server_id: string
  display_name: string
  monitoring_status: MonitoringStatus
  health_status: ServerHealthStatus
  last_update: string // ISO 8601 timestamp
  latest_metrics: ServerMetrics | null
}

// Service info
export interface ServiceInfo {
  name: string
  url: string
  monitoring_status: MonitoringStatus
  health_status: ServiceHealthStatus
  last_check: string | null // ISO 8601 timestamp
  response_time_ms: number | null
}

// API Responses
export interface HealthResponse {
  status: string
  timestamp: string
}

export interface StatsResponse {
  servers_monitored: number
  services_monitored: number
  active_alerts: number
  storage_metrics: {
    last_cleanup_time: string | null
    total_metrics: number
    total_service_checks: number
  }
}

export interface ServersResponse {
  servers: ServerInfo[]
}

export interface ServicesResponse {
  services: ServiceInfo[]
}

export interface MetricsResponse {
  server_id: string
  metrics: Array<{
    timestamp: string // ISO 8601
    data: ServerMetrics
  }>
}

export interface LatestMetricsResponse {
  server_id: string
  metrics: Array<{
    timestamp: string // ISO 8601
    data: ServerMetrics
  }>
}

export interface ServiceChecksResponse {
  service_name: string
  checks: Array<{
    timestamp: string // ISO 8601
    status: ServiceHealthStatus
    response_time_ms: number | null
  }>
}

export interface UptimeResponse {
  service_name: string
  uptime_percentage: number
  total_checks: number
  successful_checks: number
}

// WebSocket events
export interface MetricEvent {
  type: 'metric'
  server_id: string
  timestamp: string // ISO 8601
  metrics: ServerMetrics
}

export interface ServiceCheckEvent {
  type: 'service_check'
  service_name: string
  timestamp: string // ISO 8601
  status: ServiceHealthStatus
  response_time_ms: number | null
}

export type WsEvent = MetricEvent | ServiceCheckEvent
