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
    <div className="flex flex-col h-screen bg-gray-950">
      <Header isConnected={isConnected} onRefresh={handleRefresh} />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar currentTab={currentTab} onTabChange={onTabChange} />
        <main className="flex-1 overflow-auto">
          <div className="p-6">
            {children}
          </div>
        </main>
      </div>
    </div>
  )
}
