/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Custom colors for monitoring theme
        'status-up': '#10b981',      // Green
        'status-down': '#ef4444',    // Red
        'status-stale': '#f59e0b',   // Amber
        'status-degraded': '#f97316', // Orange
        'status-unknown': '#6b7280', // Gray
      },
    },
  },
  plugins: [],
}
