// Theme boot script — runs synchronously from index.html <head> BEFORE the
// app and its CSS load, to avoid a flash of the wrong theme (FOUC). It must
// stay a separate plain-JS file in public/ (served same-origin, no build
// step, no imports): the production Content-Security-Policy is
// script-src 'self' with no 'unsafe-inline', so inline <script> blocks in
// index.html are not allowed.
//
// Keep the theme/font allow-lists in sync with THEME_OPTIONS in
// src/features/theme/stores/themeStore.ts.
;(function () {
  const allowedThemes = new Set([
    'light',
    'dark',
    'modern',
    'metallic',
    'futuristic',
    'high-contrast',
    'simple',
    'dynamic',
  ])
  const darkThemes = new Set(['dark', 'futuristic', 'high-contrast', 'dynamic'])

  // Theme color definitions (must match THEME_OPTIONS in theme.ts)
  const themeColors = {
    light: {
      sidebarBg: '#1e325c',
      sidebarText: '#ffffff',
      centerChannelBg: '#ffffff',
      centerChannelColor: '#3d3c40',
      linkColor: '#166de0',
      buttonBg: '#166de0',
      buttonColor: '#ffffff',
    },
    dark: {
      sidebarBg: '#1f222a',
      sidebarText: '#ffffff',
      centerChannelBg: '#0b1220',
      centerChannelColor: '#dddddd',
      linkColor: '#38bdf8',
      buttonBg: '#38bdf8',
      buttonColor: '#ffffff',
    },
    modern: {
      sidebarBg: '#1a1d24',
      sidebarText: '#e2e8f0',
      centerChannelBg: '#f3f7f6',
      centerChannelColor: '#0f172a',
      linkColor: '#0f766e',
      buttonBg: '#0f766e',
      buttonColor: '#ffffff',
    },
    metallic: {
      sidebarBg: '#334155',
      sidebarText: '#f1f5f9',
      centerChannelBg: '#e7eaee',
      centerChannelColor: '#1e293b',
      linkColor: '#d97706',
      buttonBg: '#475569',
      buttonColor: '#ffffff',
    },
    futuristic: {
      sidebarBg: '#0f172a',
      sidebarText: '#22c55e',
      centerChannelBg: '#030712',
      centerChannelColor: '#06b6d4',
      linkColor: '#22c55e',
      buttonBg: '#06b6d4',
      buttonColor: '#000000',
    },
    'high-contrast': {
      sidebarBg: '#000000',
      sidebarText: '#ffffff',
      centerChannelBg: '#000000',
      centerChannelColor: '#ffffff',
      linkColor: '#00e5ff',
      buttonBg: '#00e5ff',
      buttonColor: '#000000',
    },
    simple: {
      sidebarBg: '#44403c',
      sidebarText: '#fafaf9',
      centerChannelBg: '#fafaf9',
      centerChannelColor: '#292524',
      linkColor: '#0369a1',
      buttonBg: '#16a34a',
      buttonColor: '#ffffff',
    },
    dynamic: {
      sidebarBg: '#1f2937',
      sidebarText: '#f9fafb',
      centerChannelBg: '#111827',
      centerChannelColor: '#e5e7eb',
      linkColor: '#e11d48',
      buttonBg: '#f59e0b',
      buttonColor: '#000000',
    },
  }

  let theme = localStorage.getItem('theme')
  if (theme === 'system') {
    theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }
  if (!allowedThemes.has(theme)) {
    theme = 'light'
  }

  const font = localStorage.getItem('chat_font')
  const allowedFonts = new Set([
    'inter',
    'figtree',
    'jetbrains-mono',
    'quicksand',
    'montserrat',
    'source-sans-3',
    'nunito',
    'manrope',
    'work-sans',
    'ibm-plex-sans',
  ])
  const fontSize = Number(localStorage.getItem('chat_font_size'))
  const allowedSizes = new Set([13, 14, 16, 18, 20])

  const root = document.documentElement
  root.setAttribute('data-theme', theme)
  if (darkThemes.has(theme)) {
    root.classList.add('dark')
  } else {
    root.classList.remove('dark')
  }

  // Apply theme colors as CSS variables
  const colors = themeColors[theme] || themeColors.light
  root.style.setProperty('--theme-sidebar-bg', colors.sidebarBg)
  root.style.setProperty('--theme-sidebar-text', colors.sidebarText)
  root.style.setProperty('--theme-center-channel-bg', colors.centerChannelBg)
  root.style.setProperty('--theme-center-channel-color', colors.centerChannelColor)
  root.style.setProperty('--theme-link-color', colors.linkColor)
  root.style.setProperty('--theme-button-bg', colors.buttonBg)
  root.style.setProperty('--theme-button-color', colors.buttonColor)

  if (font && allowedFonts.has(font)) {
    root.style.setProperty('--chat-font-family', `var(--font-${font})`)
  }
  if (allowedSizes.has(fontSize)) {
    root.style.setProperty('--chat-font-size', `${fontSize}px`)
  }
})()
