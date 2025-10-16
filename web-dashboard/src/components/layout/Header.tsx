import { Activity, AlertCircle, CheckCircle2, Zap, Clock, Sparkles, Menu, X } from 'lucide-react'
import { useMonitoringStore } from '../../stores/monitoringStore'

interface HeaderProps {
  isConnected: boolean
  onRefresh?: () => void
  onMenuToggle?: () => void
  isMobileMenuOpen?: boolean
}

export default function Header({ isConnected, onRefresh, onMenuToggle, isMobileMenuOpen }: HeaderProps) {
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
    <header className="border-b border-gray-800/50 bg-gradient-to-r from-gray-900/95 via-gray-900/90 to-gray-900/95 px-3 sm:px-6 py-3 sm:py-4 shadow-2xl backdrop-blur-xl relative overflow-hidden">
      {/* Animated sparkles background - hidden on mobile */}
      <div className="absolute inset-0 pointer-events-none hidden sm:block">
        <Sparkles className="absolute top-2 right-1/4 h-4 w-4 text-blue-400/30 animate-pulse" style={{ animationDelay: '0s' }} />
        <Sparkles className="absolute bottom-3 left-1/3 h-3 w-3 text-purple-400/30 animate-pulse" style={{ animationDelay: '1s' }} />
        <Sparkles className="absolute top-4 left-1/2 h-3 w-3 text-cyan-400/30 animate-pulse" style={{ animationDelay: '2s' }} />
      </div>
      
      <div className="flex items-center justify-between relative z-10">
        {/* Mobile menu button + Logo */}
        <div className="flex items-center gap-2 sm:gap-4">
          {/* Mobile menu button */}
          {onMenuToggle && (
            <button
              onClick={onMenuToggle}
              className="lg:hidden rounded-lg bg-gray-800/50 p-2 text-gray-400 hover:text-white hover:bg-gray-700/70 transition-all active:scale-95"
              aria-label="Toggle menu"
            >
              {isMobileMenuOpen ? (
                <X className="h-5 w-5" />
              ) : (
                <Menu className="h-5 w-5" />
              )}
            </button>
          )}
          
          {/* Logo and title */}
          <div className="flex items-center gap-2 sm:gap-4">
            <div className="relative">
              <div className="absolute inset-0 rounded-xl bg-gradient-to-br from-blue-600 to-purple-600 opacity-75 blur-lg animate-pulse-glow"></div>
              <div className="relative rounded-xl bg-gradient-to-br from-blue-500 via-indigo-500 to-purple-600 p-1.5 sm:p-2.5 shadow-lg">
                <Zap className="h-4 w-4 sm:h-6 sm:w-6 text-white" strokeWidth={2.5} />
              </div>
            </div>
            <div>
              <h1 className="text-lg sm:text-2xl font-bold bg-gradient-to-r from-blue-400 via-indigo-400 to-purple-400 bg-clip-text text-transparent">
                Guardia
              </h1>
              <p className="text-xs text-gray-400 font-medium tracking-wide hidden sm:block">
                Server Monitoring Dashboard
              </p>
            </div>
          </div>
        </div>

        {/* Connection status and controls */}
        <div className="flex items-center gap-2 sm:gap-3">
          {/* Time window selector - simplified on mobile */}
          <div className="flex items-center gap-1 sm:gap-2 rounded-xl bg-gray-800/50 border border-gray-700/50 px-2 sm:px-4 py-1.5 sm:py-2.5 backdrop-blur-sm transition-all hover:border-gray-600/70 hover:bg-gray-800/70">
            <Clock className="h-3 w-3 sm:h-4 sm:w-4 text-indigo-400" />
            <select
              value={timeWindowSeconds}
              onChange={(e) => setTimeWindow(Number(e.target.value))}
              className="bg-transparent text-xs sm:text-sm text-gray-300 font-medium focus:outline-none focus:ring-0 cursor-pointer"
            >
              {timeWindowOptions.map(option => (
                <option key={option.seconds} value={option.seconds} className="bg-gray-800">
                  {option.label}
                </option>
              ))}
            </select>
          </div>

          {/* Status indicator - compact on mobile */}
          <div className={`flex items-center gap-1.5 sm:gap-2.5 rounded-xl ${statusBg} border ${statusBorder} px-2 sm:px-4 py-1.5 sm:py-2.5 backdrop-blur-sm transition-all`}>
            <div className="relative">
              <StatusIcon className={`h-3 w-3 sm:h-4 sm:w-4 ${statusColor}`} />
              {isConnected && (
                <span className="absolute -right-0.5 -top-0.5 flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
                </span>
              )}
            </div>
            <span className={`text-xs sm:text-sm font-semibold ${statusColor} hidden sm:inline`}>
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>

          {/* Refresh button */}
          {onRefresh && (
            <button
              onClick={onRefresh}
              className="group relative rounded-xl bg-gray-800/50 border border-gray-700/50 p-1.5 sm:p-2.5 text-gray-400 backdrop-blur-sm transition-all hover:border-indigo-500/50 hover:bg-indigo-500/10 hover:text-indigo-400 active:scale-95"
              title="Refresh data"
            >
              <Activity className="h-4 w-4 sm:h-5 sm:w-5 transition-transform group-hover:rotate-180 duration-500" />
            </button>
          )}
        </div>
      </div>
    </header>
  )
}
