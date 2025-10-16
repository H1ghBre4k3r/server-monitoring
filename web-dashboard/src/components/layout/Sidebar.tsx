import { Server, AlertCircle, Bell } from 'lucide-react'

interface SidebarProps {
  currentTab: 'servers' | 'services' | 'alerts'
  onTabChange: (tab: 'servers' | 'services' | 'alerts') => void
}

const TABS = [
  { id: 'servers' as const, label: 'Servers', icon: Server, gradient: 'from-blue-500 to-cyan-500' },
  { id: 'services' as const, label: 'Services', icon: AlertCircle, gradient: 'from-purple-500 to-pink-500' },
  { id: 'alerts' as const, label: 'Alerts', icon: Bell, gradient: 'from-orange-500 to-red-500' },
]

export default function Sidebar({ currentTab, onTabChange }: SidebarProps) {
  return (
    <aside className="w-72 border-r border-gray-800/50 bg-gradient-to-b from-gray-900/95 to-gray-900/80 p-6 backdrop-blur-xl">
      <nav className="space-y-3">
        {TABS.map(({ id, label, icon: Icon, gradient }) => {
          const isActive = currentTab === id
          
          return (
            <button
              key={id}
              onClick={() => onTabChange(id)}
              className={`group relative w-full flex items-center gap-4 rounded-xl px-5 py-4 text-left font-semibold transition-all duration-300 ${
                isActive
                  ? 'text-white shadow-lg'
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              {/* Active background with gradient */}
              {isActive && (
                <>
                  <div className={`absolute inset-0 rounded-xl bg-gradient-to-r ${gradient} opacity-90 transition-opacity duration-300`}></div>
                  <div className={`absolute inset-0 rounded-xl bg-gradient-to-r ${gradient} opacity-50 blur-xl`}></div>
                </>
              )}
              
              {/* Hover background */}
              {!isActive && (
                <div className="absolute inset-0 rounded-xl bg-gray-800/0 group-hover:bg-gray-800/50 transition-all duration-300 backdrop-blur-sm"></div>
              )}
              
              {/* Icon with special styling */}
              <div className={`relative z-10 rounded-lg p-2 transition-all duration-300 ${
                isActive 
                  ? 'bg-white/20 shadow-lg' 
                  : 'bg-gray-800/50 group-hover:bg-gray-700/70'
              }`}>
                <Icon className="h-5 w-5" strokeWidth={2.5} />
              </div>
              
              {/* Label */}
              <span className="relative z-10 text-base tracking-wide">
                {label}
              </span>
              
              {/* Active indicator */}
              {isActive && (
                <div className="absolute right-4 top-1/2 -translate-y-1/2 h-2 w-2 rounded-full bg-white shadow-lg animate-pulse"></div>
              )}
            </button>
          )
        })}
      </nav>
      
      {/* Decorative gradient orb */}
      <div className="absolute bottom-10 left-1/2 -translate-x-1/2 w-32 h-32 bg-gradient-to-br from-blue-500/20 to-purple-500/20 rounded-full blur-3xl opacity-50 pointer-events-none"></div>
    </aside>
  )
}
