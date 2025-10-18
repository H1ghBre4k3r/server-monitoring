import { useMonitoringStore } from '../../stores/monitoringStore'

export default function AlertTimeline() {
  const { alerts } = useMonitoringStore()

  if (alerts.length === 0) {
    return (
      <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-8 text-center">
        <p className="text-gray-400">No alerts</p>
      </div>
    )
  }

  const getSeverityStyles = (severity: string) => {
    switch (severity) {
      case 'critical':
        return 'bg-red-500/20 border-red-500/50 text-red-400'
      case 'warning':
        return 'bg-yellow-500/20 border-yellow-500/50 text-yellow-400'
      default:
        return 'bg-blue-500/20 border-blue-500/50 text-blue-400'
    }
  }

  const getSeverityIcon = (severity: string) => {
    switch (severity) {
      case 'critical':
        return '⚡'
      case 'warning':
        return '⚠️'
      default:
        return 'ℹ️'
    }
  }

  return (
    <div className="space-y-3">
      <h2 className="text-2xl font-bold text-white mb-4">Alerts ({alerts.length})</h2>
      {alerts.map((alert) => (
        <div
          key={alert.id}
          className={`card border-l-4 ${getSeverityStyles(alert.severity)}`}
        >
          <div className="flex items-start gap-3">
            <span className="text-2xl">{getSeverityIcon(alert.severity)}</span>
            <div className="flex-1">
              <div className="flex items-center justify-between">
                <h3 className="font-semibold text-white">{alert.title}</h3>
                <span className="text-xs text-gray-400">
                  {alert.timestamp.toLocaleTimeString()}
                </span>
              </div>
              <p className="text-sm text-gray-300 mt-1">{alert.message}</p>
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}
