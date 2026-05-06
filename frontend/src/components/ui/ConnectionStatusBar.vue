<script setup lang="ts">
import { computed } from 'vue'
import { WifiOff, Loader2, AlertCircle } from 'lucide-vue-next'

const props = defineProps<{
  status: 'reconnecting' | 'disconnected' | 'failed'
  nextRetryIn: number
}>()

const emit = defineEmits<{
  retry: []
}>()

const config = computed(() => {
  switch (props.status) {
    case 'reconnecting':
      return {
        bg: 'bg-amber-500/95',
        icon: Loader2,
        iconClass: 'animate-spin',
        message: 'Reconnecting...',
        showRetry: false
      }
    case 'disconnected':
      return {
        bg: 'bg-orange-500/95',
        icon: AlertCircle,
        iconClass: '',
        message: `Connection lost. Retrying in ${props.nextRetryIn}s...`,
        showRetry: true
      }
    case 'failed':
      return {
        bg: 'bg-red-500/95',
        icon: WifiOff,
        iconClass: '',
        message: 'Connection failed. Please reconnect.',
        showRetry: true
      }
    default:
      return null
  }
})

const cfg = computed(() => config.value)
</script>

<template>
  <Transition name="slide-down">
    <div
      v-if="cfg"
      :class="cfg.bg"
      class="fixed top-0 left-0 right-0 z-[100] flex items-center justify-center gap-2 px-4 py-2 text-sm font-medium text-white shadow-lg"
    >
      <component :is="cfg.icon" class="h-4 w-4" :class="cfg.iconClass" />
      <span>{{ cfg.message }}</span>
      <button
        v-if="cfg.showRetry"
        @click="emit('retry')"
        class="ml-2 rounded bg-white/20 px-2 py-0.5 text-xs font-semibold hover:bg-white/30 transition-colors"
      >
        Retry now
      </button>
    </div>
  </Transition>
</template>

<style scoped>
.slide-down-enter-active,
.slide-down-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.slide-down-enter-from,
.slide-down-leave-to {
  transform: translateY(-100%);
  opacity: 0;
}
</style>
