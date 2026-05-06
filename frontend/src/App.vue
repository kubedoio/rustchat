<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { useRoute } from 'vue-router'
import ToastManager from './components/ui/ToastManager.vue'
import ConnectionStatusBar from './components/ui/ConnectionStatusBar.vue'
import ConnectionLostModal from './components/ui/ConnectionLostModal.vue'
import CommandPalette from './components/ui/CommandPalette.vue'
import SettingsModal from './components/settings/SettingsModal.vue'
import TermsAcceptanceModal from './components/modals/TermsAcceptanceModal.vue'
import { useToast } from './composables/useToast'
import { useWebSocket } from './composables/useWebSocket'
import { useSingleActiveTab } from './composables/useSingleActiveTab'
import { useUIStore } from './stores/ui'
import { useAuthStore } from './stores/auth'
import { useUnreadStore } from './stores/unreads'
import ActiveCall from './components/calls/ActiveCall.vue'
import IncomingCallModal from './components/calls/IncomingCallModal.vue'
import { useConfigStore } from './stores/config'

const toastManagerRef = ref(null)
const { register } = useToast()
const ui = useUIStore()
const authStore = useAuthStore()
const unreadStore = useUnreadStore()
const configStore = useConfigStore()
const route = useRoute()
const { 
    connect, 
    disconnect, 
    connectionStatus,
    nextRetryIn
} = useWebSocket()
const singleTabEnabled = computed(() => authStore.isAuthenticated)
const { isActiveTab, takeOver } = useSingleActiveTab(singleTabEnabled)

const showTermsModal = computed(() => {
    // Only show terms modal for authenticated users outside of admin section
    return authStore.isAuthenticated && !route.path.startsWith('/admin')
})

// Connection action handlers
function handleRetryConnection() {
    // Force immediate reconnect attempt
    disconnect()
    setTimeout(() => connect(), 100)
}

function handleRefreshPage() {
    window.location.reload()
}

onMounted(async () => {
    if (toastManagerRef.value) {
        register(toastManagerRef.value)
    }
    await configStore.fetchPublicConfig()
    configStore.initSync()
})

// Connect realtime only when this tab is the active tab for the session.
watch([() => authStore.isAuthenticated, isActiveTab], async ([isAuth, isActive]) => {
    if (isAuth && isActive) {
        connect()
        await unreadStore.fetchOverview()
    } else {
        disconnect()
    }
}, { immediate: true })
</script>

<template>
  <ToastManager ref="toastManagerRef" />
  
  <!-- Connection status banner (States 1 & 2) -->
  <ConnectionStatusBar
    v-if="connectionStatus !== 'connected' && connectionStatus !== 'failed'"
    :status="connectionStatus"
    :next-retry-in="nextRetryIn"
    @retry="handleRetryConnection"
  />
  
  <!-- Connection lost modal (State 3) -->
  <ConnectionLostModal
    v-if="connectionStatus === 'failed'"
    @reconnect="handleRetryConnection"
    @refresh="handleRefreshPage"
  />
  
  <CommandPalette v-if="isActiveTab" />
  <SettingsModal v-if="isActiveTab" :isOpen="ui.isSettingsOpen" @close="ui.closeSettings()" />
  <ActiveCall v-if="isActiveTab" />
  <IncomingCallModal v-if="isActiveTab" />
  <TermsAcceptanceModal v-if="showTermsModal" />
  
  <!-- Main content with dimming when disconnected -->
  <div 
    :class="{ 
      'opacity-60': connectionStatus === 'disconnected',
      'opacity-80': connectionStatus === 'reconnecting'
    }"
    class="transition-opacity duration-500"
  >
    <router-view />
  </div>
  <div
    v-if="authStore.isAuthenticated && !isActiveTab"
    class="fixed inset-0 z-[120] flex items-center justify-center bg-bg-app/90 backdrop-blur-sm p-6"
  >
    <div class="w-full max-w-md rounded-r-2 border border-border-1 bg-bg-surface-1 p-6 text-center shadow-2">
      <h2 class="text-lg font-semibold text-text-1">This tab is inactive</h2>
      <p class="mt-2 text-sm text-text-2">
        Another tab is currently active for messaging in this browser session.
      </p>
      <button
        class="mt-5 inline-flex items-center justify-center rounded-r-1 bg-brand px-4 py-2 text-sm font-medium text-brand-foreground transition-standard hover:bg-brand-hover"
        @click="takeOver"
      >
        Use this tab instead
      </button>
    </div>
  </div>
</template>
