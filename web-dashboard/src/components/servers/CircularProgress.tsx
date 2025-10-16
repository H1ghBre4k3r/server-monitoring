interface CircularProgressProps {
  percentage: number
  size?: number
  strokeWidth?: number
  color: { from: string; to: string }
  label: string
  value: string
}

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
    <div className="relative inline-flex items-center justify-center">
      {/* SVG Circle */}
      <svg width={size} height={size} className="transform -rotate-90">
        {/* Background circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          stroke="rgba(55, 65, 81, 0.3)"
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
          className="transition-all duration-1000 ease-out"
          filter={`url(#glow-${label})`}
          style={{ opacity: 0.9 }}
        />
      </svg>
      
      {/* Center content */}
      <div className="absolute inset-0 flex flex-col items-center justify-center px-2">
        <div className="text-center">
          <div className="text-xl sm:text-2xl font-bold text-white mb-0.5 sm:mb-1">
            {percentage.toFixed(1)}%
          </div>
          <div className="text-[10px] sm:text-xs text-gray-400 uppercase tracking-wider font-semibold mb-0.5 sm:mb-1">
            {label}
          </div>
          <div className="text-[10px] sm:text-xs font-medium text-gray-300 truncate">
            {value}
          </div>
        </div>
      </div>
    </div>
  )
}
