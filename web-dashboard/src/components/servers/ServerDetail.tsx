import { ServerInfo } from '../../api/types'
import CpuChart from './CpuChart'
import TemperatureChart from './TemperatureChart'
import CircularProgress from './CircularProgress'
import { HardDrive, Thermometer, Info, Monitor, Cpu as CpuIcon, Box, Terminal, Waves } from 'lucide-react'
import styles from './ServerDetail.module.css'

interface ServerDetailProps {
  server: ServerInfo
}

export default function ServerDetail({ server }: ServerDetailProps) {
  const metrics = server.latest_metrics

  const getMemoryColor = (percentage: number) => {
    if (percentage >= 85) return { from: 'var(--red)', to: 'var(--maroon)', glow: 'rgba(var(--ctp-frappe-red-rgb), 0.3)' }
    if (percentage >= 70) return { from: 'var(--peach)', to: 'var(--orange)', glow: 'rgba(var(--ctp-frappe-peach-rgb), 0.3)' }
    return { from: 'var(--green)', to: 'var(--teal)', glow: 'rgba(var(--ctp-frappe-green-rgb), 0.3)' }
  }

  const getTempColor = (temp: number) => {
    if (temp >= 80) return styles.tempCritical
    if (temp >= 60) return styles.tempWarning
    if (temp >= 40) return styles.tempCaution
    return styles.tempNormal
  }

  const getTempBgColor = (temp: number) => {
    if (temp >= 80) return styles.tempBgCritical
    if (temp >= 60) return styles.tempBgWarning
    if (temp >= 40) return styles.tempBgCaution
    return styles.tempBgNormal
  }

  return (
    <div className={styles.container}>
      {/* Elegant System Info Banner */}
      <div className={styles.systemInfoBanner}>
        {/* Decorative wave pattern - hidden on mobile */}
        <div className={styles.decorativeWave}>
          <Waves />
        </div>
        
        <div className={styles.bannerContent}>
          <div className={styles.bannerHeader}>
            <div className={styles.bannerIconContainer}>
              <Info className={styles.bannerIcon} />
            </div>
            <div className={styles.bannerTitle}>
              <h3>System Overview</h3>
              <p>Server Configuration & Details</p>
            </div>
          </div>
          
          <div className={styles.statGrid}>
            <div className={`${styles.statCard} ${styles.statCardHostname}`}>
              <div className={styles.statCardHeader}>
                <div className={styles.statCardIconContainer}>
                  <Monitor className={styles.statCardIcon} />
                </div>
                <span className={styles.statCardLabel}>Hostname</span>
              </div>
              <p className={`${styles.statCardValue} ${styles.statCardHostname}`} title={metrics?.system?.host_name || undefined}>
                {metrics?.system?.host_name || 'N/A'}
              </p>
            </div>
            
            <div className={`${styles.statCard} ${styles.statCardArch}`}>
              <div className={styles.statCardHeader}>
                <div className={styles.statCardIconContainer}>
                  <CpuIcon className={styles.statCardIcon} />
                </div>
                <span className={styles.statCardLabel}>Architecture</span>
              </div>
              <p className={`${styles.statCardValue} ${styles.statCardArch}`}>
                {metrics?.cpus?.arch || 'N/A'}
              </p>
            </div>
            
            <div className={`${styles.statCard} ${styles.statCardOs}`}>
              <div className={styles.statCardHeader}>
                <div className={styles.statCardIconContainer}>
                  <Box className={styles.statCardIcon} />
                </div>
                <span className={styles.statCardLabel}>OS</span>
              </div>
              <p className={`${styles.statCardValue} ${styles.statCardOs}`} title={metrics?.system?.os_version || undefined}>
                {metrics?.system?.os_version || 'N/A'}
              </p>
            </div>
            
            <div className={`${styles.statCard} ${styles.statCardKernel}`}>
              <div className={styles.statCardHeader}>
                <div className={styles.statCardIconContainer}>
                  <Terminal className={styles.statCardIcon} />
                </div>
                <span className={styles.statCardLabel}>Kernel</span>
              </div>
              <p className={`${styles.statCardValue} ${styles.statCardKernel}`} title={metrics?.system?.kernel_version || undefined}>
                {metrics?.system?.kernel_version || 'N/A'}
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* CPU Section */}
      {metrics && metrics.cpus?.cpus && (
        <div className={styles.chartSection} style={{ animationDelay: '50ms' }}>
          <CpuChart serverId={server.server_id} currentMetrics={metrics} />
        </div>
      )}

      {/* Memory Section */}
      {metrics && (
        <div className={styles.memorySection} style={{ animationDelay: '100ms' }}>
          <div className={styles.memoryHeader}>
            <div className={styles.memoryHeaderLeft}>
              <div className={styles.memoryIconContainer}>
                <HardDrive className={styles.memoryIcon} />
              </div>
              <div className={styles.memoryTitle}>
                <h3>Memory Resources</h3>
                <p>RAM & Swap Allocation</p>
              </div>
            </div>
            
            {/* Memory efficiency badge */}
            <div className={styles.totalMemoryBadge}>
              <div className={styles.totalMemoryLabel}>Total Memory</div>
              <div className={styles.totalMemoryValue}>
                {((metrics.memory.total + metrics.memory.total_swap) / 1024 / 1024 / 1024).toFixed(1)} GB
              </div>
            </div>
          </div>
          
          {/* Circular gauges - stack on mobile */}
          <div className={styles.gaugeGrid}>
            <div className={`${styles.gaugeContainer} ${styles.gaugeContainerRam}`}>
              <CircularProgress
                percentage={(metrics.memory.used / metrics.memory.total) * 100}
                color={getMemoryColor((metrics.memory.used / metrics.memory.total) * 100)}
                label="RAM"
                value={`${(metrics.memory.used / 1024 / 1024 / 1024).toFixed(2)} / ${(metrics.memory.total / 1024 / 1024 / 1024).toFixed(2)} GB`}
                size={140}
                strokeWidth={12}
              />
            </div>
            
            <div className={`${styles.gaugeContainer} ${styles.gaugeContainerSwap}`}>
              <CircularProgress
                percentage={(metrics.memory.used_swap / metrics.memory.total_swap) * 100}
                color={getMemoryColor((metrics.memory.used_swap / metrics.memory.total_swap) * 100)}
                label="SWAP"
                value={`${(metrics.memory.used_swap / 1024 / 1024 / 1024).toFixed(2)} / ${(metrics.memory.total_swap / 1024 / 1024 / 1024).toFixed(2)} GB`}
                size={140}
                strokeWidth={12}
              />
            </div>
          </div>
          
          {/* Detailed linear bars */}
          <div className={styles.memoryBars}>
            <div className={`${styles.memoryBarGroup} ${styles.memoryBarRam}`}>
              <div className={styles.memoryBarHeader}>
                <div className={styles.memoryBarLabel}>
                  <div className={styles.memoryBarIndicator}></div>
                  <span className={styles.memoryBarLabelText}>
                    RAM Utilization
                  </span>
                </div>
                <span className={`${styles.memoryBarValue} ${styles.memoryBarRam} ${styles.memoryBarValue}`}>
                  {((metrics.memory.used / metrics.memory.total) * 100).toFixed(1)}%
                </span>
              </div>
              <div className={styles.progressBar}>
                {(() => {
                  const percentage = (metrics.memory.used / metrics.memory.total) * 100
                  const colors = getMemoryColor(percentage)
                  return (
                    <div
                      className={styles.progressFill}
                      style={{
                        width: `${percentage}%`,
                        '--progress-from': colors.from,
                        '--progress-to': colors.to,
                        '--progress-glow': colors.glow,
                      } as any}
                    />
                  )
                })()}
              </div>
            </div>
            
            <div className={`${styles.memoryBarGroup} ${styles.memoryBarSwap}`}>
              <div className={styles.memoryBarHeader}>
                <div className={styles.memoryBarLabel}>
                  <div className={`${styles.memoryBarIndicator} ${styles.memoryBarSwap} ${styles.memoryBarIndicator}`} style={{ animationDelay: '0.5s' }}></div>
                  <span className={styles.memoryBarLabelText}>
                    Swap Utilization
                  </span>
                </div>
                <span className={`${styles.memoryBarValue} ${styles.memoryBarSwap} ${styles.memoryBarValue}`}>
                  {((metrics.memory.used_swap / metrics.memory.total_swap) * 100).toFixed(1)}%
                </span>
              </div>
              <div className={styles.progressBar}>
                {(() => {
                  const percentage = (metrics.memory.used_swap / metrics.memory.total_swap) * 100
                  const colors = getMemoryColor(percentage)
                  return (
                    <div
                      className={styles.progressFill}
                      style={{
                        width: `${percentage}%`,
                        '--progress-from': colors.from,
                        '--progress-to': colors.to,
                        '--progress-glow': colors.glow,
                      } as any}
                    />
                  )
                })()}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Combined Temperature Section with Chart and Sensors */}
      {metrics && metrics.components.components.length > 0 && (
        <div className={styles.temperatureSection} style={{ animationDelay: '150ms' }}>
          <div className={styles.tempHeader}>
            <div className={styles.tempIconContainer}>
              <Thermometer className={styles.tempIcon} />
            </div>
            <div className={styles.tempTitle}>
              <h3>Temperature Monitoring</h3>
              <p>Real-time Thermal Sensors & History</p>
            </div>
          </div>
          
          {/* Current temperature sensors in elegant cards */}
          <div className={styles.currentReadings}>
            <h4 className={styles.readingsHeader}>
              <div className={styles.readingsHeaderIndicator}></div>
              Current Readings
            </h4>
            <div className={styles.sensorGrid}>
              {metrics.components.components.map((component, idx) => (
                <div 
                  key={idx} 
                  className={`${styles.sensorCard} ${getTempBgColor(component.temperature || 0)}`}
                >
                  <div className={styles.sensorCardContent}>
                    <div className={styles.sensorCardHeader}>
                      <div className={`${
                        component.temperature && component.temperature >= 80 ? styles.sensorIndicatorCritical :
                        component.temperature && component.temperature >= 60 ? styles.sensorIndicatorWarning :
                        component.temperature && component.temperature >= 40 ? styles.sensorIndicatorCaution :
                        styles.sensorIndicatorNormal
                      } ${styles.sensorIndicator}`}></div>
                      <span className={styles.sensorLabel}>
                        {component.name}
                      </span>
                    </div>
                    <span className={`${styles.sensorTemp} ${getTempColor(component.temperature || 0)}`}>
                      {component.temperature ? `${component.temperature.toFixed(1)}°C` : 'N/A'}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
          
          {/* Temperature chart */}
          <div className={styles.tempChartSection}>
            <h4 className={styles.tempChartHeader}>
              <div className={styles.readingsHeaderIndicator}></div>
              Historical Trends
            </h4>
            <TemperatureChart serverId={server.server_id} currentMetrics={metrics} />
          </div>
        </div>
      )}
    </div>
  )
}