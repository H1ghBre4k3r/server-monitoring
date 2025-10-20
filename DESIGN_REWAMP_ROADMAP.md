# Design Revamp Roadmap

This document outlines the plan to refactor the web dashboard's UI to a flatter, Catppuccin-themed design using plain CSS instead of Tailwind CSS.

### Phase 1: Foundational Setup

1.  **Integrate Catppuccin Palette:** Define all four Catppuccin color variants (`frappe`, `latte`, `macchiato`, `mocha`) as CSS custom properties in `web-dashboard/src/index.css`. The `frappe` theme will be applied as the default.
2.  **Initial Reset & Global Styles:** Create a simple CSS reset to normalize browser styles and define global styles for fonts, backgrounds, and text colors using the new custom properties.
3.  **Remove Tailwind Imports:** Remove the `@tailwind` directives from `web-dashboard/src/index.css`.

### Phase 2: Component Refactoring

This phase involves migrating components from Tailwind CSS to plain CSS, one by one. This iterative approach ensures the application remains functional throughout the process.

1.  **Layout Components:**
    *   `Layout.tsx`
    *   `Header.tsx`
    *   `Sidebar.tsx`
2.  **Server Components:**
    *   `ServerList.tsx`
    *   `ServerDetail.tsx`
    *   `CpuChart.tsx` and `TemperatureChart.tsx` (adjusting chart options to use new theme colors).
3.  **Service & Alert Components:**
    *   `ServiceList.tsx`
    *   `AlertTimeline.tsx`

### Phase 3: Cleanup

1.  **Remove Dependencies:** Uninstall Tailwind CSS and its related dependencies (`tailwindcss`, `postcss`, `autoprefixer`) from `package.json`.
2.  **Delete Configuration:** Remove the configuration files: `tailwind.config.js` and `postcss.config.js`.

### Phase 4: Final Polish

1.  **Review and Refine:** Conduct a final review of the entire UI to ensure consistency, responsiveness, and adherence to the flat design principle.
2.  **Add Theme Switching (Optional):** With the CSS custom properties in place, a theme-switching mechanism can be easily implemented in the future.
