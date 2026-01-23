import { defineStore } from 'pinia'
import { ref } from 'vue'

export type RhsView = 'thread' | 'search' | 'info' | 'saved' | 'pinned' | 'members' | null

export const useUIStore = defineStore('ui', () => {
    const isRhsOpen = ref(false)
    const isSettingsOpen = ref(false)
    const rhsView = ref<RhsView>(null)
    const rhsContextId = ref<string | null>(null)

    function openSettings() {
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

    return { isRhsOpen, isSettingsOpen, rhsView, rhsContextId, openRhs, closeRhs, toggleRhs, openSettings, closeSettings }
})
