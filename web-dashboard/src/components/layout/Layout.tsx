import { ReactNode, useState } from 'react'
import Header from './Header'
import Sidebar from './Sidebar'
import styles from './Layout.module.css'

interface LayoutProps {
  children: ReactNode
  currentTab: 'servers' | 'services' | 'alerts'
  onTabChange: (tab: 'servers' | 'services' | 'alerts') => void
  isConnected: boolean
}

export default function Layout({
  children,
  currentTab,
  onTabChange,
  isConnected,
}: LayoutProps) {
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false)

  const handleRefresh = () => {
    window.location.reload()
  }

  const handleTabChange = (tab: 'servers' | 'services' | 'alerts') => {
    onTabChange(tab)
    setIsMobileMenuOpen(false) // Close menu on mobile after selection
  }

  return (
    <div className={styles.layout}>
      <Header 
        isConnected={isConnected} 
        onRefresh={handleRefresh}
        onMenuToggle={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
        isMobileMenuOpen={isMobileMenuOpen}
      />
      
      <div className={styles.mainContent}>
        {/* Desktop Sidebar */}
        <div className={styles.sidebarDesktop}>
          <Sidebar currentTab={currentTab} onTabChange={handleTabChange} isMobile={false} />
        </div>
        
        {/* Mobile Sidebar Overlay */}
        {isMobileMenuOpen && (
          <>
            {/* Backdrop */}
            <div 
              className={styles.mobileBackdrop}
              onClick={() => setIsMobileMenuOpen(false)}
            />
            
            {/* Sidebar - Full height on mobile */}
            <div className={styles.mobileSidebar}>
              <Sidebar currentTab={currentTab} onTabChange={handleTabChange} isMobile={true} />
            </div>
          </>
        )}
        
        <main className={styles.contentArea}>
          <div className={styles.contentWrapper}>
            {children}
          </div>
        </main>
      </div>
      
      {/* Enhanced ambient background effects */}
      <div className={styles.backgroundEffects}>
        <div className={`${styles.orb} ${styles.orb1}`}></div>
        <div className={`${styles.orb} ${styles.orb2}`}></div>
        <div className={styles.gridOverlay}></div>
      </div>
    </div>
  )
}
