import { useMonitoringStore } from '../../stores/monitoringStore'
import styles from './AlertTimeline.module.css'

export default function AlertTimeline() {
  const { alerts } = useMonitoringStore()

  if (alerts.length === 0) {
    return (
      <div className={styles.noAlertsContainer}>
        <p>No alerts</p>
      </div>
    )
  }

  const getSeverityStyles = (severity: string) => {
    switch (severity) {
      case 'critical':
        return styles.critical
      case 'warning':
        return styles.warning
      default:
        return styles.info
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
    <div className={styles.container}>
      <h2>Alerts ({alerts.length})</h2>
      {alerts.map((alert) => (
        <div
          key={alert.id}
          className={`${styles.alert} ${getSeverityStyles(alert.severity)}`}
        >
          <div className={styles.alertContent}>
            <span className={styles.alertIcon}>{getSeverityIcon(alert.severity)}</span>
            <div className={styles.alertDetails}>
              <div className={styles.alertHeader}>
                <h3 className={styles.alertTitle}>{alert.title}</h3>
                <span className={styles.alertTimestamp}>
                  {alert.timestamp.toLocaleTimeString()}
                </span>
              </div>
              <p className={styles.alertMessage}>{alert.message}</p>
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}