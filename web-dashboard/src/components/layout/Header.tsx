import { Activity, AlertCircle, CheckCircle2, Zap, Clock } from 'lucide-react'
import { useMonitoringStore } from '../../stores/monitoringStore'

interface HeaderProps {
  isConnected: boolean
  onRefresh?: () => void
}

export default function Header({ isConnected, onRefresh }: HeaderProps) {
  const { timeWindowSeconds, setTimeWindow } = useMonitoringStore()
  const statusColor = isConnected ? 'text-green-400' : 'text-red-400'
  const statusBg = isConnected ? 'bg-green-500/20' : 'bg-red-500/20'
  const StatusIcon = isConnected ? CheckCircle2 : AlertCircle

  const timeWindowOptions = [
    { label: '1m', seconds: 60 },
    { label: '5m', seconds: 300 },
    { label: '15m', seconds: 900 },
    { label: '30m', seconds: 1800 },
    { label: '1h', seconds: 3600 },
  ]

  return (
    <header className="border-b border-gray-800 bg-gray-900 px-6 py-4 shadow-lg">
      <div className="flex items-center justify-between">
        {/* Logo and title */}
        <div className="flex items-center gap-3">
          <div className="rounded-lg bg-gradient-to-br from-blue-500 to-cyan-500 p-2">
            <Zap className="h-6 w-6 text-white" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-white">Guardia</h1>
            <p className="text-xs text-gray-400">Server Monitoring Dashboard</p>
          </div>
        </div>

        {/* Connection status and controls */}
        <div className="flex items-center gap-4">
          {/* Time window selector */}
          <div className="flex items-center gap-2 rounded-lg bg-gray-800 px-3 py-2">
            <Clock className="h-4 w-4 text-gray-400" />
            <select
              value={timeWindowSeconds}
              onChange={(e) => setTimeWindow(Number(e.target.value))}
              className="bg-transparent text-sm text-gray-300 focus:outline-none focus:ring-0"
            >
              {timeWindowOptions.map(option => (
                <option key={option.seconds} value={option.seconds}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>

          {/* Status indicator */}
          <div className={`flex items-center gap-2 rounded-lg ${statusBg} px-3 py-2`}>
            <StatusIcon className={`h-4 w-4 ${statusColor}`} />
            <span className={`text-sm font-medium ${statusColor}`}>
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>

          {/* Refresh button */}
          {onRefresh && (
            <button
              onClick={onRefresh}
              className="rounded-lg bg-gray-800 p-2 text-gray-400 hover:bg-gray-700 hover:text-white transition-colors"
              title="Refresh data"
            >
              <Activity className="h-5 w-5" />
            </button>
          )}
        </div>
      </div>
    </header>
  )
}
