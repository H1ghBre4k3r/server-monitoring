import { useMemo } from 'react'
import ReactECharts from 'echarts-for-react'
import { useMonitoringStore } from '../../stores/monitoringStore'

interface TemperatureChartProps {
  serverId: string
}

export default function TemperatureChart({ serverId }: TemperatureChartProps) {
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

    // Get unique component names with null checks
    const componentNames = new Set<string>()
    filtered.forEach(point => {
      point.data?.components?.components?.forEach(component => {
        if (component.temperature !== null) {
          componentNames.add(component.name)
        }
      })
    })

    if (componentNames.size === 0) {
      return {
        title: { text: 'No temperature data available' },
        grid: { top: 50, right: 50, bottom: 50, left: 50 },
      }
    }

    // Create datasets for each component
    const series: any[] = []
    Array.from(componentNames).forEach((name, idx) => {
      const colors = ['#ef4444', '#f97316', '#eab308', '#84cc16', '#22c55e', '#06b6d4']
      const color = colors[idx % colors.length]

      const componentData: [number, number][] = filtered
        .map(point => {
          const component = point.data?.components?.components?.find(c => c.name === name)
          if (component && component.temperature !== null) {
            return [point.timestamp.getTime(), component.temperature] as [number, number]
          }
          return null
        })
        .filter((v): v is [number, number] => v !== null)

      if (componentData.length > 0) {
        series.push({
          name,
          type: 'line',
          smooth: 0.6,
          data: componentData,
          lineStyle: { width: 2, color },
          itemStyle: { opacity: 0 },
          areaStyle: { opacity: 0.1, color },
          emphasis: {
            lineStyle: { width: 3 },
            itemStyle: { opacity: 1 },
          },
        })
      }
    })

    return {
      backgroundColor: 'transparent',
      textStyle: { color: '#9ca3af' },
      title: {
        text: 'Temperature (°C)',
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
        formatter: (params: any) => {
          if (Array.isArray(params) && params.length > 0) {
            const date = new Date(params[0].axisValue as number)
            const items = params
              .map((p: any) => `${p.marker} ${p.name}: ${p.value[1].toFixed(1)}°C`)
              .join('<br/>')
            return `${date.toLocaleTimeString()}<br/>${items}`
          }
          return ''
        },
      },
      xAxis: {
        type: 'time',
        boundaryGap: false,
        axisLabel: {
          formatter: (value: number) => {
            const date = new Date(value)
            return date.toLocaleTimeString()
          },
          color: '#6b7280',
        },
        axisLine: { lineStyle: { color: '#374151' } },
        splitLine: { lineStyle: { color: '#1f2937' } },
      },
      yAxis: {
        type: 'value',
        name: '°C',
        nameTextStyle: { color: '#6b7280' },
        axisLabel: { color: '#6b7280', formatter: '{value}°' },
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
    <div className="card h-96">
      <ReactECharts
        option={chartOption}
        style={{ height: '100%' }}
        opts={{ renderer: 'canvas' }}
      />
    </div>
  )
}
