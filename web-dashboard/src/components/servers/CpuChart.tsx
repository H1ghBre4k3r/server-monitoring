import { useMemo } from 'react'
import ReactECharts from 'echarts-for-react'
import { useMonitoringStore } from '../../stores/monitoringStore'

interface CpuChartProps {
  serverId: string
}

export default function CpuChart({ serverId }: CpuChartProps) {
  const { metricsHistory, timeWindowSeconds } = useMonitoringStore()
  const metrics = metricsHistory.get(serverId) || []

  const chartOption = useMemo(() => {
    if (metrics.length === 0) {
      return {
        title: { text: 'No data available' },
        grid: { top: 50, right: 50, bottom: 50, left: 50 },
      }
    }

    // Calculate time window bounds
    const now = Date.now()
    const windowStart = now - timeWindowSeconds * 1000
    const filtered = metrics.filter(m => m.timestamp.getTime() >= windowStart)

    if (filtered.length === 0) {
      return {
        title: { text: 'No data in time window' },
        grid: { top: 50, right: 50, bottom: 50, left: 50 },
      }
    }

    // Sort by timestamp to ensure proper time series
    filtered.sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime())

    // Get number of cores with null checks
    const numCores = filtered[0]?.data?.cpus?.cpus?.length || 0

    if (numCores === 0) {
      return {
        title: { text: 'No CPU data available' },
        grid: { top: 50, right: 50, bottom: 50, left: 50 },
      }
    }

    // Create datasets for each core
    const series: any[] = []
    for (let i = 0; i < numCores; i++) {
      const coreData: [number, number][] = []

      filtered.forEach(point => {
        // Safely access nested properties with optional chaining
        if (point.data?.cpus?.cpus?.[i]) {
          coreData.push([point.timestamp.getTime(), point.data.cpus.cpus[i].usage])
        }
      })

      if (coreData.length > 0) {
        const firstPoint = filtered[0]
        series.push({
          name: firstPoint?.data?.cpus?.cpus?.[i]?.name || `Core ${i}`,
          type: 'line',
          smooth: 0.6,
          data: coreData,
          lineStyle: { width: 2 },
          itemStyle: { opacity: 0 },
          areaStyle: { opacity: 0.1 },
        })
      }
    }

    // Calculate average with null checks
    const avgData: [number, number][] = filtered
      .filter(point => point.data?.cpus?.cpus)
      .map(point => {
        const cpus = point.data!.cpus.cpus
        const avg = cpus.reduce((sum, cpu) => sum + cpu.usage, 0) / cpus.length
        return [point.timestamp.getTime(), avg]
      })

    series.push({
      name: 'Average',
      type: 'line',
      smooth: 0.6,
      data: avgData,
      lineStyle: { width: 3, type: 'solid' },
      itemStyle: { opacity: 0 },
      areaStyle: { opacity: 0.05 },
    })

    // Calculate max CPU value for y-axis
    const allValues = filtered
      .flatMap(point => point.data?.cpus?.cpus?.map(cpu => cpu.usage) || [])
    const maxCpu = Math.max(100, ...allValues, 0)

    return {
      backgroundColor: 'transparent',
      textStyle: { color: '#9ca3af' },
      title: {
        text: 'CPU Usage (%)',
        textStyle: { color: '#f3f4f6', fontSize: 14, fontWeight: 'bold' },
        left: 'left',
      },
      grid: {
        top: 50,
        right: 50,
        bottom: 40,
        left: 50,
        containLabel: true,
      },
      tooltip: {
        trigger: 'axis',
        backgroundColor: 'rgba(0, 0, 0, 0.8)',
        borderColor: '#4b5563',
        textStyle: { color: '#fff' },
        valueFormatter: (value: number) => {
          const date = new Date(value as number)
          return date.toLocaleTimeString()
        },
      },
      xAxis: {
        type: 'time',
        min: windowStart,
        max: now,
        boundaryGap: false,
        axisLabel: {
          formatter: (value: number) => {
            const date = new Date(value)
            // Show time based on window size
            if (timeWindowSeconds <= 300) {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
            } else if (timeWindowSeconds <= 3600) {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            } else {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            }
          },
          color: '#6b7280',
        },
        axisLine: { lineStyle: { color: '#374151' } },
        splitLine: { 
          lineStyle: { color: '#1f2937' },
          // Show more grid lines for shorter time windows
          interval: timeWindowSeconds <= 300 ? 'auto' : timeWindowSeconds <= 900 ? 0 : 1,
        },
      },
      yAxis: {
        type: 'value',
        min: 0,
        max: maxCpu,
        name: '%',
        nameTextStyle: { color: '#6b7280' },
        axisLabel: { color: '#6b7280', formatter: (value: number) => `${value}%` },
        axisLine: { lineStyle: { color: '#374151' } },
        splitLine: { lineStyle: { color: '#1f2937' } },
      },
      legend: {
        bottom: 0,
        textStyle: { color: '#9ca3af' },
        borderColor: '#374151',
      },
      series,
    }
  }, [metrics, timeWindowSeconds])

  return (
    <div key={`cpu-chart-${serverId}`} className="card h-96">
      <ReactECharts
        option={chartOption}
        style={{ height: '100%' }}
        opts={{ renderer: 'canvas' }}
      />
    </div>
  )
}
