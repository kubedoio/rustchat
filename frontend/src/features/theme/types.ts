// Theme Types

export type Theme =
  'light' | 'dark' | 'modern' | 'metallic' | 'futuristic' | 'high-contrast' | 'simple' | 'dynamic'

export type ChatFont =
  | 'inter'
  | 'figtree'
  | 'jetbrains-mono'
  | 'quicksand'
  | 'montserrat'
  | 'source-sans-3'
  | 'nunito'
  | 'manrope'
  | 'work-sans'
  | 'ibm-plex-sans'

export type ChatFontSize = 13 | 14 | 16 | 18 | 20

export interface ThemeOption {
  id: Theme
  label: string
  swatches: { primary: string; accent: string; background: string }
}

export interface FontOption {
  id: ChatFont
  label: string
  cssVar: string
}

export const THEME_OPTIONS: ThemeOption[] = [
  {
    id: 'light',
    label: 'Light',
    swatches: { primary: '#2563eb', accent: '#0ea5e9', background: '#f6f8fb' },
  },
  {
    id: 'dark',
    label: 'Dark',
    swatches: { primary: '#38bdf8', accent: '#22d3ee', background: '#0b1220' },
  },
  {
    id: 'modern',
    label: 'Modern',
    swatches: { primary: '#0f766e', accent: '#14b8a6', background: '#f3f7f6' },
  },
  {
    id: 'metallic',
    label: 'Metallic',
    swatches: { primary: '#475569', accent: '#d97706', background: '#e7eaee' },
  },
  {
    id: 'futuristic',
    label: 'Futuristic',
    swatches: { primary: '#06b6d4', accent: '#22c55e', background: '#030712' },
  },
  {
    id: 'high-contrast',
    label: 'High Contrast',
    swatches: { primary: '#00e5ff', accent: '#ffd400', background: '#000000' },
  },
  {
    id: 'simple',
    label: 'Simple',
    swatches: { primary: '#0369a1', accent: '#16a34a', background: '#fafaf9' },
  },
  {
    id: 'dynamic',
    label: 'Dynamic',
    swatches: { primary: '#e11d48', accent: '#f59e0b', background: '#111827' },
  },
]

export const FONT_OPTIONS: FontOption[] = [
  { id: 'inter', label: 'Inter', cssVar: 'var(--font-inter)' },
  { id: 'figtree', label: 'Figtree', cssVar: 'var(--font-figtree)' },
  { id: 'jetbrains-mono', label: 'JetBrains Mono', cssVar: 'var(--font-jetbrains-mono)' },
  { id: 'quicksand', label: 'Quicksand', cssVar: 'var(--font-quicksand)' },
  { id: 'montserrat', label: 'Montserrat', cssVar: 'var(--font-montserrat)' },
  { id: 'source-sans-3', label: 'Source Sans 3', cssVar: 'var(--font-source-sans-3)' },
  { id: 'nunito', label: 'Nunito', cssVar: 'var(--font-nunito)' },
  { id: 'manrope', label: 'Manrope', cssVar: 'var(--font-manrope)' },
  { id: 'work-sans', label: 'Work Sans', cssVar: 'var(--font-work-sans)' },
  { id: 'ibm-plex-sans', label: 'IBM Plex Sans', cssVar: 'var(--font-ibm-plex-sans)' },
]

export const FONT_SIZE_OPTIONS: ChatFontSize[] = [13, 14, 16, 18, 20]

export const DARK_THEMES: Set<Theme> = new Set(['dark', 'futuristic', 'high-contrast', 'dynamic'])
