import { useState } from 'react'
import { useMonitoringStore } from './stores/monitoringStore'
import { useWebSocket } from './hooks/useWebSocket'
import { useMonitoring } from './hooks/useMonitoring'
import Layout from './components/layout/Layout'
import ServerList from './components/servers/ServerList'
import ServiceList from './components/services/ServiceList'
import AlertTimeline from './components/alerts/AlertTimeline'

type Tab = 'servers' | 'services' | 'alerts'

export default function App() {
  const [currentTab, setCurrentTab] = useState<Tab>('servers')
  const { isConnected } = useMonitoringStore()

  // Initialize monitoring data
  useMonitoring(5000)

  // Connect WebSocket for real-time updates
  useWebSocket({ autoConnect: true, reconnectInterval: 3000 })

  return (
    <Layout currentTab={currentTab} onTabChange={setCurrentTab} isConnected={isConnected}>
      {currentTab === 'servers' && <ServerList />}
      {currentTab === 'services' && <ServiceList />}
      {currentTab === 'alerts' && <AlertTimeline />}
    </Layout>
  )
}
