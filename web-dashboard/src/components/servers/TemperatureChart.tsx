import { useMemo } from 'react'
import ReactECharts from 'echarts-for-react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import styles from './TemperatureChart.module.css'

interface TemperatureChartProps {
  serverId: string
  currentMetrics?: any
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

    // Sort by timestamp to ensure proper time series
    filtered.sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime())

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

    // Temperature-based color gradients
    const getTempColor = (temp: number): [string, string] => {
      if (temp >= 80) return ['#ef4444', '#dc2626'] // Red - Critical
      if (temp >= 70) return ['#f97316', '#ea580c'] // Orange - High
      if (temp >= 60) return ['#f59e0b', '#d97706'] // Amber - Elevated
      if (temp >= 50) return ['#eab308', '#ca8a04'] // Yellow - Warm
      if (temp >= 40) return ['#84cc16', '#65a30d'] // Lime - Moderate
      return ['#22c55e', '#16a34a'] // Green - Cool
    }

    // Create datasets for each component with dynamic colors
    const series: any[] = []
    const componentArray = Array.from(componentNames)
    
    componentArray.forEach((name) => {
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
        // Get average temp for this component to determine base color
        const avgTemp = componentData.reduce((sum, [, temp]) => sum + temp, 0) / componentData.length
        const [color1, color2] = getTempColor(avgTemp)

        series.push({
          name,
          type: 'line',
          smooth: 0.3,
          data: componentData,
          lineStyle: { width: 2.5 },
          symbol: 'none',
          emphasis: {
            focus: 'series',
            lineStyle: { width: 3.5 },
          },
          color: {
            type: 'linear',
            x: 0, y: 0, x2: 1, y2: 0,
            colorStops: [
              { offset: 0, color: color1 },
              { offset: 1, color: color2 }
            ]
          },
          areaStyle: {
            color: {
              type: 'linear',
              x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [
                { offset: 0, color: `${color1}33` },
                { offset: 1, color: `${color1}01` }
              ]
            }
          },
        })
      }
    })

    // Detect mobile viewport
    const isMobile = typeof window !== 'undefined' && window.innerWidth < 640
    const isTablet = typeof window !== 'undefined' && window.innerWidth < 1024

    return {
      backgroundColor: 'transparent',
      textStyle: { color: '#9ca3af', fontFamily: 'system-ui, -apple-system, sans-serif' },
      grid: {
        top: isMobile ? 20 : 30,
        right: isMobile ? 15 : isTablet ? 30 : 60,
        bottom: isMobile ? 45 : 80,
        left: isMobile ? 10 : isTablet ? 30 : 60,
        containLabel: true,
      },
      tooltip: {
        trigger: 'axis',
        backgroundColor: 'rgba(17, 24, 39, 0.98)',
        borderColor: '#4b5563',
        borderWidth: 1,
        textStyle: { color: '#fff', fontSize: isMobile ? 10 : 12 },
        padding: isMobile ? [8, 10] : [10, 15],
        confine: true,
        axisPointer: {
          type: 'cross',
          lineStyle: { color: '#f97316', width: 1, opacity: 0.5 },
          crossStyle: { color: '#f97316', width: 1, opacity: 0.5 },
        },
        formatter: (params: any) => {
          if (!params || params.length === 0) return ''
          const date = new Date(params[0].value[0])
          let html = `<div style="font-weight: bold; margin-bottom: ${isMobile ? 4 : 8}px; color: #fdba74; font-size: ${isMobile ? 10 : 12}px;">${date.toLocaleTimeString()}</div>`
          
          // Limit displayed items on mobile
          const displayParams = isMobile ? params.slice(0, 3) : params
          
          displayParams.forEach((param: any) => {
            const temp = param.value[1]
            const color = param.color?.colorStops?.[0]?.color || param.color
            const tempClass = temp >= 70 ? '#ef4444' : temp >= 50 ? '#f59e0b' : '#22c55e'
            html += `<div style="display: flex; align-items: center; margin: ${isMobile ? 2 : 4}px 0;">
              <span style="display: inline-block; width: ${isMobile ? 6 : 10}px; height: ${isMobile ? 6 : 10}px; border-radius: 50%; background: ${color}; margin-right: ${isMobile ? 4 : 8}px;"></span>
              <span style="flex: 1; font-size: ${isMobile ? 9 : 11}px;">${param.seriesName}:</span>
              <span style="font-weight: bold; margin-left: ${isMobile ? 6 : 12}px; color: ${tempClass}; font-size: ${isMobile ? 10 : 12}px;">${temp.toFixed(1)}°C</span>
            </div>`
          })
          
          if (isMobile && params.length > 3) {
            html += `<div style="margin-top: 4px; color: #9ca3af; font-size: 9px; text-align: center;">+${params.length - 3} more</div>`
          }
          
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
            if (isMobile) {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            }
            if (timeWindowSeconds <= 300) {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
            } else if (timeWindowSeconds <= 3600) {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            } else {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            }
          },
          color: '#6b7280',
          fontSize: isMobile ? 9 : 11,
          rotate: isMobile ? 45 : 0,
          hideOverlap: true,
        },
        axisLine: { lineStyle: { color: '#374151', width: 1 } },
        axisTick: { lineStyle: { color: '#4b5563' }, show: !isMobile },
        splitLine: { 
          lineStyle: { color: '#1f2937', type: 'dashed', opacity: 0.5 },
          show: !isMobile,
        },
      },
      yAxis: {
        type: 'value',
        name: isMobile ? '' : 'Temperature (°C)',
        nameTextStyle: { color: '#9ca3af', fontSize: isMobile ? 10 : 12, padding: [0, 0, 0, -10] },
        axisLabel: { 
          color: '#6b7280', 
          formatter: (value: number) => isMobile ? `${value}°` : `${value}°C`, 
          fontSize: isMobile ? 9 : 11 
        },
        axisLine: { show: false },
        axisTick: { show: false },
        splitLine: { lineStyle: { color: '#1f2937', type: 'dashed', opacity: 0.3 } },
      },
      legend: {
        show: !isMobile,
        bottom: 10,
        left: 'center',
        textStyle: { color: '#9ca3af', fontSize: isTablet ? 10 : 11 },
        icon: 'circle',
        itemWidth: isTablet ? 10 : 12,
        itemHeight: isTablet ? 10 : 12,
        itemGap: isTablet ? 12 : 20,
        backgroundColor: 'rgba(31, 41, 55, 0.5)',
        borderRadius: 8,
        padding: isTablet ? [6, 15] : [8, 20],
      },
      series,
    }
  }, [metrics, timeWindowSeconds])

  return (
    <div key={`temp-chart-${serverId}`} className={styles.chartContainer}>
      <ReactECharts
        option={chartOption}
        style={{ height: '100%' }}
        opts={{ renderer: 'canvas' }}
      />
    </div>
  )
}
