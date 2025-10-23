import { useEffect } from 'react'

export function useKeyboardShortcuts() {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Ctrl/Cmd + Shift + T for quick theme toggle
      if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key === 'T') {
        event.preventDefault()
        cycleTheme()
      }
      
      // Ctrl/Cmd + Shift + L for light/dark quick toggle
      if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key === 'L') {
        event.preventDefault()
        toggleLightDark()
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [])
}

function cycleTheme() {
  const themes = ['frappe', 'latte', 'macchiato', 'mocha'] as const
  const currentTheme = document.documentElement.getAttribute('data-theme') as typeof themes[number]
  const currentIndex = themes.indexOf(currentTheme || 'frappe')
  const nextIndex = (currentIndex + 1) % themes.length
  const nextTheme = themes[nextIndex]
  
  document.documentElement.setAttribute('data-theme', nextTheme)
  localStorage.setItem('theme', nextTheme)
}

function toggleLightDark() {
  const currentTheme = document.documentElement.getAttribute('data-theme') || 'frappe'
  const isLight = currentTheme === 'latte'
  
  if (isLight) {
    // Switch to a dark theme (prefer frappe)
    document.documentElement.setAttribute('data-theme', 'frappe')
    localStorage.setItem('theme', 'frappe')
  } else {
    // Switch to light theme
    document.documentElement.setAttribute('data-theme', 'latte')
    localStorage.setItem('theme', 'latte')
  }
}

export function getThemeInfo() {
  const theme = document.documentElement.getAttribute('data-theme') || 'frappe'
  const themeNames: Record<string, string> = {
    frappe: 'Catppuccin Frappe',
    latte: 'Catppuccin Latte',
    macchiato: 'Catppuccin Macchiato',
    mocha: 'Catppuccin Mocha'
  }
  
  return {
    current: theme,
    name: themeNames[theme] || 'Unknown',
    isLight: theme === 'latte',
    isDark: theme !== 'latte'
  }
}