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

    // Color palette for cores
    const colorPalette = [
      ['#3b82f6', '#2563eb'], // Blue
      ['#8b5cf6', '#7c3aed'], // Purple
      ['#06b6d4', '#0891b2'], // Cyan
      ['#10b981', '#059669'], // Green
      ['#f59e0b', '#d97706'], // Amber
      ['#ef4444', '#dc2626'], // Red
      ['#ec4899', '#db2777'], // Pink
      ['#6366f1', '#4f46e5'], // Indigo
    ]

    return {
      backgroundColor: 'transparent',
      textStyle: { color: '#9ca3af', fontFamily: 'system-ui, -apple-system, sans-serif' },
      title: {
        text: 'CPU Usage Over Time',
        textStyle: { 
          color: '#f3f4f6', 
          fontSize: 16, 
          fontWeight: 'bold',
          fontFamily: 'system-ui, -apple-system, sans-serif'
        },
        left: 'center',
        top: 10,
      },
      grid: {
        top: 60,
        right: 60,
        bottom: 80,
        left: 60,
        containLabel: true,
      },
      tooltip: {
        trigger: 'axis',
        backgroundColor: 'rgba(17, 24, 39, 0.95)',
        borderColor: '#4b5563',
        borderWidth: 1,
        textStyle: { color: '#fff', fontSize: 12 },
        padding: [10, 15],
        axisPointer: {
          type: 'cross',
          lineStyle: { color: '#6366f1', width: 1, opacity: 0.5 },
          crossStyle: { color: '#6366f1', width: 1, opacity: 0.5 },
        },
        formatter: (params: any) => {
          if (!params || params.length === 0) return ''
          const date = new Date(params[0].value[0])
          let html = `<div style="font-weight: bold; margin-bottom: 8px; color: #a5b4fc;">${date.toLocaleTimeString()}</div>`
          params.forEach((param: any) => {
            const color = param.color?.colorStops?.[0]?.color || param.color
            html += `<div style="display: flex; align-items: center; margin: 4px 0;">
              <span style="display: inline-block; width: 10px; height: 10px; border-radius: 50%; background: ${color}; margin-right: 8px;"></span>
              <span style="flex: 1;">${param.seriesName}:</span>
              <span style="font-weight: bold; margin-left: 12px;">${param.value[1].toFixed(2)}%</span>
            </div>`
          })
          return html
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
            if (timeWindowSeconds <= 300) {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
            } else if (timeWindowSeconds <= 3600) {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            } else {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            }
          },
          color: '#6b7280',
          fontSize: 11,
        },
        axisLine: { lineStyle: { color: '#374151', width: 1 } },
        axisTick: { lineStyle: { color: '#4b5563' } },
        splitLine: { 
          lineStyle: { color: '#1f2937', type: 'dashed', opacity: 0.5 },
          interval: timeWindowSeconds <= 300 ? 'auto' : timeWindowSeconds <= 900 ? 0 : 1,
        },
      },
      yAxis: {
        type: 'value',
        min: 0,
        max: maxCpu,
        name: 'Usage (%)',
        nameTextStyle: { color: '#9ca3af', fontSize: 12, padding: [0, 0, 0, -10] },
        axisLabel: { color: '#6b7280', formatter: (value: number) => `${value}%`, fontSize: 11 },
        axisLine: { show: false },
        axisTick: { show: false },
        splitLine: { lineStyle: { color: '#1f2937', type: 'dashed', opacity: 0.5 } },
      },
      legend: {
        bottom: 10,
        left: 'center',
        textStyle: { color: '#9ca3af', fontSize: 11 },
        icon: 'circle',
        itemWidth: 12,
        itemHeight: 12,
        itemGap: 20,
        backgroundColor: 'rgba(31, 41, 55, 0.5)',
        borderRadius: 8,
        padding: [8, 20],
      },
      series: series.map((s, idx) => ({
        ...s,
        lineStyle: { 
          width: s.name === 'Average' ? 3 : 2,
          type: s.name === 'Average' ? 'solid' : 'solid',
        },
        smooth: 0.3,
        symbol: 'none',
        emphasis: {
          focus: 'series',
          lineStyle: { width: s.name === 'Average' ? 4 : 3 },
        },
        color: s.name === 'Average' 
          ? {
              type: 'linear',
              x: 0, y: 0, x2: 1, y2: 0,
              colorStops: [
                { offset: 0, color: '#a855f7' },
                { offset: 1, color: '#ec4899' }
              ]
            }
          : {
              type: 'linear',
              x: 0, y: 0, x2: 1, y2: 0,
              colorStops: [
                { offset: 0, color: colorPalette[idx % colorPalette.length][0] },
                { offset: 1, color: colorPalette[idx % colorPalette.length][1] }
              ]
            },
        areaStyle: s.name === 'Average' 
          ? {
              color: {
                type: 'linear',
                x: 0, y: 0, x2: 0, y2: 1,
                colorStops: [
                  { offset: 0, color: 'rgba(168, 85, 247, 0.15)' },
                  { offset: 1, color: 'rgba(168, 85, 247, 0.01)' }
                ]
              }
            }
          : {
              color: {
                type: 'linear',
                x: 0, y: 0, x2: 0, y2: 1,
                colorStops: [
                  { offset: 0, color: `${colorPalette[idx % colorPalette.length][0]}33` },
                  { offset: 1, color: `${colorPalette[idx % colorPalette.length][0]}01` }
                ]
              }
            },
      })),
    }
  }, [metrics, timeWindowSeconds])

  return (
    <div key={`cpu-chart-${serverId}`} className="card-premium" style={{ height: '450px' }}>
      <ReactECharts
        option={chartOption}
        style={{ height: '100%' }}
        opts={{ renderer: 'canvas' }}
      />
    </div>
  )
}
