import { ReactNode, useState } from 'react'
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
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false)

  const handleRefresh = () => {
    window.location.reload()
  }

  const handleTabChange = (tab: 'servers' | 'services' | 'alerts') => {
    onTabChange(tab)
    setIsMobileMenuOpen(false) // Close menu on mobile after selection
  }

  return (
    <div className="flex flex-col h-screen overflow-hidden relative">
      <Header 
        isConnected={isConnected} 
        onRefresh={handleRefresh}
        onMenuToggle={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
        isMobileMenuOpen={isMobileMenuOpen}
      />
      
      <div className="flex flex-1 overflow-hidden relative">
        {/* Desktop Sidebar */}
        <div className="hidden lg:block">
          <Sidebar currentTab={currentTab} onTabChange={handleTabChange} />
        </div>
        
        {/* Mobile Sidebar Overlay */}
        {isMobileMenuOpen && (
          <>
            {/* Backdrop */}
            <div 
              className="fixed inset-0 bg-black/60 backdrop-blur-sm z-40 lg:hidden"
              onClick={() => setIsMobileMenuOpen(false)}
            />
            
            {/* Sidebar */}
            <div className="fixed inset-y-0 left-0 w-64 z-50 lg:hidden animate-slide-in-left">
              <Sidebar currentTab={currentTab} onTabChange={handleTabChange} />
            </div>
          </>
        )}
        
        <main className="flex-1 overflow-auto relative">
          <div className="p-4 sm:p-6 lg:p-8 animate-fade-in relative z-10">
            {children}
          </div>
        </main>
      </div>
      
      {/* Enhanced ambient background effects */}
      <div className="fixed inset-0 pointer-events-none overflow-hidden z-0">
        {/* Large floating orbs */}
        <div className="absolute top-0 right-0 w-[400px] sm:w-[600px] h-[400px] sm:h-[600px] bg-blue-500/10 rounded-full blur-[120px] animate-pulse-glow"></div>
        <div className="absolute bottom-0 left-0 w-[400px] sm:w-[600px] h-[400px] sm:h-[600px] bg-purple-500/10 rounded-full blur-[120px] animate-pulse-glow" style={{ animationDelay: '1s' }}></div>
        
        {/* Medium floating orbs - hidden on mobile */}
        <div className="hidden sm:block absolute top-1/4 left-1/4 w-[300px] h-[300px] bg-cyan-500/8 rounded-full blur-[80px] animate-float"></div>
        <div className="hidden sm:block absolute bottom-1/4 right-1/4 w-[300px] h-[300px] bg-pink-500/8 rounded-full blur-[80px] animate-float" style={{ animationDelay: '2s' }}></div>
        
        {/* Small accent orbs - hidden on mobile */}
        <div className="hidden md:block absolute top-1/2 right-1/3 w-[150px] h-[150px] bg-indigo-500/10 rounded-full blur-[60px] animate-bounce-slow"></div>
        <div className="hidden md:block absolute top-2/3 left-1/2 w-[150px] h-[150px] bg-emerald-500/10 rounded-full blur-[60px] animate-bounce-slow" style={{ animationDelay: '1.5s' }}></div>
        
        {/* Subtle grid pattern overlay */}
        <div className="absolute inset-0 opacity-[0.02] hidden sm:block" style={{
          backgroundImage: `
            linear-gradient(90deg, rgba(99, 102, 241, 0.5) 1px, transparent 1px),
            linear-gradient(rgba(99, 102, 241, 0.5) 1px, transparent 1px)
          `,
          backgroundSize: '50px 50px'
        }}></div>
      </div>
    </div>
  )
}
