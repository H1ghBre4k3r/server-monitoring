import { useState, useRef, useEffect } from 'react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import type { ServerInfo } from '../../api/types'
import { ChevronDown, Check, Server as ServerIcon } from 'lucide-react'
import styles from './ServerSelector.module.css'

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
        return styles.statusUp
      case 'down':
        return styles.statusDown
      case 'stale':
        return styles.statusStale
      default:
        return styles.statusUnknown
    }
  }

  const handleSelect = (serverId: string) => {
    setSelectedServer(serverId)
    setIsOpen(false)
  }

  if (servers.length === 0) {
    return (
      <div className={styles.noServersContainer}>
        <ServerIcon />
        <span>No servers available</span>
      </div>
    )
  }

  return (
    <div className={styles.container} ref={dropdownRef}>
      {/* Selected server button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={styles.selectedButton}
      >
        <div className={styles.selectedButtonContent}>
          <div className={styles.selectedServerInfo}>
            {/* Status indicator */}
            <div className={styles.statusIndicator}>
              <div className={`${styles.statusDot} ${selectedServer ? getStatusColor(selectedServer.health_status) : styles.statusUnknown}`}></div>
              {selectedServer?.health_status === 'up' && (
                <div className={`${styles.statusPing} ${getStatusColor(selectedServer.health_status)}`}></div>
              )}
            </div>

            {/* Server info */}
            <div className={styles.serverInfo}>
              <div className={styles.serverInfoHeader}>
                <ServerIcon />
                <h3>
                  {selectedServer?.display_name || 'Select a server'}
                </h3>
              </div>
              {selectedServer && (
                <p className={styles.serverInfoId}>
                  {selectedServer.server_id}
                </p>
              )}
            </div>
          </div>

          {/* Dropdown icon */}
          <ChevronDown className={`${styles.dropdownIcon} ${isOpen ? styles.open : ''}`} />
        </div>
      </button>

      {/* Dropdown menu */}
      {isOpen && (
        <div className={styles.dropdownMenu}>
          <div className={styles.dropdownList}>
            {servers.map((server) => {
              const isSelected = selectedServerId === server.server_id

              return (
                <button
                  key={server.server_id}
                  onClick={() => handleSelect(server.server_id)}
                  className={`${styles.dropdownItem} ${isSelected ? styles.selected : ''}`}
                >
                  <div className={styles.dropdownItemContent}>
                    <div className={styles.dropdownItemInfo}>
                      {/* Status indicator */}
                      <div className={styles.dropdownItemIndicator}>
                        <div className={`${styles.dropdownItemDot} ${getStatusColor(server.health_status)}`}></div>
                        {server.health_status === 'up' && (
                          <div className={`${styles.dropdownItemPing} ${getStatusColor(server.health_status)}`}></div>
                        )}
                      </div>

                      <div className={styles.dropdownItemText}>
                        <h4>
                          {server.display_name}
                        </h4>
                        <p>
                          {server.server_id}
                        </p>
                      </div>
                    </div>

                    {/* Selected checkmark */}
                    {isSelected && (
                      <Check className={styles.checkmark} />
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