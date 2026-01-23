<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import ToastManager from './components/ui/ToastManager.vue'
import CommandPalette from './components/ui/CommandPalette.vue'
import SettingsModal from './components/settings/SettingsModal.vue'
import { useToast } from './composables/useToast'
import { useWebSocket } from './composables/useWebSocket'
import { useUIStore } from './stores/ui'
import { useAuthStore } from './stores/auth'
import ActiveCall from './components/calls/ActiveCall.vue'
import IncomingCallModal from './components/calls/IncomingCallModal.vue'
import { useConfigStore } from './stores/config'

const toastManagerRef = ref(null)
const { register } = useToast()
const ui = useUIStore()
const authStore = useAuthStore()
const configStore = useConfigStore()
const { connect, disconnect } = useWebSocket()

onMounted(async () => {
    if (toastManagerRef.value) {
        register(toastManagerRef.value)
    }
    await configStore.fetchPublicConfig()
    configStore.initSync()
})

// Connect WebSocket when authenticated
watch(() => authStore.isAuthenticated, (isAuth) => {
    if (isAuth) {
        connect()
    } else {
        disconnect()
    }
}, { immediate: true })
</script>

<template>
  <ToastManager ref="toastManagerRef" />
  <CommandPalette />
  <SettingsModal :isOpen="ui.isSettingsOpen" @close="ui.closeSettings()" />
  <ActiveCall />
  <IncomingCallModal />
  <router-view />
</template>
