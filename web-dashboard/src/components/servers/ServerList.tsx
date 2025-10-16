import { useEffect, useState } from 'react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import { apiClient } from '../../api/client'
import ServerDetail from './ServerDetail'
import ServerSelector from './ServerSelector'
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
    <div className="space-y-6 animate-slide-up">
      {/* Elegant server selector */}
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <ServerSelector />
        
        {/* Server count badge */}
        <div className="flex items-center gap-2 px-4 py-2 rounded-xl border border-gray-700/50 bg-gradient-to-br from-gray-800/50 to-gray-900/50 backdrop-blur-sm">
          <ServerIcon className="h-4 w-4 text-gray-400" />
          <span className="text-sm font-medium text-gray-400">
            {servers.length} {servers.length === 1 ? 'Server' : 'Servers'}
          </span>
        </div>
      </div>

      {/* Server detail */}
      {selectedServer && (
        <div className="animate-scale-in">
          <ServerDetail server={selectedServer} />
        </div>
      )}
    </div>
  )
}
