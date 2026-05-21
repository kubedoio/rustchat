// Theme Service - Business logic for appearance/theming

import { log } from '@/utils/log';
import { themeRepository } from '../repositories/themeRepository'
import type { Theme, ChatFont, ChatFontSize } from '../types'
import { THEME_OPTIONS, FONT_OPTIONS, FONT_SIZE_OPTIONS, DARK_THEMES } from '../types'
import { useThemeStore } from '../stores/themeStore'
import { getGlobalAuthToken } from '../../auth'

const STORAGE_THEME = 'theme'
const STORAGE_FONT = 'chat_font'
const STORAGE_FONT_SIZE = 'chat_font_size'

class ThemeService {
  private get store() {
    return useThemeStore()
  }

  // Initialize theme from storage and server
  async initialize(): Promise<void> {
    if (typeof window === 'undefined') return

    // Load from localStorage
    const savedTheme = localStorage.getItem(STORAGE_THEME)
    const savedFont = localStorage.getItem(STORAGE_FONT)
    const savedFontSize = localStorage.getItem(STORAGE_FONT_SIZE)

    if (savedTheme) {
      this.store.setTheme(this.normalizeTheme(savedTheme), false)
    } else {
      this.store.setTheme('light', false)
    }

    if (savedFont && this.isValidFont(savedFont)) {
      this.store.setFont(savedFont, false)
    } else {
      this.store.setFont('inter', false)
    }

    if (savedFontSize) {
      this.store.setFontSize(this.normalizeFontSize(savedFontSize), false)
    } else {
      this.store.setFontSize(14, false)
    }

    // Apply immediately
    this.applyAppearance()

    // Sync from server
    await this.syncFromServer()
  }

  // Set theme
  setTheme(theme: Theme | 'system'): void {
    const normalized = theme === 'system' 
      ? this.getSystemTheme() 
      : this.normalizeTheme(theme)

    this.store.setTheme(normalized)
    localStorage.setItem(STORAGE_THEME, normalized)
    this.applyTheme()
    void this.persistToServer()
  }

  // Set font
  setFont(font: ChatFont): void {
    if (!this.isValidFont(font)) return

    this.store.setFont(font)
    localStorage.setItem(STORAGE_FONT, font)
    this.applyTypography()
    void this.persistToServer()
  }

  // Set font size
  setFontSize(size: ChatFontSize): void {
    if (!FONT_SIZE_OPTIONS.includes(size)) return

    this.store.setFontSize(size)
    localStorage.setItem(STORAGE_FONT_SIZE, String(size))
    this.applyTypography()
    void this.persistToServer()
  }

  // Apply theme to DOM
  applyTheme(): void {
    if (typeof window === 'undefined') return

    const root = document.documentElement
    const theme = this.store.theme

    root.setAttribute('data-theme', theme)

    if (DARK_THEMES.has(theme)) {
      root.classList.add('dark')
    } else {
      root.classList.remove('dark')
    }
  }

  // Apply typography to DOM
  applyTypography(): void {
    if (typeof window === 'undefined') return

    const root = document.documentElement
    root.style.setProperty('--chat-font-family', `var(--font-${this.store.font})`)
    root.style.setProperty('--chat-font-size', `${this.store.fontSize}px`)
  }

  // Apply all appearance settings
  applyAppearance(): void {
    this.applyTheme()
    this.applyTypography()
  }

  // Sync from server
  async syncFromServer(force = false): Promise<void> {
    const token = getGlobalAuthToken()
    if (!token) return

    if (!force && this.store.syncedServerToken === token) return

    try {
      const prefs = await themeRepository.loadFromServer()

      if (prefs.theme && this.isValidTheme(prefs.theme)) {
        this.store.setTheme(prefs.theme)
        localStorage.setItem(STORAGE_THEME, prefs.theme)
      }

      if (prefs.font && this.isValidFont(prefs.font)) {
        this.store.setFont(prefs.font)
        localStorage.setItem(STORAGE_FONT, prefs.font)
      }

      if (prefs.fontSize && this.isValidFontSize(prefs.fontSize)) {
        this.store.setFontSize(prefs.fontSize)
        localStorage.setItem(STORAGE_FONT_SIZE, String(prefs.fontSize))
      }

      this.applyAppearance()
      this.store.setSyncedServerToken(token)
    } catch (error) {
      log.debug('Failed to sync theme from server', error)
    }
  }

  // Private helpers
  private async persistToServer(): Promise<void> {
    const token = getGlobalAuthToken()
    if (!token) return

    try {
      await themeRepository.saveToServer(this.store.theme, this.store.font, this.store.fontSize)
    } catch (error) {
      log.debug('Failed to persist theme to server', error)
    }
  }

  private normalizeTheme(value: string): Theme {
    if (value === 'system') {
      return this.getSystemTheme()
    }
    if (this.isValidTheme(value)) {
      return value
    }
    return 'light'
  }

  private normalizeFontSize(value: string): ChatFontSize {
    const parsed = Number(value)
    if (this.isValidFontSize(parsed)) {
      return parsed
    }
    return 14
  }

  private getSystemTheme(): Theme {
    if (typeof window === 'undefined') return 'light'
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }

  private isValidTheme(value: string): value is Theme {
    return THEME_OPTIONS.some(opt => opt.id === value)
  }

  private isValidFont(value: string): value is ChatFont {
    return FONT_OPTIONS.some(opt => opt.id === value)
  }

  private isValidFontSize(value: number): value is ChatFontSize {
    return FONT_SIZE_OPTIONS.includes(value as ChatFontSize)
  }
}

export const themeService = new ThemeService()
