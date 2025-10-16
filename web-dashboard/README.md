# Guardia Web Dashboard

A **modern, elegant, and visually stunning** web dashboard for the Guardia server monitoring system. Built with React, TypeScript, and Vite.

## ✨ Design Features

The dashboard features a **completely redesigned UI** with:

- 🎨 **Glassmorphism** - Translucent cards with backdrop blur effects
- 🌈 **Dynamic Gradients** - Beautiful color transitions throughout
- ✨ **Smooth Animations** - Fade-ins, slides, and micro-interactions
- 💫 **Glow Effects** - Subtle shadows and highlights for depth
- 📊 **Professional Charts** - Enhanced ECharts visualizations with gradients
- 🎯 **Color-Coded Metrics** - Temperature and usage-based color schemes
- 🔄 **Pulsing Status Indicators** - Live connection and health status
- 🖼️ **Ambient Backgrounds** - Animated gradient orbs for atmosphere

> See [DESIGN_IMPROVEMENTS.md](./DESIGN_IMPROVEMENTS.md) for complete design documentation.

## Features

- **Real-time Server Monitoring**: View CPU, memory, and temperature metrics with live charts
- **Service Health Checks**: Monitor HTTP/HTTPS endpoints with uptime tracking
- **Alert Timeline**: Track critical events and alerts with severity indicators
- **WebSocket Streaming**: Real-time updates without polling
- **Time Window Selection**: 1m, 5m, 15m, 30m, 1h views
- **Responsive Design**: Works on desktop and tablet
- **Modern Dark Theme**: Professional glassmorphic UI with gradients

## Development

### Prerequisites

- Node.js 18+
- npm 9+

### Quick Start

```bash
# Install dependencies
npm install

# Start development server (runs on http://localhost:5173)
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

### Environment

The dashboard connects to the API server via proxy during development. Set `api_url` via environment or proxy configuration in `vite.config.ts`.

Default: `http://localhost:8080` (hub API)

## Project Structure

```
src/
├── api/
│   ├── types.ts       # TypeScript definitions matching Rust API
│   └── client.ts      # HTTP + WebSocket client
├── stores/
│   └── monitoringStore.ts  # Zustand state management
├── components/
│   ├── layout/        # Layout, header, sidebar
│   ├── servers/       # Server list, detail, charts
│   ├── services/      # Service list
│   └── alerts/        # Alert timeline
├── hooks/             # Custom React hooks
├── lib/               # Utilities
├── App.tsx            # Main app component
├── main.tsx           # Entry point
└── index.css          # Global styles with custom animations
```

## Building for Hub

The dashboard is built as a static bundle and served by the hub via Axum. Build outputs go to `dist/`.

```bash
# Production build
npm run build

# Files are in dist/ - ready to be served by hub
```

## Configuration

### API URL

During development, the dev server proxies API requests to `http://localhost:8080` (configurable in `vite.config.ts`).

In production, the hub serves the dashboard from `/` and API from `/api/v1/`.

### Authentication

If the hub has an API token configured, pass it via:

1. URL query parameter: `?token=your-token`
2. Local storage: Store in `monitoring_token` key
3. Hard-coded in environment (not recommended)

## Technologies

- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool and dev server
- **Tailwind CSS** - Styling with custom utilities
- **ECharts** - Charts and visualization with gradient support
- **Zustand** - Lightweight state management
- **Lucide React** - Beautiful icon set

## Design System

### Custom CSS Classes

```css
/* Cards */
.card            /* Enhanced glassmorphic card */
.card-premium    /* Card with animated gradient border */
.stat-card       /* Metric cards with hover effects */

/* Badges */
.badge-up        /* Green gradient with glow */
.badge-down      /* Red gradient with glow */
.badge-stale     /* Yellow gradient with glow */
.badge-degraded  /* Orange gradient with glow */

/* Progress */
.progress-bar    /* Enhanced container with shadow */
.progress-fill   /* Gradient-filled progress with glow */

/* Animations */
.animate-fade-in    /* Fade in content */
.animate-slide-up   /* Slide up with fade */
.animate-scale-in   /* Scale in with fade */
.animate-pulse-glow /* Pulsing glow effect */
```

### Color Palette

- **Primary**: Blue to Indigo gradients
- **Success**: Green to Emerald gradients
- **Warning**: Yellow to Amber gradients
- **Danger**: Red to Rose gradients
- **Info**: Cyan to Teal gradients
- **Accent**: Purple to Pink gradients

## Performance

- Lazy-load ECharts library
- GPU-accelerated CSS animations
- Memoized components and chart options
- Efficient WebSocket updates
- Canvas rendering for charts

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

## Contributing

When adding new features:

1. Keep types in sync with Rust API (`src/api/types.ts`)
2. Use Zustand for state management
3. Follow the component structure
4. Follow the design system (glassmorphism, gradients, animations)
5. Add TypeScript types
6. Test with hub API
7. Ensure accessibility

## License

GPL-3.0 - Same as the main project
