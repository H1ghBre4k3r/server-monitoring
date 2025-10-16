import { ServerInfo } from '../../api/types'
import CpuChart from './CpuChart'
import TemperatureChart from './TemperatureChart'
import CircularProgress from './CircularProgress'
import { HardDrive, Thermometer, Info, Monitor, Cpu as CpuIcon, Box, Terminal, Waves } from 'lucide-react'

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

  const getTempColor = (temp: number) => {
    if (temp >= 80) return 'text-red-400'
    if (temp >= 60) return 'text-orange-400'
    if (temp >= 40) return 'text-yellow-400'
    return 'text-green-400'
  }

  const getTempBgColor = (temp: number) => {
    if (temp >= 80) return 'bg-red-500/10 border-red-500/30'
    if (temp >= 60) return 'bg-orange-500/10 border-orange-500/30'
    if (temp >= 40) return 'bg-yellow-500/10 border-yellow-500/30'
    return 'bg-green-500/10 border-green-500/30'
  }

  return (
    <div className="space-y-6">
      {/* Elegant System Info Banner */}
      <div className="card-premium relative overflow-hidden">
        {/* Decorative wave pattern */}
        <div className="absolute top-0 right-0 opacity-5">
          <Waves className="h-32 w-32 text-indigo-400" />
        </div>
        
        <div className="relative z-10">
          <div className="flex items-center gap-3 mb-6">
            <div className="rounded-xl bg-gradient-to-br from-indigo-500/30 to-purple-500/30 p-3 border border-indigo-500/40 shadow-lg shadow-indigo-500/20">
              <Info className="h-6 w-6 text-indigo-300" />
            </div>
            <div>
              <h3 className="text-2xl font-bold bg-gradient-to-r from-indigo-400 via-purple-400 to-pink-400 bg-clip-text text-transparent">
                System Overview
              </h3>
              <p className="text-xs text-gray-400 font-medium tracking-wide">Server Configuration & Details</p>
            </div>
          </div>
          
          <div className="grid grid-cols-4 gap-3">
            <div className="stat-card group hover:border-indigo-500/40">
              <div className="flex items-center gap-2 mb-2">
                <div className="p-1.5 rounded-lg bg-indigo-500/10">
                  <Monitor className="h-3.5 w-3.5 text-indigo-400" />
                </div>
                <span className="text-gray-400 text-xs uppercase tracking-wider font-semibold">Hostname</span>
              </div>
              <p className="text-white font-bold text-sm group-hover:text-indigo-400 transition-colors truncate" title={metrics?.system?.host_name || undefined}>
                {metrics?.system?.host_name || 'N/A'}
              </p>
            </div>
            
            <div className="stat-card group hover:border-purple-500/40">
              <div className="flex items-center gap-2 mb-2">
                <div className="p-1.5 rounded-lg bg-purple-500/10">
                  <CpuIcon className="h-3.5 w-3.5 text-purple-400" />
                </div>
                <span className="text-gray-400 text-xs uppercase tracking-wider font-semibold">Architecture</span>
              </div>
              <p className="text-white font-bold text-sm group-hover:text-purple-400 transition-colors">
                {metrics?.cpus?.arch || 'N/A'}
              </p>
            </div>
            
            <div className="stat-card group hover:border-cyan-500/40">
              <div className="flex items-center gap-2 mb-2">
                <div className="p-1.5 rounded-lg bg-cyan-500/10">
                  <Box className="h-3.5 w-3.5 text-cyan-400" />
                </div>
                <span className="text-gray-400 text-xs uppercase tracking-wider font-semibold">OS</span>
              </div>
              <p className="text-white font-bold text-sm group-hover:text-cyan-400 transition-colors truncate" title={metrics?.system?.os_version || undefined}>
                {metrics?.system?.os_version || 'N/A'}
              </p>
            </div>
            
            <div className="stat-card group hover:border-pink-500/40">
              <div className="flex items-center gap-2 mb-2">
                <div className="p-1.5 rounded-lg bg-pink-500/10">
                  <Terminal className="h-3.5 w-3.5 text-pink-400" />
                </div>
                <span className="text-gray-400 text-xs uppercase tracking-wider font-semibold">Kernel</span>
              </div>
              <p className="text-white font-bold text-sm group-hover:text-pink-400 transition-colors truncate" title={metrics?.system?.kernel_version || undefined}>
                {metrics?.system?.kernel_version || 'N/A'}
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* CPU Section */}
      {metrics && metrics.cpus?.cpus && (
        <div className="animate-fade-in" style={{ animationDelay: '50ms' }}>
          <CpuChart serverId={server.server_id} currentMetrics={metrics} />
        </div>
      )}

      {/* Memory Section */}
      {metrics && (
        <div className="card-premium animate-fade-in" style={{ animationDelay: '100ms' }}>
          <div className="flex items-center justify-between mb-6">
            <div className="flex items-center gap-3">
              <div className="rounded-xl bg-gradient-to-br from-emerald-500/30 to-teal-500/30 p-3 border border-emerald-500/40 shadow-lg shadow-emerald-500/20">
                <HardDrive className="h-6 w-6 text-emerald-300" />
              </div>
              <div>
                <h3 className="text-2xl font-bold bg-gradient-to-r from-emerald-400 via-teal-400 to-cyan-400 bg-clip-text text-transparent">
                  Memory Resources
                </h3>
                <p className="text-xs text-gray-400 font-medium tracking-wide">RAM & Swap Allocation</p>
              </div>
            </div>
            
            {/* Memory efficiency badge */}
            <div className="px-4 py-2 rounded-xl bg-gradient-to-r from-emerald-500/10 to-teal-500/10 border border-emerald-500/30">
              <div className="text-xs text-gray-400 font-semibold uppercase tracking-wider">Total Memory</div>
              <div className="text-lg font-bold text-emerald-400">
                {((metrics.memory.total + metrics.memory.total_swap) / 1024 / 1024 / 1024).toFixed(1)} GB
              </div>
            </div>
          </div>
          
          {/* Circular gauges */}
          <div className="grid grid-cols-2 gap-6 mb-6">
            <div className="flex flex-col items-center justify-center p-6 rounded-2xl bg-gradient-to-br from-gray-800/40 to-gray-900/40 border border-gray-700/40 hover:border-emerald-500/30 transition-all group">
              <CircularProgress
                percentage={(metrics.memory.used / metrics.memory.total) * 100}
                color={getMemoryColor((metrics.memory.used / metrics.memory.total) * 100)}
                label="RAM"
                value={`${(metrics.memory.used / 1024 / 1024 / 1024).toFixed(2)} / ${(metrics.memory.total / 1024 / 1024 / 1024).toFixed(2)} GB`}
                size={180}
                strokeWidth={14}
              />
            </div>
            
            <div className="flex flex-col items-center justify-center p-6 rounded-2xl bg-gradient-to-br from-gray-800/40 to-gray-900/40 border border-gray-700/40 hover:border-teal-500/30 transition-all group">
              <CircularProgress
                percentage={(metrics.memory.used_swap / metrics.memory.total_swap) * 100}
                color={getMemoryColor((metrics.memory.used_swap / metrics.memory.total_swap) * 100)}
                label="SWAP"
                value={`${(metrics.memory.used_swap / 1024 / 1024 / 1024).toFixed(2)} / ${(metrics.memory.total_swap / 1024 / 1024 / 1024).toFixed(2)} GB`}
                size={180}
                strokeWidth={14}
              />
            </div>
          </div>
          
          {/* Detailed linear bars */}
          <div className="space-y-4 pt-6 border-t border-gray-800/50">
            <div className="group">
              <div className="flex justify-between items-center mb-2">
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse-glow"></div>
                  <span className="text-sm font-semibold text-gray-300 group-hover:text-white transition-colors">
                    RAM Utilization
                  </span>
                </div>
                <span className="text-sm font-bold text-emerald-400 bg-emerald-500/10 px-3 py-1 rounded-lg">
                  {((metrics.memory.used / metrics.memory.total) * 100).toFixed(1)}%
                </span>
              </div>
              <div className="progress-bar h-3">
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
            
            <div className="group">
              <div className="flex justify-between items-center mb-2">
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-teal-500 animate-pulse-glow" style={{ animationDelay: '0.5s' }}></div>
                  <span className="text-sm font-semibold text-gray-300 group-hover:text-white transition-colors">
                    Swap Utilization
                  </span>
                </div>
                <span className="text-sm font-bold text-teal-400 bg-teal-500/10 px-3 py-1 rounded-lg">
                  {((metrics.memory.used_swap / metrics.memory.total_swap) * 100).toFixed(1)}%
                </span>
              </div>
              <div className="progress-bar h-3">
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

      {/* Combined Temperature Section with Chart and Sensors */}
      {metrics && metrics.components.components.length > 0 && (
        <div className="card-premium animate-fade-in" style={{ animationDelay: '150ms' }}>
          <div className="flex items-center gap-3 mb-6">
            <div className="rounded-xl bg-gradient-to-br from-orange-500/30 to-red-500/30 p-3 border border-orange-500/40 shadow-lg shadow-orange-500/20">
              <Thermometer className="h-6 w-6 text-orange-300" />
            </div>
            <div>
              <h3 className="text-2xl font-bold bg-gradient-to-r from-orange-400 via-red-400 to-pink-400 bg-clip-text text-transparent">
                Temperature Monitoring
              </h3>
              <p className="text-xs text-gray-400 font-medium tracking-wide">Real-time Thermal Sensors & History</p>
            </div>
          </div>
          
          {/* Current temperature sensors in elegant cards */}
          <div className="mb-6">
            <h4 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center gap-2">
              <div className="w-1 h-4 bg-gradient-to-b from-orange-500 to-red-500 rounded-full"></div>
              Current Readings
            </h4>
            <div className="grid grid-cols-3 gap-3">
              {metrics.components.components.map((component, idx) => (
                <div 
                  key={idx} 
                  className={`stat-card group hover:scale-105 transition-all duration-300 ${getTempBgColor(component.temperature || 0)} border`}
                >
                  <div className="flex justify-between items-start">
                    <div className="flex-1">
                      <div className="flex items-center gap-1 mb-1">
                        <div className={`w-1.5 h-1.5 rounded-full ${
                          component.temperature && component.temperature >= 80 ? 'bg-red-500' :
                          component.temperature && component.temperature >= 60 ? 'bg-orange-500' :
                          component.temperature && component.temperature >= 40 ? 'bg-yellow-500' :
                          'bg-green-500'
                        } animate-pulse-glow`}></div>
                        <span className="text-xs text-gray-400 uppercase tracking-wider font-semibold truncate">
                          {component.name}
                        </span>
                      </div>
                      <span className={`text-2xl font-bold ${getTempColor(component.temperature || 0)} group-hover:scale-110 transition-transform inline-block`}>
                        {component.temperature ? `${component.temperature.toFixed(1)}°C` : 'N/A'}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
          
          {/* Temperature chart */}
          <div className="pt-6 border-t border-gray-800/50">
            <h4 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4 flex items-center gap-2">
              <div className="w-1 h-4 bg-gradient-to-b from-orange-500 to-red-500 rounded-full"></div>
              Historical Trends
            </h4>
            <TemperatureChart serverId={server.server_id} currentMetrics={metrics} />
          </div>
        </div>
      )}
    </div>
  )
}
