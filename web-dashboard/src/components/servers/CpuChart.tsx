import { useMemo } from 'react'
import ReactECharts from 'echarts-for-react'
import { useMonitoringStore } from '../../stores/monitoringStore'
import { Cpu } from 'lucide-react'
import styles from './CpuChart.module.css'

interface CpuChartProps {
  serverId: string
  currentMetrics?: {
    cpus: {
      cpus: Array<{ name: string; usage: number }>
    }
  }
}

export default function CpuChart({ serverId, currentMetrics }: CpuChartProps) {
  const { metricsHistory, timeWindowSeconds } = useMonitoringStore()
  const metrics = metricsHistory.get(serverId) || []

  const getCpuColor = (usage: number) => {
    if (usage >= 80) return { from: '#e78284', to: '#ea999c', light: '#b5bfe2' }
    if (usage >= 60) return { from: '#ef9f76', to: '#f5a97f', light: '#b5bfe2' }
    return { from: '#8caaee', to: '#85c1dc', light: '#b5bfe2' }
  }

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

    // Color palette for cores using Catppuccin colors
    const colorPalette = [
      [ '#8caaee', '#85c1dc' ], // Blue
      [ '#ca9ee6', '#f4b8e4' ], // Mauve
      [ '#81c8be', '#99d1db' ], // Teal
      [ '#a6d189', '#a6da95' ], // Green
      [ '#e5c890', '#eed49f' ], // Yellow
      [ '#e78284', '#ea999c' ], // Red
      [ '#f4b8e4', '#f2d5cf' ], // Pink
      [ '#babbf1', '#b7bdf8' ], // Lavender
    ]

    // Detect mobile viewport
    const isMobile = typeof window !== 'undefined' && window.innerWidth < 640
    const isTablet = typeof window !== 'undefined' && window.innerWidth < 1024

    return {
      backgroundColor: 'transparent',
      textStyle: { color: '#b5bfe2', fontFamily: 'system-ui, -apple-system, sans-serif' },
      title: {
        text: isMobile ? 'CPU Usage' : 'CPU Usage Over Time',
        textStyle: { 
          color: '#c6d0f5', 
          fontSize: isMobile ? 13 : 16, 
          fontWeight: 'bold',
          fontFamily: 'system-ui, -apple-system, sans-serif'
        },
        left: 'center',
        top: isMobile ? 5 : 10,
      },
      grid: {
        top: isMobile ? 40 : 60,
        right: isMobile ? 15 : isTablet ? 30 : 60,
        bottom: isMobile ? 50 : 80,
        left: isMobile ? 10 : isTablet ? 30 : 60,
        containLabel: true,
      },
      tooltip: {
        trigger: 'axis',
        backgroundColor: 'rgba(41, 44, 60, 0.98)',
        borderColor: '#838ba7',
        borderWidth: 1,
        textStyle: { color: '#c6d0f5', fontSize: isMobile ? 10 : 12 },
        padding: isMobile ? [8, 10] : [10, 15],
        confine: true, // Keep tooltip within chart bounds on mobile
        axisPointer: {
          type: 'cross',
          lineStyle: { color: '#babbf1', width: 1, opacity: 0.5 },
          crossStyle: { color: '#babbf1', width: 1, opacity: 0.5 },
        },
        formatter: (params: any) => {
          if (!params || params.length === 0) return ''
          const date = new Date(params[0].value[0])
          let html = `<div style="font-weight: bold; margin-bottom: ${isMobile ? 4 : 8}px; color: #babbf1; font-size: ${isMobile ? 10 : 12}px;">${date.toLocaleTimeString()}</div>`
          
          // Limit to top 3 cores on mobile
          const displayParams = isMobile ? params.slice(0, 4) : params
          
          displayParams.forEach((param: any) => {
            const color = param.color?.colorStops?.[0]?.color || param.color
            html += `<div style="display: flex; align-items: center; margin: ${isMobile ? 2 : 4}px 0;">
              <span style="display: inline-block; width: ${isMobile ? 6 : 10}px; height: ${isMobile ? 6 : 10}px; border-radius: 50%; background: ${color}; margin-right: ${isMobile ? 4 : 8}px;"></span>
              <span style="flex: 1; font-size: ${isMobile ? 9 : 11}px;">${param.seriesName}:</span>
              <span style="font-weight: bold; margin-left: ${isMobile ? 6 : 12}px; font-size: ${isMobile ? 10 : 12}px;">${param.value[1].toFixed(1)}%</span>
            </div>`
          })
          
          if (isMobile && params.length > 4) {
            html += `<div style="margin-top: 4px; color: #b5bfe2; font-size: 9px; text-align: center;">+${params.length - 4} more</div>`
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
              // Mobile: show only time without seconds
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
          color: '#838ba7',
          fontSize: isMobile ? 9 : 11,
          rotate: isMobile ? 45 : 0,
          hideOverlap: true,
        },
        axisLine: { lineStyle: { color: '#414559', width: 1 } },
        axisTick: { lineStyle: { color: '#838ba7' }, show: !isMobile },
        splitLine: { 
          lineStyle: { color: '#414559', type: 'dashed', opacity: 0.5 },
          show: !isMobile,
        },
      },
      yAxis: {
        type: 'value',
        min: 0,
        max: maxCpu,
        name: isMobile ? '' : 'Usage (%)',
        nameTextStyle: { color: '#b5bfe2', fontSize: isMobile ? 10 : 12, padding: [0, 0, 0, -10] },
        axisLabel: { 
          color: '#838ba7', 
          formatter: (value: number) => isMobile ? `${value}` : `${value}%`, 
          fontSize: isMobile ? 9 : 11 
        },
        axisLine: { show: false },
        axisTick: { show: false },
        splitLine: { lineStyle: { color: '#414559', type: 'dashed', opacity: 0.3 } },
      },
      legend: {
        show: !isMobile, // Hide legend on mobile
        bottom: 10,
        left: 'center',
        textStyle: { color: '#b5bfe2', fontSize: isTablet ? 10 : 11 },
        icon: 'circle',
        itemWidth: isTablet ? 10 : 12,
        itemHeight: isTablet ? 10 : 12,
        itemGap: isTablet ? 12 : 20,
        backgroundColor: 'rgba(65, 69, 89, 0.5)',
        borderRadius: 8,
        padding: isTablet ? [6, 15] : [8, 20],
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
                { offset: 0, color: '#ca9ee6' },
                { offset: 1, color: '#f4b8e4' }
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
                  { offset: 0, color: 'rgba(202, 158, 230, 0.15)' },
                  { offset: 1, color: 'rgba(202, 158, 230, 0.01)' }
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
    <div className={styles.container}>
      {/* CPU Core Status - Responsive Layout */}
      {currentMetrics && (
        <div className={styles.coreStatusContainer}>
          <div className={styles.coreStatus}>
            <div className={styles.coreStatusHeader}>
              <div className={styles.coreStatusIconContainer}>
                <Cpu className={styles.coreStatusIcon} />
              </div>
              <span className={styles.coreStatusLabel}>Live CPU Cores</span>
            </div>
            <div className={styles.coreList}>
              {currentMetrics.cpus.cpus.map((cpu, idx) => {
                const colors = getCpuColor(cpu.usage)
                return (
                  <div key={idx} className={styles.coreItem}>
                    <div className={styles.coreItemHeader}>
                      <span className={styles.coreItemName} title={cpu.name}>
                        {cpu.name}
                      </span>
                      <span 
                        className={styles.coreItemValue}
                        style={{ 
                          background: `linear-gradient(90deg, ${colors.from}33, ${colors.to}33)`,
                          color: colors.light,
                          border: `1px solid ${colors.from}55`
                        }}
                      >
                        {cpu.usage.toFixed(1)}%
                      </span>
                    </div>
                    <div className={styles.coreProgressBar}>
                      <div
                        className={styles.coreProgressFill}
                        style={{
                          width: `${cpu.usage}%`,
                          background: `linear-gradient(90deg, ${colors.from}, ${colors.to})`,
                          boxShadow: `0 0 8px ${colors.from}88`
                        }}
                      />
                    </div>
                  </div>
                )
              })}
            </div>
          </div>
        </div>
      )}
      
      {/* Chart - Responsive Height */}
      <div key={`cpu-chart-${serverId}`} className={styles.chartContainer}>
        <ReactECharts
          option={chartOption}
          style={{ height: '100%' }}
          opts={{ renderer: 'canvas' }}
        />
      </div>
    </div>
  )
}