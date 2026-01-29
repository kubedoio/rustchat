<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAuthStore } from '../../stores/auth'
import { ChevronDown, Check, Circle } from 'lucide-vue-next'
import { usePresenceStore } from '../../stores/presence'

const authStore = useAuthStore()
const presenceStore = usePresenceStore()
const isOpen = ref(false)

const statuses = [
  { id: 'online', label: 'Online', color: 'text-green-500' },
  { id: 'away', label: 'Away', color: 'text-amber-500' },
  { id: 'dnd', label: 'Do Not Disturb', color: 'text-red-500' },
  { id: 'offline', label: 'Invisible', color: 'text-gray-400' },
]

async function setStatus(status: string) {
  try {
    await authStore.updateStatus({ presence: status })
    // Also update presenceStore immediately for local feel
    if (authStore.user) {
      presenceStore.setUserPresence(authStore.user.id, authStore.user.username, status as any)
    }
    isOpen.value = false
  } catch (e) {
    console.error('Failed to set status', e)
  }
}

const currentStatus = computed(() => {
  return authStore.user?.presence || 'online'
})
</script>

<template>
  <div class="relative">
    <button 
      @click="isOpen = !isOpen"
      class="flex items-center space-x-2 px-2 py-1 rounded-md hover:bg-black/5 dark:hover:bg-white/5 transition-colors border border-transparent hover:border-black/10 dark:hover:border-white/10"
    >
      <Circle 
        class="w-3 h-3 fill-current" 
        :class="statuses.find(s => s.id === currentStatus)?.color || 'text-green-500'" 
      />
      <ChevronDown class="w-3.5 h-3.5 text-gray-500" />
    </button>

    <Teleport to="body">
      <div v-if="isOpen" @click="isOpen = false" class="fixed inset-0 z-[110]"></div>
    </Teleport>

    <transition
      enter-active-class="transition duration-100 ease-out"
      enter-from-class="transform scale-95 opacity-0"
      enter-to-class="transform scale-100 opacity-100"
      leave-active-class="transition duration-75 ease-in"
      leave-from-class="transform scale-100 opacity-100"
      leave-to-class="transform scale-95 opacity-0"
    >
      <div 
        v-if="isOpen"
        class="absolute right-0 mt-2 w-48 rounded-lg bg-white dark:bg-gray-800 shadow-xl border border-gray-200 dark:border-gray-700 py-1 z-[120]"
      >
        <div class="px-3 py-1.5 text-[10px] font-bold text-gray-400 uppercase tracking-wider">
          Set your status
        </div>
        <button
          v-for="status in statuses"
          :key="status.id"
          @click="setStatus(status.id)"
          class="w-full flex items-center justify-between px-3 py-2 text-xs hover:bg-indigo-50 dark:hover:bg-indigo-900/20 transition-colors"
          :class="currentStatus === status.id ? 'text-indigo-600 dark:text-indigo-400 font-semibold' : 'text-gray-700 dark:text-gray-300'"
        >
          <div class="flex items-center space-x-2">
            <Circle class="w-2.5 h-2.5 fill-current" :class="status.color" />
            <span>{{ status.label }}</span>
          </div>
          <Check v-if="currentStatus === status.id" class="w-3.5 h-3.5" />
        </button>
      </div>
    </transition>
  </div>
</template>
