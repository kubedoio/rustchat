import { writable } from 'svelte/store'
import { readLocalStorage, writeLocalStorage } from '../lib/storage'
import { svelteApi } from './http'

export type Theme =
    | 'light'
    | 'dark'
    | 'modern'
    | 'metallic'
    | 'futuristic'
    | 'high-contrast'
    | 'simple'
    | 'dynamic'

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

export interface ThemeColors {
    sidebarBg: string
    sidebarText: string
    centerChannelBg: string
    centerChannelColor: string
    linkColor: string
    buttonBg: string
    buttonColor: string
}

export interface ThemeOption {
    id: Theme
    label: string
    swatches: { primary: string; accent: string; background: string }
    colors: ThemeColors
}

export interface ThemeState {
    theme: Theme
    chatFont: ChatFont
    chatFontSize: ChatFontSize
    syncedServerToken: string | null
}

interface Preference {
    user_id: string
    category: string
    name: string
    value: string
}

export const THEME_OPTIONS: ThemeOption[] = [
    {
        id: 'light',
        label: 'Light',
        swatches: { primary: '#b45309', accent: '#14b8a6', background: '#f5f3ef' },
        colors: {
            sidebarBg: '#f2eee7',
            sidebarText: '#1c1917',
            centerChannelBg: '#fffdf8',
            centerChannelColor: '#1c1917',
            linkColor: '#b45309',
            buttonBg: '#b45309',
            buttonColor: '#fffdf8',
        },
    },
    {
        id: 'dark',
        label: 'Dark',
        swatches: { primary: '#f59e0b', accent: '#14b8a6', background: '#12100d' },
        colors: {
            sidebarBg: '#15120f',
            sidebarText: '#f7f3ec',
            centerChannelBg: '#1a1713',
            centerChannelColor: '#f7f3ec',
            linkColor: '#f59e0b',
            buttonBg: '#f59e0b',
            buttonColor: '#1a1713',
        },
    },
    {
        id: 'modern',
        label: 'Modern',
        swatches: { primary: '#0f766e', accent: '#14b8a6', background: '#f3f7f6' },
        colors: {
            sidebarBg: '#1a1d24',
            sidebarText: '#e2e8f0',
            centerChannelBg: '#f3f7f6',
            centerChannelColor: '#0f172a',
            linkColor: '#0f766e',
            buttonBg: '#0f766e',
            buttonColor: '#ffffff',
        },
    },
    {
        id: 'metallic',
        label: 'Metallic',
        swatches: { primary: '#475569', accent: '#d97706', background: '#e7eaee' },
        colors: {
            sidebarBg: '#334155',
            sidebarText: '#f1f5f9',
            centerChannelBg: '#e7eaee',
            centerChannelColor: '#1e293b',
            linkColor: '#d97706',
            buttonBg: '#475569',
            buttonColor: '#ffffff',
        },
    },
    {
        id: 'futuristic',
        label: 'Futuristic',
        swatches: { primary: '#22d3ee', accent: '#22c55e', background: '#030712' },
        colors: {
            sidebarBg: '#0f172a',
            sidebarText: '#22c55e',
            centerChannelBg: '#030712',
            centerChannelColor: '#d6f1ff',
            linkColor: '#22c55e',
            buttonBg: '#22d3ee',
            buttonColor: '#000000',
        },
    },
    {
        id: 'high-contrast',
        label: 'High Contrast',
        swatches: { primary: '#00e5ff', accent: '#ffd400', background: '#000000' },
        colors: {
            sidebarBg: '#000000',
            sidebarText: '#ffffff',
            centerChannelBg: '#000000',
            centerChannelColor: '#ffffff',
            linkColor: '#00e5ff',
            buttonBg: '#00e5ff',
            buttonColor: '#000000',
        },
    },
    {
        id: 'simple',
        label: 'Simple',
        swatches: { primary: '#0369a1', accent: '#16a34a', background: '#fafaf9' },
        colors: {
            sidebarBg: '#44403c',
            sidebarText: '#fafaf9',
            centerChannelBg: '#fafaf9',
            centerChannelColor: '#292524',
            linkColor: '#0369a1',
            buttonBg: '#16a34a',
            buttonColor: '#ffffff',
        },
    },
    {
        id: 'dynamic',
        label: 'Dynamic',
        swatches: { primary: '#e11d48', accent: '#f59e0b', background: '#111827' },
        colors: {
            sidebarBg: '#1f2937',
            sidebarText: '#f9fafb',
            centerChannelBg: '#111827',
            centerChannelColor: '#e5e7eb',
            linkColor: '#e11d48',
            buttonBg: '#f59e0b',
            buttonColor: '#000000',
        },
    },
]

