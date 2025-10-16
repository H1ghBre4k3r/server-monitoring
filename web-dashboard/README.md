# Guardia Web Dashboard

A modern, interactive web dashboard for the Guardia server monitoring system. Built with React, TypeScript, and Vite.

## Features

- **Real-time Server Monitoring**: View CPU, memory, and temperature metrics
- **Service Health Checks**: Monitor HTTP/HTTPS endpoints with uptime tracking
- **Alert Timeline**: Track critical events and alerts
- **WebSocket Streaming**: Real-time updates without polling
- **Responsive Design**: Works on desktop and tablet
- **Dark Theme**: Professional, modern UI

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
└── index.css          # Global styles
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
- **Tailwind CSS** - Styling
- **ECharts** - Charts and visualization
- **Zustand** - State management
- **Lucide React** - Icons

## Performance

- Lazy-load ECharts library
- Debounce WebSocket updates
- Virtual scrolling for large lists
- Memoized components

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
4. Add TypeScript types
5. Test with hub API

## License

GPL-3.0 - Same as the main project
