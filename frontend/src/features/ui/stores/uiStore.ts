// UI Store - Backwards-compatible state management for UI state

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { normalizeSafeHttpUrl } from '../../../utils/safeUrl'

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

export const useUIStore = defineStore('uiStore', () => {
  // State
  const isLhsOpen = ref(false)
  const isRhsOpen = ref(false)
  const isSettingsOpen = ref(false)
  const settingsTab = ref<SettingsTab>('notifications')
  const rhsView = ref<RhsView>(null)
  const rhsContextId = ref<string | null>(null)

  const videoCallUrl = ref<string | null>(null)
  const isVideoCallOpen = ref(false)
  const density = ref<Density>((localStorage.getItem('density') as Density) || 'comfortable')

  // Actions
  function openSettings(tab: SettingsTab = 'notifications') {
    settingsTab.value = tab
    isSettingsOpen.value = true
  }

  function closeSettings() {
    isSettingsOpen.value = false
  }

  function openRhs(view: RhsView, contextId?: string) {
    rhsView.value = view
    rhsContextId.value = contextId || null
    isRhsOpen.value = true
  }

  function openLhs() {
    isLhsOpen.value = true
  }

  function closeLhs() {
    isLhsOpen.value = false
  }

  function toggleLhs() {
    isLhsOpen.value = !isLhsOpen.value
  }

  function closeRhs() {
    isRhsOpen.value = false
    rhsView.value = null
    rhsContextId.value = null
  }

  function toggleRhs(view: RhsView) {
    if (isRhsOpen.value && rhsView.value === view) {
      closeRhs()
    } else {
      openRhs(view)
    }
  }

  function openVideoCall(url: string) {
    const safeUrl = normalizeSafeHttpUrl(url)
    if (!safeUrl) {
      return
    }
    videoCallUrl.value = safeUrl
    isVideoCallOpen.value = true
  }

  function closeVideoCall() {
    isVideoCallOpen.value = false
    videoCallUrl.value = null
  }

  function setDensity(newDensity: Density) {
    density.value = newDensity
    localStorage.setItem('density', newDensity)
  }

  // Feature-compatible initialize
  function initialize() {
    const savedDensity = localStorage.getItem('density') as Density
    if (savedDensity === 'comfortable' || savedDensity === 'compact') {
      density.value = savedDensity
    }
  }

  return {
    // State
    isLhsOpen,
    isRhsOpen,
    isSettingsOpen,
    settingsTab,
    rhsView,
    rhsContextId,
    videoCallUrl,
    isVideoCallOpen,
    density,
    // Actions
    openLhs,
    closeLhs,
    toggleLhs,
    openRhs,
    closeRhs,
    toggleRhs,
    openSettings,
    closeSettings,
    openVideoCall,
    closeVideoCall,
    setDensity,
    initialize,
  }
})
