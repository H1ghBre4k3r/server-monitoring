import { Server, AlertCircle, Bell } from 'lucide-react'
import styles from './Sidebar.module.css'

interface SidebarProps {
  currentTab: 'servers' | 'services' | 'alerts'
  onTabChange: (tab: 'servers' | 'services' | 'alerts') => void
  isMobile?: boolean
}

const TABS = [
  { id: 'servers' as const, label: 'Servers', icon: Server, gradientClass: styles.navButtonServersGradient, tabClass: styles.navButtonServers },
  { id: 'services' as const, label: 'Services', icon: AlertCircle, gradientClass: styles.navButtonServicesGradient, tabClass: styles.navButtonServices },
  { id: 'alerts' as const, label: 'Alerts', icon: Bell, gradientClass: styles.navButtonAlertsGradient, tabClass: styles.navButtonAlerts },
]

export default function Sidebar({ currentTab, onTabChange, isMobile = false }: SidebarProps) {
  return (
    <aside className={`${styles.sidebar} ${isMobile ? styles.sidebarMobile : styles.sidebarDesktop}`}>
      <nav className={`${styles.nav} ${isMobile ? styles.navMobile : ''}`}>
        {TABS.map(({ id, label, icon: Icon, gradientClass, tabClass }) => {
          const isActive = currentTab === id
          
          return (
            <button
              key={id}
              onClick={() => onTabChange(id)}
              className={`${styles.navButton} ${tabClass} ${isActive ? styles.navButtonActive : ''}`}
            >
              {isActive && (
                <div className={`${styles.navButtonActiveBackground} ${gradientClass}`}></div>
              )}
              
              <div className={styles.iconContainer}>
                <Icon strokeWidth={2.5} />
              </div>
              
              <span className={styles.label}>
                {label}
              </span>
            </button>
          )
        })}
        
        {!isMobile && (
          <div className={`${styles.orb} ${styles.desktopOrb}`}></div>
        )}
        
        {isMobile && (
          <div className={styles.mobileOrbContainer}>
            <div className={`${styles.orb} ${styles.mobileOrb}`}></div>
          </div>
        )}
      </nav>
    </aside>
  )
}
