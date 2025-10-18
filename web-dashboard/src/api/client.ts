/**
 * API Client - Handles HTTP requests to the monitoring hub
 */

import type {
  HealthResponse,
  ServersResponse,
  ServicesResponse,
  MetricsResponse,
  LatestMetricsResponse,
  ServiceChecksResponse,
  UptimeResponse,
  StatsResponse,
} from './types'

interface ClientConfig {
  baseUrl?: string
  token?: string
}

export class ApiClient {
  private baseUrl: string
  private token?: string

  constructor(config: ClientConfig = {}) {
    this.baseUrl = config.baseUrl || '/api/v1'
    this.token = config.token
  }

  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.baseUrl}${path}`
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...(typeof options.headers === 'object' && !Array.isArray(options.headers)
        ? (options.headers as Record<string, string>)
        : {}),
    }

    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`
    }

    const response = await fetch(url, {
      ...options,
      headers,
    })

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`)
    }

    return response.json() as Promise<T>
  }

  async health(): Promise<HealthResponse> {
    return this.request('/health')
  }

  async getStats(): Promise<StatsResponse> {
    return this.request('/stats')
  }

  async getServers(): Promise<ServersResponse> {
    return this.request('/servers')
  }

  async getServices(): Promise<ServicesResponse> {
    return this.request('/services')
  }

  async getServerMetrics(
    serverId: string,
    params?: { start?: string; end?: string; limit?: number }
  ): Promise<MetricsResponse> {
    const queryParams = new URLSearchParams()
    if (params?.start) queryParams.append('start', params.start)
    if (params?.end) queryParams.append('end', params.end)
    if (params?.limit) queryParams.append('limit', params.limit.toString())

    const query = queryParams.toString()
    const path = `/servers/${serverId}/metrics${query ? `?${query}` : ''}`
    return this.request(path)
  }

  async getLatestMetrics(serverId: string, limit: number = 100): Promise<LatestMetricsResponse> {
    return this.request(`/servers/${serverId}/metrics/latest?limit=${limit}`)
  }

  async getServiceChecks(
    serviceName: string,
    params?: { start?: string; end?: string }
  ): Promise<ServiceChecksResponse> {
    const queryParams = new URLSearchParams()
    if (params?.start) queryParams.append('start', params.start)
    if (params?.end) queryParams.append('end', params.end)

    const query = queryParams.toString()
    const path = `/services/${serviceName}/checks${query ? `?${query}` : ''}`
    return this.request(path)
  }

  async getUptime(
    serviceName: string,
    params?: { since?: string }
  ): Promise<UptimeResponse> {
    const queryParams = new URLSearchParams()
    if (params?.since) queryParams.append('since', params.since)

    const query = queryParams.toString()
    const path = `/services/${serviceName}/uptime${query ? `?${query}` : ''}`
    return this.request(path)
  }

  connectWebSocket(onMessage: (data: unknown) => void, onClose: () => void): WebSocket {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const host = window.location.host
    const url = `${protocol}//${host}/api/v1/stream`

    const ws = new WebSocket(url)

    if (this.token) {
      ws.onopen = () => {
        ws.send(JSON.stringify({ type: 'auth', token: this.token }))
      }
    }

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        onMessage(data)
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e)
      }
    }

    ws.onclose = () => {
      console.warn('WebSocket closed by server')
      onClose()
    }
    ws.onerror = (event) => {
      console.error('WebSocket error event:', event)
      console.warn('Check if the API server is running and WebSocket is accessible at /api/v1/stream')
    }

    return ws
  }
}

// Default client instance
export const apiClient = new ApiClient()
