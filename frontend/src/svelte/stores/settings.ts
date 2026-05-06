import { derived } from 'svelte/store'
import { uiStore } from './ui'
import type { SettingsTab } from './ui'

export type { SettingsTab }

export const settingsOpen = derived(uiStore, ($ui) => $ui.isSettingsOpen)
export const settingsTab = derived(uiStore, ($ui) => $ui.settingsTab)

export function openSettings(tab: SettingsTab = 'notifications'): void {
    uiStore.openSettings(tab)
}

export function closeSettings(): void {
    uiStore.closeSettings()
}
