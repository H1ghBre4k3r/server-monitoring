import { Server, AlertCircle, Bell } from 'lucide-react'

interface SidebarProps {
  currentTab: 'servers' | 'services' | 'alerts'
  onTabChange: (tab: 'servers' | 'services' | 'alerts') => void
}

const TABS = [
  { id: 'servers' as const, label: 'Servers', icon: Server },
  { id: 'services' as const, label: 'Services', icon: AlertCircle },
  { id: 'alerts' as const, label: 'Alerts', icon: Bell },
]

export default function Sidebar({ currentTab, onTabChange }: SidebarProps) {
  return (
    <aside className="w-64 border-r border-gray-800 bg-gray-900 p-4">
      <nav className="space-y-2">
        {TABS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => onTabChange(id)}
            className={`w-full flex items-center gap-3 rounded-lg px-4 py-3 text-left font-medium transition-colors ${
              currentTab === id
                ? 'bg-blue-600 text-white'
                : 'text-gray-400 hover:bg-gray-800 hover:text-white'
            }`}
          >
            <Icon className="h-5 w-5" />
            {label}
          </button>
        ))}
      </nav>
    </aside>
  )
}
