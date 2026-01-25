import { defineStore } from 'pinia'
import { ref } from 'vue'

export type RhsView = 'thread' | 'search' | 'info' | 'saved' | 'pinned' | 'members' | null

export const useUIStore = defineStore('ui', () => {
    const isRhsOpen = ref(false)
    const isSettingsOpen = ref(false)
    const rhsView = ref<RhsView>(null)
    const rhsContextId = ref<string | null>(null)

    const videoCallUrl = ref<string | null>(null)
    const isVideoCallOpen = ref(false)

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

    function openVideoCall(url: string) {
        videoCallUrl.value = url
        isVideoCallOpen.value = true
    }

    function closeVideoCall() {
        isVideoCallOpen.value = false
        videoCallUrl.value = null
    }

    return {
        isRhsOpen,
        isSettingsOpen,
        rhsView,
        rhsContextId,
        videoCallUrl,
        isVideoCallOpen,
        openRhs,
        closeRhs,
        toggleRhs,
        openSettings,
        closeSettings,
        openVideoCall,
        closeVideoCall
    }
})
