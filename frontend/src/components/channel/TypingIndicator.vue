<script setup lang="ts">
import { computed, watch, ref } from 'vue'
import { usePresenceStore } from '../../features/presence'

const props = defineProps<{
    channelId: string
    threadId?: string
}>()

const presenceStore = usePresenceStore()

// Local reactive ref for typing users - ensures reactivity when channel changes
const localTypingUsers = ref<{ userId: string; username: string }[]>([])

// Watch for changes in the store and update local ref
watch(
  () => presenceStore.typingUsers,
  () => {
    const users: { userId: string; username: string }[] = []
    for (const user of presenceStore.typingUsers.values()) {
      if (user.channelId === props.channelId) {
        if (props.threadId) {
          if (user.threadRootId === props.threadId) {
            users.push({ userId: user.userId, username: user.username })
          }
        } else {
          if (!user.threadRootId) {
            users.push({ userId: user.userId, username: user.username })
          }
        }
      }
    }
    localTypingUsers.value = users
  },
  { immediate: true, deep: true }
)

// Also re-evaluate when channelId changes
watch(() => props.channelId, () => {
  const users: { userId: string; username: string }[] = []
  for (const user of presenceStore.typingUsers.values()) {
    if (user.channelId === props.channelId) {
      if (props.threadId) {
        if (user.threadRootId === props.threadId) {
          users.push({ userId: user.userId, username: user.username })
        }
      } else {
        if (!user.threadRootId) {
          users.push({ userId: user.userId, username: user.username })
        }
      }
    }
  }
  localTypingUsers.value = users
}, { immediate: true })

const typingText = computed(() => {
    const names = localTypingUsers.value.map(u => u.username)
    if (names.length === 0) return ''
    if (names.length === 1) return `${names[0]} is typing...`
    if (names.length === 2) return `${names[0]} and ${names[1]} are typing...`
    if (names.length === 3) return `${names[0]}, ${names[1]}, and ${names[2]} are typing...`
    return `${names[0]} and ${names.length - 1} others are typing...`
})
</script>

<template>
  <div 
    v-if="localTypingUsers.length > 0"
    class="px-5 py-2 text-xs font-medium text-gray-500 flex items-center space-x-2 bg-transparent transition-opacity duration-200"
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
