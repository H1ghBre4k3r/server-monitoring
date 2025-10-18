import { useEffect, useState } from 'react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import { apiClient } from '../../api/client'

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
        return 'badge-up'
      case 'down':
        return 'badge-down'
      case 'degraded':
        return 'badge-degraded'
      case 'stale':
        return 'badge-stale'
      default:
        return 'badge-unknown'
    }
  }

  if (loading && services.length === 0) {
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

  if (services.length === 0) {
    return (
      <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-8 text-center">
        <p className="text-gray-400">No services configured</p>
      </div>
    )
  }

  return (
    <div className="card">
      <h2 className="text-2xl font-bold text-white mb-4">Services</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-700">
              <th className="px-4 py-3 text-left text-gray-400 font-medium">Service</th>
              <th className="px-4 py-3 text-left text-gray-400 font-medium">URL</th>
              <th className="px-4 py-3 text-left text-gray-400 font-medium">Status</th>
              <th className="px-4 py-3 text-left text-gray-400 font-medium">Response Time</th>
              <th className="px-4 py-3 text-left text-gray-400 font-medium">Last Check</th>
            </tr>
          </thead>
          <tbody>
            {services.map((service) => (
              <tr key={service.name} className="border-b border-gray-800 hover:bg-gray-800/50 transition-colors">
                <td className="px-4 py-3 text-white font-medium">{service.name}</td>
                <td className="px-4 py-3 text-gray-400 text-xs break-all">{service.url}</td>
                <td className="px-4 py-3">
                  <span className={`badge ${getStatusColor(service.health_status)}`}>
                    {service.health_status.toUpperCase()}
                  </span>
                </td>
                <td className="px-4 py-3 text-white">
                  {service.response_time_ms !== null ? `${service.response_time_ms}ms` : 'N/A'}
                </td>
                <td className="px-4 py-3 text-gray-400">
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
