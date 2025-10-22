import { Server, AlertCircle, Bell } from 'lucide-react'
import { ThemeSwitcher } from './ThemeSwitcher'
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
      {/* Sidebar Header with Logo */}
      {!isMobile && (
        <div className={styles.sidebarHeader}>
          <div className={styles.sidebarLogo}>
            <svg viewBox="0 0 24 24" fill="none" strokeWidth={2.5}>
              <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </div>
          <div className={styles.sidebarTitle}>Guardia</div>
          <div className={styles.sidebarSubtitle}>Monitoring</div>
        </div>
      )}
      
      {/* Separator */}
      {!isMobile && <div className={styles.sidebarSeparator}></div>}
      
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
      </nav>
      
      {/* Theme Switcher at bottom of sidebar */}
      <div className={styles.sidebarFooter}>
        <ThemeSwitcher variant="sidebar" />
        {isMobile && (
          <div className={styles.mobileOrbContainer}>
            <div className={`${styles.orb} ${styles.mobileOrb}`}></div>
          </div>
        )}
      </div>
      
      {/* Mobile orb */}
      {isMobile && (
        <div className={styles.mobileOrbContainer}>
          <div className={`${styles.orb} ${styles.mobileOrb}`}></div>
        </div>
      )}
    </aside>
  )
}
