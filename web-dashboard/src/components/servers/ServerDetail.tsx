import { useEffect } from 'react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import { apiClient } from '../../api/client'
import type { ServerInfo } from '../../api/types'
import CpuChart from './CpuChart'
import TemperatureChart from './TemperatureChart'

interface ServerDetailProps {
  server: ServerInfo
}

export default function ServerDetail({ server }: ServerDetailProps) {
  const { setMetricsHistory } = useMonitoringStore()

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        const response = await apiClient.getLatestMetrics(server.server_id, 150)
        const points = response.metrics.map(m => ({
          timestamp: new Date(m.timestamp),
          server_id: server.server_id,
          data: m.data,
        }))
        setMetricsHistory(server.server_id, points)
      } catch (err) {
        console.error('Failed to fetch metrics:', err)
      }
    }

    fetchMetrics()
  }, [server.server_id, setMetricsHistory])

  const metrics = server.latest_metrics

  return (
    <div className="space-y-4">
      {/* System Info */}
      <div className="card">
        <h3 className="text-lg font-semibold text-white mb-3">System Information</h3>
        <div className="grid grid-cols-2 gap-3 text-sm">
          <div>
            <span className="text-gray-400">Hostname:</span>
            <p className="text-white font-medium">{metrics?.system?.host_name || 'N/A'}</p>
          </div>
          <div>
            <span className="text-gray-400">Architecture:</span>
            <p className="text-white font-medium">{metrics?.cpus?.arch || 'N/A'}</p>
          </div>
          <div>
            <span className="text-gray-400">OS:</span>
            <p className="text-white font-medium">{metrics?.system?.os_version || 'N/A'}</p>
          </div>
          <div>
            <span className="text-gray-400">Kernel:</span>
            <p className="text-white font-medium">{metrics?.system?.kernel_version || 'N/A'}</p>
          </div>
        </div>
      </div>

      {/* CPU Info */}
      {metrics && (
        <div className="card">
          <h3 className="text-lg font-semibold text-white mb-3">CPU</h3>
          <div className="space-y-2">
            {metrics.cpus.cpus.map((cpu, idx) => (
              <div key={idx} className="flex items-center justify-between">
                <span className="text-sm text-gray-400">{cpu.name}</span>
                <div className="flex items-center gap-2">
                  <div className="w-32 h-2 rounded-full bg-gray-700 overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-blue-500 to-cyan-500"
                      style={{ width: `${cpu.usage}%` }}
                    />
                  </div>
                  <span className="text-sm font-medium text-white w-12 text-right">
                    {cpu.usage.toFixed(1)}%
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Memory Info */}
      {metrics && (
        <div className="card">
          <h3 className="text-lg font-semibold text-white mb-3">Memory</h3>
          <div className="space-y-3">
            <div>
              <div className="flex justify-between text-sm mb-1">
                <span className="text-gray-400">RAM</span>
                <span className="text-white font-medium">
                  {(metrics.memory.used / 1024 / 1024 / 1024).toFixed(1)}GB / {(metrics.memory.total / 1024 / 1024 / 1024).toFixed(1)}GB
                </span>
              </div>
              <div className="w-full h-2 rounded-full bg-gray-700 overflow-hidden">
                <div
                  className="h-full bg-green-500"
                  style={{
                    width: `${(metrics.memory.used / metrics.memory.total) * 100}%`,
                  }}
                />
              </div>
            </div>
            <div>
              <div className="flex justify-between text-sm mb-1">
                <span className="text-gray-400">Swap</span>
                <span className="text-white font-medium">
                  {(metrics.memory.used_swap / 1024 / 1024 / 1024).toFixed(1)}GB / {(metrics.memory.total_swap / 1024 / 1024 / 1024).toFixed(1)}GB
                </span>
              </div>
              <div className="w-full h-2 rounded-full bg-gray-700 overflow-hidden">
                <div
                  className="h-full bg-orange-500"
                  style={{
                    width: `${(metrics.memory.used_swap / metrics.memory.total_swap) * 100}%`,
                  }}
                />
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Temperature Info */}
      {metrics && metrics.components.components.length > 0 && (
        <div className="card">
          <h3 className="text-lg font-semibold text-white mb-3">Temperatures</h3>
          <div className="space-y-2">
            {metrics.components.components.map((component, idx) => (
              <div key={idx} className="flex justify-between text-sm">
                <span className="text-gray-400">{component.name}</span>
                <span className="text-white font-medium">
                  {component.temperature ? `${component.temperature.toFixed(1)}°C` : 'N/A'}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Charts - only show if we have metrics data */}
      {metrics && (
        <div className="space-y-4">
          {metrics.cpus?.cpus && <CpuChart serverId={server.server_id} />}
          {metrics.components?.components && <TemperatureChart serverId={server.server_id} />}
        </div>
      )}
    </div>
  )
}
