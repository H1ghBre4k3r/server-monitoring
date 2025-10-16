import { useEffect, useState } from 'react'
import { useMonitoringStore } from '../stores/monitoringStore'
import { apiClient } from '../api/client'

export function useMonitoring(refreshInterval: number = 5000) {
  const { setServers, setServices, setMetricsHistory } = useMonitoringStore()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchData = async () => {
      try {
        setError(null)

        // Fetch servers
        const serversResponse = await apiClient.getServers()
        setServers(serversResponse.servers)

        // Fetch services
        const servicesResponse = await apiClient.getServices()
        setServices(servicesResponse.services)

        // Fetch initial metrics for each server
        for (const server of serversResponse.servers) {
          try {
            const metricsResponse = await apiClient.getLatestMetrics(server.server_id, 150)
            const points = metricsResponse.metrics.map(m => ({
              timestamp: new Date(m.timestamp),
              server_id: server.server_id,
              data: m.metadata, // Note: data is in the 'metadata' field
            }))
            setMetricsHistory(server.server_id, points)
          } catch (err) {
            console.warn(`Failed to fetch metrics for ${server.server_id}:`, err)
          }
        }

        setLoading(false)
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Failed to fetch monitoring data'
        setError(message)
        setLoading(false)
      }
    }

    fetchData()
    const interval = setInterval(fetchData, refreshInterval)

    return () => clearInterval(interval)
  }, [refreshInterval, setServers, setServices, setMetricsHistory])

  return { loading, error }
}
