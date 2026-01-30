import { defineStore } from 'pinia'
import { ref, onMounted, onUnmounted } from 'vue'

export type RhsView = 'thread' | 'search' | 'info' | 'saved' | 'pinned' | 'members' | null

export const useUIStore = defineStore('ui', () => {
    const isRhsOpen = ref(false)
    const isSettingsOpen = ref(false)
    const rhsView = ref<RhsView>(null)
    const rhsContextId = ref<string | null>(null)

    const videoCallUrl = ref<string | null>(null)
    const isVideoCallOpen = ref(false)
    
    // Mobile state
    const isMobile = ref(false)
    const isSidebarOpen = ref(false)
    const isTeamRailOpen = ref(false)
    
    // Check if mobile based on screen width
    const MOBILE_BREAKPOINT = 768 // md breakpoint
    
    function checkMobile() {
        isMobile.value = window.innerWidth < MOBILE_BREAKPOINT
    }
    
    function openSidebar() {
        isSidebarOpen.value = true
    }
    
    function closeSidebar() {
        isSidebarOpen.value = false
    }
    
    function toggleSidebar() {
        isSidebarOpen.value = !isSidebarOpen.value
    }
    
    function openTeamRail() {
        isTeamRailOpen.value = true
    }
    
    function closeTeamRail() {
        isTeamRailOpen.value = false
    }
    
    function toggleTeamRail() {
        isTeamRailOpen.value = !isTeamRailOpen.value
    }
    
    // Initialize mobile check
    onMounted(() => {
        checkMobile()
        window.addEventListener('resize', checkMobile)
    })
    
    onUnmounted(() => {
        window.removeEventListener('resize', checkMobile)
    })

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
        isMobile,
        isSidebarOpen,
        isTeamRailOpen,
        openRhs,
        closeRhs,
        toggleRhs,
        openSettings,
        closeSettings,
        openVideoCall,
        closeVideoCall,
        openSidebar,
        closeSidebar,
        toggleSidebar,
        openTeamRail,
        closeTeamRail,
        toggleTeamRail,
        checkMobile
    }
})
