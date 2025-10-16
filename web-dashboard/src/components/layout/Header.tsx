import { Activity, AlertCircle, CheckCircle2, Zap } from 'lucide-react'

interface HeaderProps {
  isConnected: boolean
  onRefresh?: () => void
}

export default function Header({ isConnected, onRefresh }: HeaderProps) {
  const statusColor = isConnected ? 'text-green-400' : 'text-red-400'
  const statusBg = isConnected ? 'bg-green-500/20' : 'bg-red-500/20'
  const StatusIcon = isConnected ? CheckCircle2 : AlertCircle

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
