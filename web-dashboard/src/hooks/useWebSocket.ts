import { useEffect, useRef, useCallback } from 'react'
import { useMonitoringStore } from '../stores/monitoringStore'
import { apiClient } from '../api/client'
import type { WsEvent, MetricEvent, ServiceCheckEvent } from '../api/types'

interface UseWebSocketOptions {
  autoConnect?: boolean
  reconnectInterval?: number
  maxReconnectAttempts?: number
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const {
    autoConnect = true,
    reconnectInterval = 3000,
    maxReconnectAttempts = 10,
  } = options

  const wsRef = useRef<WebSocket | null>(null)
  const reconnectCountRef = useRef(0)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  const {
    setConnected,
    addMetricPoint,
    addAlert,
  } = useMonitoringStore()

  const handleMessage = useCallback((event: WsEvent) => {
    try {
      if (event.type === 'metric') {
        const metricEvent = event as MetricEvent
        // Validate data exists before adding
        if (metricEvent.metrics?.cpus || metricEvent.metrics?.temperatures) {
          addMetricPoint(metricEvent.server_id, {
            timestamp: new Date(metricEvent.timestamp),
            server_id: metricEvent.server_id,
            data: metricEvent.metrics,
          })
        }
      } else if (event.type === 'service_check') {
        const serviceEvent = event as ServiceCheckEvent
        // Could emit alerts here if status is down
        if (serviceEvent.status === 'down') {
          addAlert({
            id: `${serviceEvent.service_name}-${serviceEvent.timestamp}`,
            timestamp: new Date(serviceEvent.timestamp),
            type: 'service',
            severity: 'critical',
            title: `Service Down: ${serviceEvent.service_name}`,
            message: `Service is not responding`,
          })
        }
      }
    } catch (err) {
      console.warn('Error handling WebSocket message:', err, event)
    }
  }, [addMetricPoint, addAlert])

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return
    }

    try {
      wsRef.current = apiClient.connectWebSocket(
        (data) => {
          handleMessage(data as WsEvent)
          setConnected(true)
          reconnectCountRef.current = 0
        },
        () => {
          // Connection closed
          console.warn('WebSocket connection closed')
          setConnected(false)
          attemptReconnect()
        }
      )

      console.log('WebSocket connected successfully')
      setConnected(true)
    } catch (err) {
      console.error('WebSocket connection error:', err)
      setConnected(false)
      attemptReconnect()
    }
  }, [handleMessage, setConnected])

  const attemptReconnect = useCallback(() => {
    if (reconnectCountRef.current < maxReconnectAttempts) {
      reconnectCountRef.current += 1
      console.log(
        `Attempting to reconnect (${reconnectCountRef.current}/${maxReconnectAttempts})...`
      )

      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
      }

      reconnectTimeoutRef.current = setTimeout(() => {
        connect()
      }, reconnectInterval)
    } else {
      console.error('Max reconnection attempts reached')
      setConnected(false)
    }
  }, [connect, reconnectInterval, maxReconnectAttempts, setConnected])

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
    }

    if (wsRef.current) {
      wsRef.current.close()
      wsRef.current = null
    }

    setConnected(false)
  }, [setConnected])

  useEffect(() => {
    if (autoConnect) {
      connect()
    }

    return () => {
      disconnect()
    }
  }, [autoConnect, connect, disconnect])

  return {
    connect,
    disconnect,
    isConnected: wsRef.current?.readyState === WebSocket.OPEN,
  }
}
