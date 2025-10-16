import { useEffect, useState } from 'react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import { apiClient } from '../../api/client'
import type { ServerInfo } from '../../api/types'
import ServerDetail from './ServerDetail'
import { Activity, Server as ServerIcon } from 'lucide-react'

export default function ServerList() {
  const { servers, setServers, selectedServerId, setSelectedServer } = useMonitoringStore()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchServers = async () => {
      try {
        setLoading(true)
        setError(null)
        const response = await apiClient.getServers()
        setServers(response.servers)

        // Select first server by default
        if (response.servers.length > 0 && !selectedServerId) {
          setSelectedServer(response.servers[0].server_id)
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch servers')
      } finally {
        setLoading(false)
      }
    }

    fetchServers()
    const interval = setInterval(fetchServers, 5000)
    return () => clearInterval(interval)
  }, [setServers, selectedServerId, setSelectedServer])

  const getStatusColor = (status: ServerInfo['health_status']) => {
    switch (status) {
      case 'up':
        return 'badge-up'
      case 'down':
        return 'badge-down'
      case 'stale':
        return 'badge-stale'
      default:
        return 'badge-unknown'
    }
  }

  const getStatusDot = (status: ServerInfo['health_status']) => {
    const colors = {
      up: 'bg-green-500',
      down: 'bg-red-500',
      stale: 'bg-yellow-500',
      unknown: 'bg-gray-500'
    }
    return colors[status] || colors.unknown
  }

  const selectedServer = servers.find(s => s.server_id === selectedServerId)

  if (loading && servers.length === 0) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="relative">
          <div className="w-16 h-16 border-4 border-gray-800 border-t-blue-500 rounded-full animate-spin"></div>
          <div className="absolute inset-0 w-16 h-16 border-4 border-transparent border-t-purple-500 rounded-full animate-spin" style={{ animationDirection: 'reverse', animationDuration: '1s' }}></div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="rounded-2xl border border-red-500/30 bg-gradient-to-br from-red-500/10 to-red-600/5 p-6 text-red-400 backdrop-blur-sm">
        <div className="flex items-start gap-3">
          <Activity className="h-5 w-5 mt-0.5 flex-shrink-0" />
          <div>
            <h3 className="font-semibold mb-1">Connection Error</h3>
            <p className="text-sm text-red-400/80">{error}</p>
          </div>
        </div>
      </div>
    )
  }

  if (servers.length === 0) {
    return (
      <div className="rounded-2xl border border-gray-700/50 bg-gradient-to-br from-gray-800/50 to-gray-900/50 p-12 text-center backdrop-blur-sm">
        <ServerIcon className="h-16 w-16 text-gray-600 mx-auto mb-4" />
        <p className="text-lg text-gray-400 font-medium">No servers configured</p>
        <p className="text-sm text-gray-500 mt-2">Add servers to your monitoring configuration to get started</p>
      </div>
    )
  }

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-8 animate-slide-up">
      {/* Server list */}
      <div className="lg:col-span-1 space-y-3">
        <div className="flex items-center gap-2 mb-4">
          <ServerIcon className="h-5 w-5 text-gray-400" />
          <h2 className="text-lg font-semibold text-gray-300">Servers</h2>
          <span className="ml-auto text-xs font-medium text-gray-500 bg-gray-800/50 px-2 py-1 rounded-full">
            {servers.length}
          </span>
        </div>
        
        <div className="space-y-3">
          {servers.map((server, index) => {
            const isSelected = selectedServerId === server.server_id
            
            return (
              <button
                key={server.server_id}
                onClick={() => setSelectedServer(server.server_id)}
                className={`group relative w-full text-left p-4 rounded-xl border transition-all duration-300 overflow-hidden ${
                  isSelected
                    ? 'border-blue-500/50 bg-gradient-to-br from-blue-600/20 via-indigo-600/15 to-purple-600/10 shadow-lg shadow-blue-500/20'
                    : 'border-gray-700/50 bg-gradient-to-br from-gray-800/50 to-gray-900/50 hover:border-gray-600/70 hover:shadow-lg backdrop-blur-sm'
                }`}
                style={{ animationDelay: `${index * 50}ms` }}
              >
                {/* Animated gradient border for selected */}
                {isSelected && (
                  <div className="absolute inset-0 bg-gradient-to-r from-blue-500/20 via-purple-500/20 to-pink-500/20 opacity-50 blur-xl animate-pulse-glow"></div>
                )}
                
                <div className="relative flex items-center justify-between">
                  <div className="flex items-center gap-3 flex-1">
                    {/* Status indicator with pulse */}
                    <div className="relative">
                      <div className={`w-3 h-3 rounded-full ${getStatusDot(server.health_status)}`}></div>
                      {server.health_status === 'up' && (
                        <div className={`absolute inset-0 w-3 h-3 rounded-full ${getStatusDot(server.health_status)} animate-ping opacity-75`}></div>
                      )}
                    </div>
                    
                    {/* Server info */}
                    <div className="flex-1 min-w-0">
                      <h3 className="font-semibold text-white truncate">
                        {server.display_name}
                      </h3>
                      <p className="text-xs text-gray-400 truncate">
                        {server.server_id}
                      </p>
                    </div>
                  </div>
                  
                  {/* Status badge */}
                  <div className={`px-2.5 py-1 rounded-lg text-xs font-bold ${getStatusColor(server.health_status)}`}>
                    {server.health_status.toUpperCase()}
                  </div>
                </div>
                
                {/* Hover effect */}
                <div className="absolute inset-0 bg-gradient-to-r from-blue-500/0 via-purple-500/0 to-pink-500/0 group-hover:from-blue-500/5 group-hover:via-purple-500/5 group-hover:to-pink-500/5 transition-all duration-500 pointer-events-none"></div>
              </button>
            )
          })}
        </div>
      </div>

      {/* Server detail */}
      {selectedServer && (
        <div className="lg:col-span-2 animate-scale-in">
          <ServerDetail server={selectedServer} />
        </div>
      )}
    </div>
  )
}
