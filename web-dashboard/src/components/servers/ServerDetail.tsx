import { ServerInfo } from '../../api/types'
import CpuChart from './CpuChart'
import TemperatureChart from './TemperatureChart'
import { Cpu, HardDrive, Thermometer, Info } from 'lucide-react'

interface ServerDetailProps {
  server: ServerInfo
}

export default function ServerDetail({ server }: ServerDetailProps) {
  const metrics = server.latest_metrics

  const getMemoryColor = (percentage: number) => {
    if (percentage >= 85) return { from: '#ef4444', to: '#dc2626', glow: 'rgba(239, 68, 68, 0.3)' }
    if (percentage >= 70) return { from: '#f97316', to: '#ea580c', glow: 'rgba(249, 115, 22, 0.3)' }
    return { from: '#10b981', to: '#059669', glow: 'rgba(16, 185, 129, 0.3)' }
  }

  const getCpuColor = (usage: number) => {
    if (usage >= 80) return { from: '#ef4444', to: '#dc2626' }
    if (usage >= 60) return { from: '#f59e0b', to: '#d97706' }
    return { from: '#3b82f6', to: '#2563eb' }
  }

  const getTempColor = (temp: number) => {
    if (temp >= 80) return 'text-red-400'
    if (temp >= 60) return 'text-orange-400'
    if (temp >= 40) return 'text-yellow-400'
    return 'text-green-400'
  }

  return (
    <div className="space-y-6">
      {/* System Info Card */}
      <div className="card-premium">
        <div className="flex items-center gap-3 mb-4">
          <div className="rounded-lg bg-gradient-to-br from-indigo-500/20 to-purple-500/20 p-2.5 border border-indigo-500/30">
            <Info className="h-5 w-5 text-indigo-400" />
          </div>
          <h3 className="text-xl font-bold bg-gradient-to-r from-white to-gray-300 bg-clip-text text-transparent">
            System Information
          </h3>
        </div>
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div className="stat-card group">
            <span className="text-gray-400 text-xs uppercase tracking-wider font-semibold mb-1 block">Hostname</span>
            <p className="text-white font-bold text-base group-hover:text-indigo-400 transition-colors">
              {metrics?.system?.host_name || 'N/A'}
            </p>
          </div>
          <div className="stat-card group">
            <span className="text-gray-400 text-xs uppercase tracking-wider font-semibold mb-1 block">Architecture</span>
            <p className="text-white font-bold text-base group-hover:text-indigo-400 transition-colors">
              {metrics?.cpus?.arch || 'N/A'}
            </p>
          </div>
          <div className="stat-card group">
            <span className="text-gray-400 text-xs uppercase tracking-wider font-semibold mb-1 block">Operating System</span>
            <p className="text-white font-bold text-base group-hover:text-indigo-400 transition-colors">
              {metrics?.system?.os_version || 'N/A'}
            </p>
          </div>
          <div className="stat-card group">
            <span className="text-gray-400 text-xs uppercase tracking-wider font-semibold mb-1 block">Kernel Version</span>
            <p className="text-white font-bold text-base group-hover:text-indigo-400 transition-colors">
              {metrics?.system?.kernel_version || 'N/A'}
            </p>
          </div>
        </div>
      </div>

      {/* CPU Info Card */}
      {metrics && (
        <div className="card-premium">
          <div className="flex items-center gap-3 mb-5">
            <div className="rounded-lg bg-gradient-to-br from-blue-500/20 to-cyan-500/20 p-2.5 border border-blue-500/30">
              <Cpu className="h-5 w-5 text-blue-400" />
            </div>
            <h3 className="text-xl font-bold bg-gradient-to-r from-white to-gray-300 bg-clip-text text-transparent">
              CPU Cores
            </h3>
            <span className="ml-auto text-xs font-semibold text-gray-400 bg-gray-800/50 px-3 py-1.5 rounded-lg">
              {metrics.cpus.cpus.length} cores
            </span>
          </div>
          <div className="space-y-3">
            {metrics.cpus.cpus.map((cpu, idx) => {
              const colors = getCpuColor(cpu.usage)
              return (
                <div key={idx} className="group">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm text-gray-300 font-medium group-hover:text-white transition-colors">
                      {cpu.name}
                    </span>
                    <span className="text-sm font-bold text-white bg-gray-800/50 px-3 py-1 rounded-lg group-hover:bg-gray-700/70 transition-all">
                      {cpu.usage.toFixed(1)}%
                    </span>
                  </div>
                  <div className="progress-bar">
                    <div
                      className="progress-fill"
                      style={{
                        width: `${cpu.usage}%`,
                        '--progress-from': colors.from,
                        '--progress-to': colors.to,
                        '--progress-glow': colors.to,
                      } as any}
                    />
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      )}

      {/* Memory Info Card */}
      {metrics && (
        <div className="card-premium">
          <div className="flex items-center gap-3 mb-5">
            <div className="rounded-lg bg-gradient-to-br from-emerald-500/20 to-teal-500/20 p-2.5 border border-emerald-500/30">
              <HardDrive className="h-5 w-5 text-emerald-400" />
            </div>
            <h3 className="text-xl font-bold bg-gradient-to-r from-white to-gray-300 bg-clip-text text-transparent">
              Memory Usage
            </h3>
          </div>
          <div className="space-y-5">
            {/* RAM */}
            <div className="group">
              <div className="flex justify-between items-center mb-2">
                <span className="text-sm font-semibold text-gray-300 group-hover:text-white transition-colors">
                  RAM
                </span>
                <div className="flex items-baseline gap-2">
                  <span className="text-base font-bold text-white">
                    {(metrics.memory.used / 1024 / 1024 / 1024).toFixed(2)} GB
                  </span>
                  <span className="text-xs text-gray-400">
                    / {(metrics.memory.total / 1024 / 1024 / 1024).toFixed(2)} GB
                  </span>
                  <span className="text-xs font-bold text-indigo-400 bg-indigo-500/10 px-2 py-1 rounded ml-2">
                    {((metrics.memory.used / metrics.memory.total) * 100).toFixed(1)}%
                  </span>
                </div>
              </div>
              <div className="progress-bar h-4">
                {(() => {
                  const percentage = (metrics.memory.used / metrics.memory.total) * 100
                  const colors = getMemoryColor(percentage)
                  return (
                    <div
                      className="progress-fill"
                      style={{
                        width: `${percentage}%`,
                        '--progress-from': colors.from,
                        '--progress-to': colors.to,
                        '--progress-glow': colors.glow,
                      } as any}
                    />
                  )
                })()}
              </div>
            </div>
            
            {/* Swap */}
            <div className="group">
              <div className="flex justify-between items-center mb-2">
                <span className="text-sm font-semibold text-gray-300 group-hover:text-white transition-colors">
                  Swap
                </span>
                <div className="flex items-baseline gap-2">
                  <span className="text-base font-bold text-white">
                    {(metrics.memory.used_swap / 1024 / 1024 / 1024).toFixed(2)} GB
                  </span>
                  <span className="text-xs text-gray-400">
                    / {(metrics.memory.total_swap / 1024 / 1024 / 1024).toFixed(2)} GB
                  </span>
                  <span className="text-xs font-bold text-orange-400 bg-orange-500/10 px-2 py-1 rounded ml-2">
                    {((metrics.memory.used_swap / metrics.memory.total_swap) * 100).toFixed(1)}%
                  </span>
                </div>
              </div>
              <div className="progress-bar h-4">
                {(() => {
                  const percentage = (metrics.memory.used_swap / metrics.memory.total_swap) * 100
                  const colors = getMemoryColor(percentage)
                  return (
                    <div
                      className="progress-fill"
                      style={{
                        width: `${percentage}%`,
                        '--progress-from': colors.from,
                        '--progress-to': colors.to,
                        '--progress-glow': colors.glow,
                      } as any}
                    />
                  )
                })()}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Temperature Info Card */}
      {metrics && metrics.components.components.length > 0 && (
        <div className="card-premium">
          <div className="flex items-center gap-3 mb-5">
            <div className="rounded-lg bg-gradient-to-br from-orange-500/20 to-red-500/20 p-2.5 border border-orange-500/30">
              <Thermometer className="h-5 w-5 text-orange-400" />
            </div>
            <h3 className="text-xl font-bold bg-gradient-to-r from-white to-gray-300 bg-clip-text text-transparent">
              Temperature Sensors
            </h3>
          </div>
          <div className="grid grid-cols-2 gap-3">
            {metrics.components.components.map((component, idx) => (
              <div key={idx} className="stat-card group">
                <div className="flex justify-between items-start">
                  <span className="text-xs text-gray-400 uppercase tracking-wider font-semibold">
                    {component.name}
                  </span>
                  <span className={`text-lg font-bold ${getTempColor(component.temperature || 0)} group-hover:scale-110 transition-transform`}>
                    {component.temperature ? `${component.temperature.toFixed(1)}°C` : 'N/A'}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Charts */}
      {metrics && (
        <div className="space-y-6">
          {metrics.cpus?.cpus && (
            <div className="animate-fade-in" style={{ animationDelay: '100ms' }}>
              <CpuChart serverId={server.server_id} />
            </div>
          )}
          {metrics.components?.components && (
            <div className="animate-fade-in" style={{ animationDelay: '200ms' }}>
              <TemperatureChart serverId={server.server_id} />
            </div>
          )}
        </div>
      )}
    </div>
  )
}
