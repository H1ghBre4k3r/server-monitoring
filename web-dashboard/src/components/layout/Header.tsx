import { Activity, AlertCircle, CheckCircle2, Zap, Clock } from 'lucide-react'
import { useMonitoringStore } from '../../stores/monitoringStore'

interface HeaderProps {
  isConnected: boolean
  onRefresh?: () => void
}

export default function Header({ isConnected, onRefresh }: HeaderProps) {
  const { timeWindowSeconds, setTimeWindow } = useMonitoringStore()
  const statusColor = isConnected ? 'text-green-400' : 'text-red-400'
  const statusBg = isConnected ? 'bg-green-500/10' : 'bg-red-500/10'
  const statusBorder = isConnected ? 'border-green-500/30' : 'border-red-500/30'
  const StatusIcon = isConnected ? CheckCircle2 : AlertCircle

  const timeWindowOptions = [
    { label: '1m', seconds: 60 },
    { label: '5m', seconds: 300 },
    { label: '15m', seconds: 900 },
    { label: '30m', seconds: 1800 },
    { label: '1h', seconds: 3600 },
  ]

  return (
    <header className="border-b border-gray-800/50 bg-gradient-to-r from-gray-900/95 via-gray-900/90 to-gray-900/95 px-6 py-4 shadow-2xl backdrop-blur-xl">
      <div className="flex items-center justify-between">
        {/* Logo and title with gradient effect */}
        <div className="flex items-center gap-4">
          <div className="relative">
            <div className="absolute inset-0 rounded-xl bg-gradient-to-br from-blue-600 to-purple-600 opacity-75 blur-lg animate-pulse-glow"></div>
            <div className="relative rounded-xl bg-gradient-to-br from-blue-500 via-indigo-500 to-purple-600 p-2.5 shadow-lg">
              <Zap className="h-6 w-6 text-white" strokeWidth={2.5} />
            </div>
          </div>
          <div>
            <h1 className="text-2xl font-bold bg-gradient-to-r from-blue-400 via-indigo-400 to-purple-400 bg-clip-text text-transparent">
              Guardia
            </h1>
            <p className="text-xs text-gray-400 font-medium tracking-wide">
              Server Monitoring Dashboard
            </p>
          </div>
        </div>

        {/* Connection status and controls */}
        <div className="flex items-center gap-3">
          {/* Time window selector */}
          <div className="flex items-center gap-2 rounded-xl bg-gray-800/50 border border-gray-700/50 px-4 py-2.5 backdrop-blur-sm transition-all hover:border-gray-600/70 hover:bg-gray-800/70">
            <Clock className="h-4 w-4 text-indigo-400" />
            <select
              value={timeWindowSeconds}
              onChange={(e) => setTimeWindow(Number(e.target.value))}
              className="bg-transparent text-sm text-gray-300 font-medium focus:outline-none focus:ring-0 cursor-pointer"
            >
              {timeWindowOptions.map(option => (
                <option key={option.seconds} value={option.seconds} className="bg-gray-800">
                  {option.label}
                </option>
              ))}
            </select>
          </div>

          {/* Status indicator with pulsing effect */}
          <div className={`flex items-center gap-2.5 rounded-xl ${statusBg} border ${statusBorder} px-4 py-2.5 backdrop-blur-sm transition-all`}>
            <div className="relative">
              <StatusIcon className={`h-4 w-4 ${statusColor}`} />
              {isConnected && (
                <span className="absolute -right-0.5 -top-0.5 flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
                </span>
              )}
            </div>
            <span className={`text-sm font-semibold ${statusColor}`}>
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>

          {/* Refresh button with hover effect */}
          {onRefresh && (
            <button
              onClick={onRefresh}
              className="group relative rounded-xl bg-gray-800/50 border border-gray-700/50 p-2.5 text-gray-400 backdrop-blur-sm transition-all hover:border-indigo-500/50 hover:bg-indigo-500/10 hover:text-indigo-400 active:scale-95"
              title="Refresh data"
            >
              <Activity className="h-5 w-5 transition-transform group-hover:rotate-180 duration-500" />
            </button>
          )}
        </div>
      </div>
    </header>
  )
}