export const FONT_OPTIONS: Array<{ id: ChatFont; label: string; cssVar: string }> = [
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

const STORAGE_THEME = 'theme'
const STORAGE_FONT = 'chat_font'
const STORAGE_FONT_SIZE = 'chat_font_size'
const AUTH_TOKEN_KEY = 'auth_token'
const SERVER_PREFERENCE_CATEGORY = 'rustchat_display'
const SERVER_PREFERENCE_THEME = 'theme'
const SERVER_PREFERENCE_FONT = 'font'
const SERVER_PREFERENCE_FONT_SIZE = 'font_size'
const DARK_THEME_SET = new Set<Theme>(['dark', 'futuristic', 'high-contrast', 'dynamic'])

const DEFAULT_THEME_COLORS: ThemeColors = {
    sidebarBg: '#f2eee7',
    sidebarText: '#1c1917',
    centerChannelBg: '#fffdf8',
    centerChannelColor: '#1c1917',
    linkColor: '#b45309',
    buttonBg: '#b45309',
    buttonColor: '#fffdf8',
}

function isTheme(value: unknown): value is Theme {
    return typeof value === 'string' && THEME_OPTIONS.some((option) => option.id === value)
}

function isChatFont(value: unknown): value is ChatFont {
    return typeof value === 'string' && FONT_OPTIONS.some((option) => option.id === value)
}

function normalizeTheme(value: string | null): Theme {
    if (value === 'system' && typeof window !== 'undefined') {
        return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }

    return isTheme(value) ? value : 'light'
}

function normalizeFontSize(value: string | null): ChatFontSize {
    const parsed = Number(value)
    return FONT_SIZE_OPTIONS.includes(parsed as ChatFontSize) ? (parsed as ChatFontSize) : 14
}

function getInitialState(): ThemeState {
    const initialFont = readLocalStorage(STORAGE_FONT, 'ibm-plex-sans')

    return {
        theme: normalizeTheme(readLocalStorage(STORAGE_THEME, 'light')),
        chatFont: isChatFont(initialFont) ? initialFont : 'ibm-plex-sans',
        chatFontSize: normalizeFontSize(readLocalStorage(STORAGE_FONT_SIZE, '14')),
        syncedServerToken: null,
    }
}

function buildPreferencePayload(state: ThemeState): Preference[] {
    return [
        {
            user_id: 'me',
            category: SERVER_PREFERENCE_CATEGORY,
            name: SERVER_PREFERENCE_THEME,
            value: state.theme,
        },
        {
            user_id: 'me',
            category: SERVER_PREFERENCE_CATEGORY,
            name: SERVER_PREFERENCE_FONT,
            value: state.chatFont,
        },
        {
            user_id: 'me',
            category: SERVER_PREFERENCE_CATEGORY,
            name: SERVER_PREFERENCE_FONT_SIZE,
            value: String(state.chatFontSize),
        },
    ]
}

function parseServerAppearancePreferences(rows: unknown): Partial<ThemeState> {
    if (!Array.isArray(rows)) {
        return {}
    }

    const prefs = rows as Preference[]
    const getValue = (name: string) =>
        prefs.find((preference) => preference.category === SERVER_PREFERENCE_CATEGORY && preference.name === name)
            ?.value
    const fontSize = getValue(SERVER_PREFERENCE_FONT_SIZE)

    return {
        theme: isTheme(getValue(SERVER_PREFERENCE_THEME)) ? getValue(SERVER_PREFERENCE_THEME) as Theme : undefined,
        chatFont: isChatFont(getValue(SERVER_PREFERENCE_FONT)) ? getValue(SERVER_PREFERENCE_FONT) as ChatFont : undefined,
        chatFontSize:
            typeof fontSize === 'string' && FONT_SIZE_OPTIONS.includes(Number(fontSize) as ChatFontSize)
                ? Number(fontSize) as ChatFontSize
                : undefined,
    }
}

export function getThemeColors(themeId: Theme): ThemeColors {
    return THEME_OPTIONS.find((option) => option.id === themeId)?.colors ?? DEFAULT_THEME_COLORS
}

function applyThemeValue(theme: Theme): void {
    if (typeof window === 'undefined') {
        return
    }

    const root = window.document.documentElement
    root.setAttribute('data-theme', theme)
    root.classList.toggle('dark', DARK_THEME_SET.has(theme))
}

function applyTypographyValue(chatFont: ChatFont, chatFontSize: ChatFontSize): void {
    if (typeof window === 'undefined') {
        return
    }

    const root = window.document.documentElement
    root.style.setProperty('--chat-font-family', `var(--font-${chatFont})`)
    root.style.setProperty('--chat-font-size', `${chatFontSize}px`)
}

function createThemeStore() {
    const { subscribe, set, update } = writable<ThemeState>(getInitialState())

    function applyAppearance(state?: ThemeState): void {
        const next = state ?? getInitialState()
        applyThemeValue(next.theme)
        applyTypographyValue(next.chatFont, next.chatFontSize)
    }

    async function persistToServer(state: ThemeState): Promise<void> {
        const token = readLocalStorage(AUTH_TOKEN_KEY, '')
        if (!token) {
            return
        }

        try {
            await svelteApi.put('/users/me/preferences', buildPreferencePayload(state), { baseURL: '/api/v4' })
        } catch (error) {
            console.debug('Failed to persist appearance preferences to server', error)
        }
    }

    function updateAppearance(mutator: (state: ThemeState) => ThemeState): void {
        update((state) => {
            const next = mutator(state)
            writeLocalStorage(STORAGE_THEME, next.theme)
            writeLocalStorage(STORAGE_FONT, next.chatFont)
            writeLocalStorage(STORAGE_FONT_SIZE, String(next.chatFontSize))
            applyAppearance(next)
            void persistToServer(next)
            return next
        })
    }

    return {
        subscribe,
        applyAppearance,
        applyTheme(theme?: Theme): void {
            applyThemeValue(theme ?? getInitialState().theme)
        },
        setTheme(theme: Theme | 'system'): void {
            updateAppearance((state) => ({ ...state, theme: normalizeTheme(theme) }))
        },
        setChatFont(chatFont: ChatFont): void {
            if (!isChatFont(chatFont)) {
                return
            }

            updateAppearance((state) => ({ ...state, chatFont }))
        },
        setChatFontSize(chatFontSize: ChatFontSize): void {
            if (!FONT_SIZE_OPTIONS.includes(chatFontSize)) {
                return
            }

            updateAppearance((state) => ({ ...state, chatFontSize }))
        },
        async syncFromServer(force = false): Promise<void> {
            const token = readLocalStorage(AUTH_TOKEN_KEY, '')
            if (!token) {
                update((state) => ({ ...state, syncedServerToken: null }))
                return
            }

            let shouldSync = false
            update((state) => {
                shouldSync = force || state.syncedServerToken !== token
                return state
            })

            if (!shouldSync) {
                return
            }

            try {
                const { data } = await svelteApi.get<Preference[]>('/users/me/preferences', { baseURL: '/api/v4' })
                const serverPrefs = parseServerAppearancePreferences(data)

                update((state) => {
                    const next = { ...state, ...serverPrefs, syncedServerToken: token }
                    writeLocalStorage(STORAGE_THEME, next.theme)
                    writeLocalStorage(STORAGE_FONT, next.chatFont)
                    writeLocalStorage(STORAGE_FONT_SIZE, String(next.chatFontSize))
                    applyAppearance(next)
                    return next
                })
            } catch (error) {
                console.debug('Failed to sync appearance preferences from server', error)
            }
        },
        reset(): void {
            const next = getInitialState()
            set(next)
            applyAppearance(next)
        },
    }
}

export const themeStore = createThemeStore()

if (typeof window !== 'undefined') {
    themeStore.applyAppearance()
}
