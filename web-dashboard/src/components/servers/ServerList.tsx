import { useEffect, useState } from 'react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import { apiClient } from '../../api/client'
import ServerDetail from './ServerDetail'
import ServerSelector from './ServerSelector'
import { Activity, Server as ServerIcon } from 'lucide-react'
import styles from './ServerList.module.css'

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
      <div className={styles.loadingContainer}>
        <div className={styles.spinner}>
          <div className={`${styles.spinnerRing} ${styles.spinnerRing1}`}></div>
          <div className={`${styles.spinnerRing} ${styles.spinnerRing2}`}></div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className={styles.errorContainer}>
        <div className={styles.errorContent}>
          <Activity className="h-5 w-5 mt-0.5 flex-shrink-0" />
          <div>
            <h3>Connection Error</h3>
            <p>{error}</p>
          </div>
        </div>
      </div>
    )
  }

  if (servers.length === 0) {
    return (
      <div className={styles.noServersContainer}>
        <ServerIcon className="h-16 w-16 mx-auto mb-4" />
        <p>No servers configured</p>
        <p>Add servers to your monitoring configuration to get started</p>
      </div>
    )
  }

  return (
    <div className={styles.container}>
      <div className={styles.headerSection}>
        <ServerSelector />
        <div className={styles.serverCountBadge}>
          <ServerIcon />
          <span>
            {servers.length} {servers.length === 1 ? 'Server' : 'Servers'}
          </span>
        </div>
      </div>

      {selectedServer && (
        <div className={styles.serverDetailContainer}>
          <ServerDetail server={selectedServer} />
        </div>
      )}
    </div>
  )
}
