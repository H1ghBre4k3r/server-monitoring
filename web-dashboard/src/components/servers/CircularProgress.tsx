interface CircularProgressProps {
  percentage: number
  size?: number
  strokeWidth?: number
  color: { from: string; to: string }
  label: string
  value: string
}

import styles from './CircularProgress.module.css'

export default function CircularProgress({
  percentage,
  size = 160,
  strokeWidth = 12,
  color,
  label,
  value
}: CircularProgressProps) {
  const radius = (size - strokeWidth) / 2
  const circumference = radius * 2 * Math.PI
  const offset = circumference - (percentage / 100) * circumference

  return (
    <div className={styles.container}>
      {/* SVG Circle */}
      <svg width={size} height={size} className={styles.svg}>
        {/* Background circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          stroke="rgba(var(--ctp-frappe-surface1-rgb), 0.3)"
          strokeWidth={strokeWidth}
          fill="none"
        />
        {/* Progress circle with gradient */}
        <defs>
          <linearGradient id={`gradient-${label}`} x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor={color.from} />
            <stop offset="100%" stopColor={color.to} />
          </linearGradient>
          <filter id={`glow-${label}`}>
            <feGaussianBlur stdDeviation="3" result="coloredBlur"/>
            <feMerge>
              <feMergeNode in="coloredBlur"/>
              <feMergeNode in="SourceGraphic"/>
            </feMerge>
          </filter>
        </defs>
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          stroke={`url(#gradient-${label})`}
          strokeWidth={strokeWidth}
          fill="none"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
          className={styles.progressCircle}
          filter={`url(#glow-${label})`}
          style={{ opacity: 0.9 }}
        />
      </svg>
      
      {/* Center content */}
      <div className={styles.centerContent}>
        <div className={styles.textContent}>
          <div className={styles.percentage}>
            {percentage.toFixed(1)}%
          </div>
          <div className={styles.label}>
            {label}
          </div>
          <div className={styles.value}>
            {value}
          </div>
        </div>
      </div>
    </div>
  )
}
