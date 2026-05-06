import { writable } from 'svelte/store'
import { readLocalStorage, writeLocalStorage } from '../lib/storage'

export type RhsView = 'thread' | 'search' | 'info' | 'saved' | 'pinned' | 'members' | null
export type Density = 'comfortable' | 'compact'
export type SettingsTab =
    | 'profile'
    | 'notifications'
    | 'display'
    | 'sidebar'
    | 'advanced'
    | 'calls'
    | 'security'

export interface UIState {
    isLhsOpen: boolean
    isRhsOpen: boolean
    isSettingsOpen: boolean
    settingsTab: SettingsTab
    rhsView: RhsView
    rhsContextId: string | null
    videoCallUrl: string | null
    isVideoCallOpen: boolean
    density: Density
    unreadNotificationCount: number
    unreadCountsByTeam: Record<string, number>
}

function normalizeDensity(value: string): Density {
    return value === 'compact' ? 'compact' : 'comfortable'
}

function getInitialState(): UIState {
    return {
        isLhsOpen: false,
        isRhsOpen: false,
        isSettingsOpen: false,
        settingsTab: 'notifications',
        rhsView: null,
        rhsContextId: null,
        videoCallUrl: null,
        isVideoCallOpen: false,
        density: normalizeDensity(readLocalStorage('density', 'comfortable')),
        unreadNotificationCount: 0,
        unreadCountsByTeam: {},
    }
}

function createUIStore() {
    const { subscribe, set, update } = writable<UIState>(getInitialState())

    return {
        subscribe,
        openSettings(settingsTab: SettingsTab = 'notifications'): void {
            update((state) => ({ ...state, settingsTab, isSettingsOpen: true }))
        },
        closeSettings(): void {
            update((state) => ({ ...state, isSettingsOpen: false }))
        },
        openRhs(rhsView: RhsView, rhsContextId: string | null = null): void {
            update((state) => ({ ...state, rhsView, rhsContextId, isRhsOpen: true }))
        },
        closeRhs(): void {
            update((state) => ({ ...state, isRhsOpen: false, rhsView: null, rhsContextId: null }))
        },
        toggleRhs(rhsView: RhsView): void {
            update((state) => {
                if (state.isRhsOpen && state.rhsView === rhsView) {
                    return { ...state, isRhsOpen: false, rhsView: null, rhsContextId: null }
                }

                return { ...state, isRhsOpen: true, rhsView, rhsContextId: null }
            })
        },
        openLhs(): void {
            update((state) => ({ ...state, isLhsOpen: true }))
        },
        closeLhs(): void {
            update((state) => ({ ...state, isLhsOpen: false }))
        },
        toggleLhs(): void {
            update((state) => ({ ...state, isLhsOpen: !state.isLhsOpen }))
        },
        openVideoCall(videoCallUrl: string): void {
            update((state) => ({ ...state, videoCallUrl, isVideoCallOpen: true }))
        },
        closeVideoCall(): void {
            update((state) => ({ ...state, videoCallUrl: null, isVideoCallOpen: false }))
        },
        setDensity(density: Density): void {
            writeLocalStorage('density', density)
            update((state) => ({ ...state, density }))
        },
        setRhsView(rhsView: RhsView): void {
            update((state) => ({ ...state, rhsView, isRhsOpen: true }))
        },
        reset(): void {
            set(getInitialState())
        },
    }
}

export const uiStore = createUIStore()
