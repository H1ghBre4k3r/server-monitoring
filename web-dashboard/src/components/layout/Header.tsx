import { Activity, AlertCircle, CheckCircle2, Zap, Clock, Sparkles, Menu, X } from 'lucide-react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import styles from './Header.module.css'

interface HeaderProps {
  isConnected: boolean
  onRefresh?: () => void
  onMenuToggle?: () => void
  isMobileMenuOpen?: boolean
}

export default function Header({ isConnected, onRefresh, onMenuToggle, isMobileMenuOpen }: HeaderProps) {
  const { timeWindowSeconds, setTimeWindow } = useMonitoringStore()
  const statusIconClass = isConnected ? `${styles.statusIndicator} ${styles.connected}` : `${styles.statusIndicator} ${styles.disconnected}`
  const statusIconWrapperClass = isConnected ? `${styles.statusIconWrapper} ${styles.connected}` : `${styles.statusIconWrapper} ${styles.disconnected}`
  const StatusIcon = isConnected ? CheckCircle2 : AlertCircle

  const timeWindowOptions = [
    { label: '1m', seconds: 60 },
    { label: '5m', seconds: 300 },
    { label: '15m', seconds: 900 },
    { label: '30m', seconds: 1800 },
    { label: '1h', seconds: 3600 },
  ]

  return (
    <header className={styles.header}>
      <div className={styles.sparklesBackground}>
        <Sparkles className={`${styles.sparkle} ${styles.sparkle1}`} />
        <Sparkles className={`${styles.sparkle} ${styles.sparkle2}`} />
        <Sparkles className={`${styles.sparkle} ${styles.sparkle3}`} />
      </div>
      
      <div className={styles.headerContent}>
        <div className={styles.leftSection}>
          {/* Mobile menu button */}
          {onMenuToggle && (
            <button
              onClick={onMenuToggle}
              className={styles.menuButton}
              aria-label="Toggle menu"
            >
              {isMobileMenuOpen ? (
                <X className="h-5 w-5" />
              ) : (
                <Menu className="h-5 w-5" />
              )}
            </button>
          )}
          
          <div className={styles.logoSection}>
            <div className={styles.logoContainer}>
              <div className={styles.logoGlow}></div>
              <div className={styles.logo}>
                <Zap className="h-4 w-4 sm:h-6 sm:w-6 text-white" strokeWidth={2.5} />
              </div>
            </div>
            <div className={styles.titleText}>
              <h1>Guardia</h1>
              <p>Server Monitoring Dashboard</p>
            </div>
          </div>
        </div>

        <div className={styles.rightSection}>
          {/* Time window selector - simplified on mobile */}
          <div className={styles.timeWindowSelector}>
            <Clock />
            <select
              value={timeWindowSeconds}
              onChange={(e) => setTimeWindow(Number(e.target.value))}
            >
              {timeWindowOptions.map(option => (
                <option key={option.seconds} value={option.seconds} style={{ backgroundColor: 'var(--mantle)' }}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>

          {/* Status indicator - compact on mobile */}
          <div className={statusIconClass}>
            <div className={statusIconWrapperClass}>
              <StatusIcon className={styles.statusIcon} />
              {isConnected && (
                <span className={styles.statusPing}>
                  <span className={styles.pingRing}></span>
                  <span className={styles.pingDot}></span>
                </span>
              )}
            </div>
            <span className={styles.statusText}>
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>

          {/* Refresh button */}
          {onRefresh && (
            <button
              onClick={onRefresh}
              className={styles.refreshButton}
              title="Refresh data"
            >
              <Activity />
            </button>
          )}
        </div>
      </div>
    </header>
  )
}
