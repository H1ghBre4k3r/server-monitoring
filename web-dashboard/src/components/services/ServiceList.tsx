import { useEffect, useState } from 'react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import { apiClient } from '../../api/client'
import styles from './ServiceList.module.css'

export default function ServiceList() {
  const { services, setServices } = useMonitoringStore()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchServices = async () => {
      try {
        setLoading(true)
        setError(null)
        const response = await apiClient.getServices()
        setServices(response.services)
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch services')
      } finally {
        setLoading(false)
      }
    }

    fetchServices()
    const interval = setInterval(fetchServices, 5000)
    return () => clearInterval(interval)
  }, [setServices])

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'up':
        return styles.badgeUp
      case 'down':
        return styles.badgeDown
      case 'degraded':
        return styles.badgeDegraded
      case 'stale':
        return styles.badgeStale
      default:
        return styles.badgeUnknown
    }
  }

  if (loading && services.length === 0) {
    return (
      <div className={styles.loadingContainer}>
        <div className={`${styles.spinner} w-8 h-8`}></div>
      </div>
    )
  }

  if (error) {
    return (
      <div className={styles.errorContainer}>
        {error}
      </div>
    )
  }

  if (services.length === 0) {
    return (
      <div className={styles.noServicesContainer}>
        <p>No services configured</p>
      </div>
    )
  }

  return (
    <div className={styles.card}>
      <h2>Services</h2>
      <div className={styles.tableContainer}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th>Service</th>
              <th>URL</th>
              <th>Status</th>
              <th>Response Time</th>
              <th>Last Check</th>
            </tr>
          </thead>
          <tbody>
            {services.map((service) => (
              <tr key={service.name}>
                <td className={styles.serviceName}>{service.name}</td>
                <td className={styles.serviceUrl}>{service.url}</td>
                <td>
                  <span className={`${styles.badge} ${getStatusColor(service.health_status)}`}>
                    {service.health_status.toUpperCase()}
                  </span>
                </td>
                <td className={styles.responseTime}>
                  {service.response_time_ms !== null ? `${service.response_time_ms}ms` : 'N/A'}
                </td>
                <td className={styles.lastCheck}>
                  {service.last_check ? new Date(service.last_check).toLocaleTimeString() : 'Never'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}