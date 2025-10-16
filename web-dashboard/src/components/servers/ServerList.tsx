import { useEffect, useState } from 'react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import { apiClient } from '../../api/client'
import type { ServerInfo } from '../../api/types'
import ServerDetail from './ServerDetail'

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
        return 'bg-green-500/20 text-green-400 badge-up'
      case 'down':
        return 'bg-red-500/20 text-red-400 badge-down'
      case 'stale':
        return 'bg-yellow-500/20 text-yellow-400 badge-stale'
      default:
        return 'bg-gray-500/20 text-gray-400 badge-unknown'
    }
  }

  const selectedServer = servers.find(s => s.server_id === selectedServerId)

  if (loading && servers.length === 0) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-blue-500"></div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="rounded-lg border border-red-500/20 bg-red-500/10 p-4 text-red-400">
        {error}
      </div>
    )
  }

  if (servers.length === 0) {
    return (
      <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-8 text-center">
        <p className="text-gray-400">No servers configured</p>
      </div>
    )
  }

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
      {/* Server list */}
      <div className="lg:col-span-1">
        <div className="space-y-2">
          {servers.map((server) => (
            <button
              key={server.server_id}
              onClick={() => setSelectedServer(server.server_id)}
              className={`w-full text-left p-3 rounded-lg border transition-colors ${
                selectedServerId === server.server_id
                  ? 'border-blue-500 bg-blue-600/20'
                  : 'border-gray-700 bg-gray-800/50 hover:bg-gray-800'
              }`}
            >
              <div className="flex items-center justify-between">
                <div className="flex-1">
                  <h3 className="font-medium text-white">{server.display_name}</h3>
                  <p className="text-xs text-gray-400">{server.server_id}</p>
                </div>
                <div className={`px-2 py-1 rounded text-xs font-medium ${getStatusColor(server.health_status)}`}>
                  {server.health_status.toUpperCase()}
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Server detail */}
      {selectedServer && (
        <div className="lg:col-span-2">
          <ServerDetail server={selectedServer} />
        </div>
      )}
    </div>
  )
}
