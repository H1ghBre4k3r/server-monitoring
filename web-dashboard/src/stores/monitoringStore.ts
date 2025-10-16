/**
 * Monitoring Store - Zustand state management for monitoring data
 */

import { create } from 'zustand'
import type { ServerInfo, ServiceInfo, MetricEvent } from '../api/types'

export interface AlertEntry {
  id: string
  timestamp: Date
  type: 'metric' | 'service' | 'info'
  severity: 'critical' | 'warning' | 'info'
  title: string
  message: string
}

export interface MetricPoint {
  timestamp: Date
  server_id: string
  data: MetricEvent['metrics']
}

interface MonitoringState {
  // Connection state
  isConnected: boolean
  setConnected: (connected: boolean) => void

  // Servers
  servers: ServerInfo[]
  selectedServerId: string | null
  setServers: (servers: ServerInfo[]) => void
  setSelectedServer: (id: string | null) => void

  // Services
  services: ServiceInfo[]
  setServices: (services: ServiceInfo[]) => void

  // Metrics history (per server, max 1000 per server)
  metricsHistory: Map<string, MetricPoint[]>
  addMetricPoint: (serverId: string, point: MetricPoint) => void
  setMetricsHistory: (serverId: string, points: MetricPoint[]) => void
  clearMetricsHistory: (serverId: string) => void

  // Alerts (max 500 total)
  alerts: AlertEntry[]
  addAlert: (alert: AlertEntry) => void
  clearAlerts: () => void

  // Settings
  timeWindowSeconds: number
  setTimeWindow: (seconds: number) => void
  apiToken?: string
  setApiToken: (token: string) => void
}

const MAX_METRICS_PER_SERVER = 1000
const MAX_ALERTS = 500

export const useMonitoringStore = create<MonitoringState>((set) => ({
  // Connection state
  isConnected: false,
  setConnected: (connected) => set({ isConnected: connected }),

  // Servers
  servers: [],
  selectedServerId: null,
  setServers: (servers) => set({ servers }),
  setSelectedServer: (id) => set({ selectedServerId: id }),

  // Services
  services: [],
  setServices: (services) => set({ services }),

  // Metrics history
  metricsHistory: new Map(),
  addMetricPoint: (serverId, point) =>
    set((state) => {
      const history = state.metricsHistory.get(serverId) || []
      const updated = [...history, point]

      // Time-based cleanup: remove metrics older than 2x time window
      const cutoffTime = new Date(Date.now() - state.timeWindowSeconds * 2000)
      const filtered = updated.filter(m => m.timestamp > cutoffTime)

      // Also enforce maximum limit
      const final = filtered.length > MAX_METRICS_PER_SERVER 
        ? filtered.slice(-MAX_METRICS_PER_SERVER)
        : filtered

      const newMap = new Map(state.metricsHistory)
      newMap.set(serverId, final)
      return { metricsHistory: newMap }
    }),
  setMetricsHistory: (serverId, points) =>
    set((state) => {
      const newMap = new Map(state.metricsHistory)
      newMap.set(serverId, points)
      return { metricsHistory: newMap }
    }),
  clearMetricsHistory: (serverId) =>
    set((state) => {
      const newMap = new Map(state.metricsHistory)
      newMap.delete(serverId)
      return { metricsHistory: newMap }
    }),

  // Alerts
  alerts: [],
  addAlert: (alert) =>
    set((state) => {
      const updated = [alert, ...state.alerts]
      // Keep only MAX_ALERTS most recent alerts
      if (updated.length > MAX_ALERTS) {
        updated.pop()
      }
      return { alerts: updated }
    }),
  clearAlerts: () => set({ alerts: [] }),

  // Settings
  timeWindowSeconds: 300, // 5 minutes default
  setTimeWindow: (seconds) => set({ timeWindowSeconds: seconds }),
  apiToken: undefined,
  setApiToken: (token) => set({ apiToken: token }),
}))
