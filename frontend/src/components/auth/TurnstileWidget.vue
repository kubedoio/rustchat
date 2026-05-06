<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

declare global {
  interface Window {
    turnstile?: {
      render: (container: HTMLElement | string, options: {
        sitekey: string
        callback?: (token: string) => void
        'error-callback'?: () => void
        'expired-callback'?: () => void
        theme?: 'light' | 'dark' | 'auto'
        size?: 'normal' | 'compact' | 'invisible'
      }) => string
      remove: (widgetId: string) => void
      reset: (widgetId: string) => void
    }
    onTurnstileLoad?: () => void
  }
}

const props = defineProps<{
  siteKey: string
}>()

const emit = defineEmits<{
  (e: 'verify', token: string): void
  (e: 'error'): void
  (e: 'expired'): void
}>()

const containerRef = ref<HTMLElement>()
const widgetId = ref<string>()
const scriptLoaded = ref(false)

function onTurnstileLoad() {
  scriptLoaded.value = true
  renderWidget()
}

function renderWidget() {
  if (!containerRef.value || !window.turnstile) return
  
  widgetId.value = window.turnstile.render(containerRef.value, {
    sitekey: props.siteKey,
    callback: (token: string) => {
      emit('verify', token)
    },
    'error-callback': () => {
      emit('error')
    },
    'expired-callback': () => {
      emit('expired')
    },
    theme: 'auto',
  })
}

onMounted(() => {
  // Check if Turnstile script is already loaded
  if (window.turnstile) {
    onTurnstileLoad()
    return
  }

  // Set up callback for when script loads
  window.onTurnstileLoad = onTurnstileLoad

  // Load Turnstile script if not already present
  if (!document.querySelector('script[src="https://challenges.cloudflare.com/turnstile/v0/api.js"]')) {
    const script = document.createElement('script')
    script.src = 'https://challenges.cloudflare.com/turnstile/v0/api.js?onload=onTurnstileLoad'
    script.async = true
    script.defer = true
    document.head.appendChild(script)
  }
})

onUnmounted(() => {
  if (widgetId.value && window.turnstile) {
    window.turnstile.remove(widgetId.value)
  }
})

defineExpose({
  reset: () => {
    if (widgetId.value && window.turnstile) {
      window.turnstile.reset(widgetId.value)
    }
  }
})
</script>

<template>
  <div ref="containerRef" class="turnstile-container"></div>
</template>

<style scoped>
.turnstile-container {
  display: flex;
  justify-content: center;
  min-height: 65px;
}
</style>
