import { useState, useRef, useEffect } from 'react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import type { ServerInfo } from '../../api/types'
import { ChevronDown, Check, Server as ServerIcon } from 'lucide-react'

export default function ServerSelector() {
  const { servers, selectedServerId, setSelectedServer } = useMonitoringStore()
  const [isOpen, setIsOpen] = useState(false)
  const dropdownRef = useRef<HTMLDivElement>(null)

  const selectedServer = servers.find(s => s.server_id === selectedServerId)

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false)
      }
    }

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside)
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [isOpen])

  const getStatusColor = (status: ServerInfo['health_status']) => {
    switch (status) {
      case 'up':
        return 'bg-green-500'
      case 'down':
        return 'bg-red-500'
      case 'stale':
        return 'bg-yellow-500'
      default:
        return 'bg-gray-500'
    }
  }

  const handleSelect = (serverId: string) => {
    setSelectedServer(serverId)
    setIsOpen(false)
  }

  if (servers.length === 0) {
    return (
      <div className="rounded-xl border border-gray-700/50 bg-gradient-to-br from-gray-800/50 to-gray-900/50 p-4 backdrop-blur-sm">
        <div className="flex items-center gap-3 text-gray-400">
          <ServerIcon className="h-5 w-5" />
          <span className="text-sm font-medium">No servers available</span>
        </div>
      </div>
    )
  }

  return (
    <div className="relative" ref={dropdownRef}>
      {/* Selected server button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="group relative w-full md:w-96 text-left p-4 rounded-xl border border-gray-700/50 bg-gradient-to-br from-gray-800/50 to-gray-900/50 hover:border-gray-600/70 transition-all duration-300 backdrop-blur-sm hover:shadow-lg"
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3 flex-1 min-w-0">
            {/* Status indicator */}
            <div className="relative flex-shrink-0">
              <div className={`w-3 h-3 rounded-full ${selectedServer ? getStatusColor(selectedServer.health_status) : 'bg-gray-500'}`}></div>
              {selectedServer?.health_status === 'up' && (
                <div className={`absolute inset-0 w-3 h-3 rounded-full ${getStatusColor(selectedServer.health_status)} animate-ping opacity-75`}></div>
              )}
            </div>

            {/* Server info */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <ServerIcon className="h-4 w-4 text-gray-400 flex-shrink-0" />
                <h3 className="font-semibold text-white truncate">
                  {selectedServer?.display_name || 'Select a server'}
                </h3>
              </div>
              {selectedServer && (
                <p className="text-xs text-gray-400 truncate mt-0.5">
                  {selectedServer.server_id}
                </p>
              )}
            </div>
          </div>

          {/* Dropdown icon */}
          <ChevronDown className={`h-5 w-5 text-gray-400 transition-transform duration-300 flex-shrink-0 ml-2 ${isOpen ? 'rotate-180' : ''}`} />
        </div>
      </button>

      {/* Dropdown menu */}
      {isOpen && (
        <div className="absolute z-50 mt-2 w-full md:w-[32rem] max-h-[70vh] overflow-y-auto rounded-xl border border-gray-700/50 bg-gray-900/95 backdrop-blur-xl shadow-2xl animate-scale-in">
          <div className="p-2 space-y-1">
            {servers.map((server) => {
              const isSelected = selectedServerId === server.server_id

              return (
                <button
                  key={server.server_id}
                  onClick={() => handleSelect(server.server_id)}
                  className={`group relative w-full text-left p-3 rounded-lg border transition-all duration-200 overflow-hidden ${
                    isSelected
                      ? 'border-blue-500/50 bg-gradient-to-br from-blue-600/20 to-purple-600/10'
                      : 'border-transparent hover:border-gray-600/50 hover:bg-gray-800/50'
                  }`}
                >
                  <div className="relative flex items-center justify-between">
                    <div className="flex items-center gap-2.5 flex-1 min-w-0">
                      {/* Status indicator */}
                      <div className="relative flex-shrink-0">
                        <div className={`w-2.5 h-2.5 rounded-full ${getStatusColor(server.health_status)}`}></div>
                        {server.health_status === 'up' && (
                          <div className={`absolute inset-0 w-2.5 h-2.5 rounded-full ${getStatusColor(server.health_status)} animate-ping opacity-75`}></div>
                        )}
                      </div>

                      <div className="flex-1 min-w-0">
                        <h4 className="font-semibold text-white text-sm truncate">
                          {server.display_name}
                        </h4>
                        <p className="text-xs text-gray-400 truncate">
                          {server.server_id}
                        </p>
                      </div>
                    </div>

                    {/* Selected checkmark */}
                    {isSelected && (
                      <Check className="h-4 w-4 text-blue-400 flex-shrink-0 ml-2" />
                    )}
                  </div>
                </button>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
