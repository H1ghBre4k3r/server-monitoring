import { useState, useEffect } from 'react'
import { Sun, Moon, Monitor, Palette, Check } from 'lucide-react'
import styles from './ThemeSwitcher.module.css'

type Theme = 'frappe' | 'latte' | 'macchiato' | 'mocha'
type ThemeVariant = 'dark' | 'light'

interface ThemeConfig {
  name: string
  variant: ThemeVariant
  icon: React.ReactNode
  description: string
}

const themes: Record<Theme, ThemeConfig> = {
  frappe: {
    name: 'Catppuccin Frappe',
    variant: 'dark',
    icon: <Moon size={16} />,
    description: 'Dark theme with cool tones'
  },
  latte: {
    name: 'Catppuccin Latte',
    variant: 'light',
    icon: <Sun size={16} />,
    description: 'Light theme with warm tones'
  },
  macchiato: {
    name: 'Catppuccin Macchiato',
    variant: 'dark',
    icon: <Moon size={16} />,
    description: 'Dark theme with medium contrast'
  },
  mocha: {
    name: 'Catppuccin Mocha',
    variant: 'dark',
    icon: <Moon size={16} />,
    description: 'Darkest theme with high contrast'
  }
}

interface ThemeSwitcherProps {
  variant?: 'sidebar' | 'header'
}

export function ThemeSwitcher({ variant = 'header' }: ThemeSwitcherProps = {}) {
  const [currentTheme, setCurrentTheme] = useState<Theme>('frappe')
  const [isOpen, setIsOpen] = useState(false)

  // Load saved theme on mount
  useEffect(() => {
    const savedTheme = localStorage.getItem('theme') as Theme
    if (savedTheme && themes[savedTheme]) {
      setCurrentTheme(savedTheme)
      document.documentElement.setAttribute('data-theme', savedTheme)
    } else {
      // Auto-detect system preference
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
      const defaultTheme = prefersDark ? 'frappe' : 'latte'
      setCurrentTheme(defaultTheme)
      document.documentElement.setAttribute('data-theme', defaultTheme)
    }
  }, [])

  // Apply theme to document
  const applyTheme = (theme: Theme) => {
    setCurrentTheme(theme)
    document.documentElement.setAttribute('data-theme', theme)
    localStorage.setItem('theme', theme)
    setIsOpen(false)
  }

  // Auto-detect system theme
  const useSystemTheme = () => {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
    const systemTheme = prefersDark ? 'frappe' : 'latte'
    applyTheme(systemTheme)
  }

  // Get themes by variant
  const lightThemes = Object.entries(themes).filter(([, config]) => config.variant === 'light')
  const darkThemes = Object.entries(themes).filter(([, config]) => config.variant === 'dark')

  return (
    <div className={`${styles.themeSwitcher} ${styles[variant]}`}>
      <button
        className={styles.themeButton}
        onClick={() => setIsOpen(!isOpen)}
        title="Change theme"
      >
        <Palette size={18} />
        <span className={styles.themeLabel}>Theme</span>
      </button>

      {isOpen && (
        <>
          <div className={styles.backdrop} onClick={() => setIsOpen(false)} />
          <div className={`${styles.dropdown} ${styles[variant]}`}>
            <div className={styles.header}>
              <div className={styles.title}>
                <Monitor size={16} />
                <span>Theme Selection</span>
              </div>
              <button
                className={styles.systemButton}
                onClick={useSystemTheme}
                title="Use system preference"
              >
                <Monitor size={14} />
                <span>Auto</span>
              </button>
            </div>

            <div className={styles.themeSections}>
              {/* Light Themes */}
              <div className={styles.section}>
                <div className={styles.sectionTitle}>
                  <Sun size={12} />
                  <span>Light</span>
                </div>
                <div className={styles.themeGrid}>
                  {lightThemes.map(([key, config]) => (
                    <button
                      key={key}
                      className={`${styles.themeOption} ${
                        currentTheme === key ? styles.active : ''
                      }`}
                      onClick={() => applyTheme(key as Theme)}
                      title={config.description}
                    >
                      <div className={styles.themePreview}>
                        {config.icon}
                      </div>
                      <div className={styles.themeInfo}>
                        <div className={styles.themeName}>{config.name}</div>
                        <div className={styles.themeDescription}>{config.description}</div>
                      </div>
                      {currentTheme === key && (
                        <div className={styles.checkmark}>
                          <Check size={12} />
                        </div>
                      )}
                    </button>
                  ))}
                </div>
              </div>

              {/* Dark Themes */}
              <div className={styles.section}>
                <div className={styles.sectionTitle}>
                  <Moon size={12} />
                  <span>Dark</span>
                </div>
                <div className={styles.themeGrid}>
                  {darkThemes.map(([key, config]) => (
                    <button
                      key={key}
                      className={`${styles.themeOption} ${
                        currentTheme === key ? styles.active : ''
                      }`}
                      onClick={() => applyTheme(key as Theme)}
                      title={config.description}
                    >
                      <div className={styles.themePreview}>
                        {config.icon}
                      </div>
                      <div className={styles.themeInfo}>
                        <div className={styles.themeName}>{config.name}</div>
                        <div className={styles.themeDescription}>{config.description}</div>
                      </div>
                      {currentTheme === key && (
                        <div className={styles.checkmark}>
                          <Check size={12} />
                        </div>
                      )}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className={styles.footer}>
              <div className={styles.currentTheme}>
                Current: {themes[currentTheme].name}
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  )
}