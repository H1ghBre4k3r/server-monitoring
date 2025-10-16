import { ReactNode } from 'react'
import Header from './Header'
import Sidebar from './Sidebar'

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
  const handleRefresh = () => {
    // Trigger a page reload to refresh data
    window.location.reload()
  }

  return (
    <div className="flex flex-col h-screen overflow-hidden">
      <Header isConnected={isConnected} onRefresh={handleRefresh} />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar currentTab={currentTab} onTabChange={onTabChange} />
        <main className="flex-1 overflow-auto">
          <div className="p-8 animate-fade-in">
            {children}
          </div>
        </main>
      </div>
      
      {/* Ambient background effects */}
      <div className="fixed inset-0 pointer-events-none overflow-hidden">
        <div className="absolute top-0 right-0 w-[500px] h-[500px] bg-blue-500/10 rounded-full blur-[120px] animate-pulse-glow"></div>
        <div className="absolute bottom-0 left-0 w-[500px] h-[500px] bg-purple-500/10 rounded-full blur-[120px] animate-pulse-glow" style={{ animationDelay: '1s' }}></div>
      </div>
    </div>
  )
}
