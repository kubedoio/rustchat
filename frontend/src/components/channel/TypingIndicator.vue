<script setup lang="ts">
import { computed } from 'vue'
import { usePresenceStore } from '../../stores/presence'

const props = defineProps<{
    channelId: string
    threadId?: string
}>()

const presenceStore = usePresenceStore()
// Use the store getter which handles thread filtering
const typingUsers = presenceStore.getTypingUsersForChannel(props.channelId, props.threadId)

const typingText = computed(() => {
    const names = typingUsers.value.map(u => u.username)
    if (names.length === 0) return ''
    if (names.length === 1) return `${names[0]} is typing...`
    if (names.length === 2) return `${names[0]} and ${names[1]} are typing...`
    if (names.length === 3) return `${names[0]}, ${names[1]}, and ${names[2]} are typing...`
    return `${names[0]} and ${names.length - 1} others are typing...`
})
</script>

<template>
  <div 
    v-if="typingUsers.length > 0"
    class="px-5 py-2 text-xs font-medium text-gray-500 dark:text-gray-400 flex items-center space-x-2 bg-transparent transition-opacity duration-200"
  >
    <!-- Typing dots animation -->
    <div class="flex space-x-1">
      <div class="w-1.5 h-1.5 bg-blue-500 rounded-full animate-bounce" style="animation-delay: 0ms"></div>
      <div class="w-1.5 h-1.5 bg-blue-500 rounded-full animate-bounce" style="animation-delay: 150ms"></div>
      <div class="w-1.5 h-1.5 bg-blue-500 rounded-full animate-bounce" style="animation-delay: 300ms"></div>
    </div>
    <span class="animate-pulse">{{ typingText }}</span>
  </div>
</template>

<style scoped>
@keyframes bounce {
  0%, 80%, 100% {
    transform: translateY(0);
  }
  40% {
    transform: translateY(-4px);
  }
}
.animate-bounce {
  animation: bounce 1s infinite;
}
</style>
