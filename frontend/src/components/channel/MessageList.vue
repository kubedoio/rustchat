<script setup lang="ts">
import { ref, watch, computed, nextTick } from 'vue'
import { ArrowDown } from 'lucide-vue-next'
import { useMessageStore } from '../../stores/messages'
import { useUnreadStore } from '../../stores/unreads'
import MessageItem from './MessageItem.vue'

const props = defineProps<{
  channelId: string
}>()

const emit = defineEmits<{
  (e: 'reply', messageId: string): void
  (e: 'delete', messageId: string): void
  (e: 'edit', messageId: string): void
}>()

const messageStore = useMessageStore()
const unreadStore = useUnreadStore()
const containerRef = ref<HTMLElement | null>(null)
const shouldAutoScroll = ref(true)
const showNewMessagesBtn = ref(false)

const messages = computed(() => messageStore.messagesByChannel[props.channelId] || [])
const readState = computed(() => unreadStore.getChannelReadState(props.channelId))

// Handle scroll events to detect if user is at bottom or top (infinite scroll)
async function handleScroll() {
  if (!containerRef.value || messageStore.loading) return
  
  const { scrollTop, scrollHeight, clientHeight } = containerRef.value
  const distanceToBottom = scrollHeight - scrollTop - clientHeight
  
  // 1. Auto-scroll logic
  const atBottom = distanceToBottom < 50
  shouldAutoScroll.value = atBottom
  
  if (atBottom) {
    showNewMessagesBtn.value = false

    // Mark as read if there are unreads
    if (unreadStore.getChannelUnreadCount(props.channelId) > 0) {
        unreadStore.markAsRead(props.channelId)
    }
  }

  // 2. Reverse infinite scroll (load older)
  if (scrollTop < 100 && messageStore.hasMoreOlder(props.channelId)) {
    const oldScrollHeight = scrollHeight
    await messageStore.fetchOlderMessages(props.channelId)
    
    // Preserve scroll position after prepending messages
    nextTick(() => {
      if (containerRef.value) {
        const newScrollHeight = containerRef.value.scrollHeight
        containerRef.value.scrollTop = newScrollHeight - oldScrollHeight
      }
    })
  }
}

// Scroll to bottom
function scrollToBottom(behavior: ScrollBehavior = 'auto') {
  if (!containerRef.value) return
  containerRef.value.scrollTo({
    top: containerRef.value.scrollHeight,
    behavior
  })
}

// Watch for NEW messages (not full fetch)
watch(() => messages.value.length, (newLen, oldLen) => {
  if (newLen > oldLen) {
    if (shouldAutoScroll.value) {
      nextTick(() => scrollToBottom('smooth'))
    } else {
      showNewMessagesBtn.value = true
    }
  }
})

// Watch for loading state change (e.g. refresh via WebSocket)
watch(() => messageStore.loading, (loading) => {
    if (!loading && !messageStore.isLoadingOlder) {
        nextTick(() => scrollToBottom('auto'))
    }
})

// Watch for channel change to refetch and reset scroll
watch(() => props.channelId, async (newId) => {
    if (newId) {
        showNewMessagesBtn.value = false
        shouldAutoScroll.value = true
        await messageStore.fetchMessages(newId)
        nextTick(() => scrollToBottom())
    }
}, { immediate: true })

const highlightedMessageId = ref<string | null>(null)

function scrollToMessage(messageId: string) {
  const element = containerRef.value?.querySelector(`[data-message-id="${messageId}"]`)
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'center' })
    highlightedMessageId.value = messageId
    setTimeout(() => {
      highlightedMessageId.value = null
    }, 2000)
    shouldAutoScroll.value = false
  }
}

defineExpose({ scrollToMessage })

function handleReply(id: string) {
  emit('reply', id)
}

function handleDelete(id: string) {
  emit('delete', id)
}

function handleEdit(id: string) {
  emit('edit', id)
}
</script>

<template>
  <div 
    class="flex-1 overflow-y-auto px-4 py-4 space-y-6 custom-scrollbar relative" 
    ref="containerRef"
    @scroll="handleScroll"
  >
    <!-- New Messages Floating Button -->
    <transition
      enter-active-class="transition ease-out duration-200"
      enter-from-class="transform translate-y-4 opacity-0"
      enter-to-class="transform translate-y-0 opacity-100"
      leave-active-class="transition ease-in duration-150"
      leave-from-class="transform translate-y-0 opacity-100"
      leave-to-class="transform translate-y-4 opacity-0"
    >
      <div 
        v-if="showNewMessagesBtn"
        @click="scrollToBottom('smooth')"
        class="absolute bottom-6 left-1/2 -translate-x-1/2 z-10 bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-full shadow-lg cursor-pointer flex items-center space-x-2 text-sm font-medium animate-bounce"
      >
        <ArrowDown class="w-4 h-4" />
        <span>New messages</span>
      </div>
    </transition>

    <!-- Loading State -->
    <div v-if="messageStore.loading" class="text-center text-gray-500 py-10">
        <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full mx-auto mb-2"></div>
        <p>Loading messages...</p>
    </div>

    <!-- Empty State -->
    <div v-else-if="messages.length === 0" class="text-center text-gray-500 py-10">
        <p>This is the start of the channel.</p>
        <p class="text-sm mt-2">Send a message to start the conversation.</p>
    </div>

    <!-- Message List -->
    <div v-else class="space-y-[1px]">
        <template v-for="msg in messages" :key="msg.id">
            <!-- New Messages Divider -->
            <div
                v-if="readState?.first_unread_message_id && Number(msg.seq) === Number(readState.first_unread_message_id)"
                class="flex items-center my-6 py-2"
            >
                <div class="flex-1 h-px bg-rose-500/30"></div>
                <div class="px-3 flex items-center space-x-2">
                    <span class="text-[11px] font-bold text-rose-500 uppercase tracking-wider">New Messages</span>
                </div>
                <div class="flex-1 h-px bg-rose-500/30"></div>
            </div>

            <MessageItem
                :message="msg"
                :data-message-id="msg.id"
                :class="{ 'bg-yellow-100/50 dark:bg-yellow-500/10 ring-1 ring-yellow-400/50': highlightedMessageId === msg.id }"
                class="transition-all duration-500 rounded-sm"
                @reply="handleReply"
                @delete="handleDelete"
                @edit="handleEdit"
            />
        </template>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #CBD5E1;
  border-radius: 6px;
}
.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: #374151;
}
</style>
