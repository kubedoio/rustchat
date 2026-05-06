<script setup lang="ts">
import { ref, watch, computed, nextTick } from 'vue'
import { format, isSameYear, isToday, isYesterday } from 'date-fns'
import { ArrowDown } from 'lucide-vue-next'
import { useMessageStore } from '../../stores/messages'
import { useUnreadStore } from '../../stores/unreads'
import { useChannelStore } from '../../stores/channels'
import { usePresence, extractUserIds } from '../../composables/usePresence'
import MessageItem from './MessageItem.vue'

const props = defineProps<{
  channelId: string
}>()

const emit = defineEmits<{
  (e: 'reply', messageId: string): void
  (e: 'delete', messageId: string): void
  (e: 'edit', messageId: string): void
  (e: 'openProfile', userId: string): void
}>()

const messageStore = useMessageStore()
const unreadStore = useUnreadStore()
const channelStore = useChannelStore()
const presence = usePresence()
const containerRef = ref<HTMLElement | null>(null)
const shouldAutoScroll = ref(true)
const showNewMessagesBtn = ref(false)

const messages = computed(() => messageStore.messagesByChannel[props.channelId] || [])
const readState = computed(() => unreadStore.getChannelReadState(props.channelId))
const currentChannel = computed(() => channelStore.currentChannel)

type TimelineItem =
  | { kind: 'date'; key: string; label: string }
  | { kind: 'message'; key: string; message: (typeof messages.value)[number] }

function formatDateSeparator(date: Date): string {
  if (!Number.isFinite(date.getTime())) {
    return ''
  }
  if (isToday(date)) {
    return 'Today'
  }
  if (isYesterday(date)) {
    return 'Yesterday'
  }
  if (isSameYear(date, new Date())) {
    return format(date, 'EEEE, MMM d')
  }
  return format(date, 'PP')
}

const timelineItems = computed<TimelineItem[]>(() => {
  const items: TimelineItem[] = []
  let lastDayKey: string | null = null

  for (const message of messages.value) {
    const date = new Date(message.timestamp)
    const dayKey = Number.isFinite(date.getTime()) ? format(date, 'yyyy-MM-dd') : message.id

    if (dayKey !== lastDayKey) {
      items.push({
        kind: 'date',
        key: `date-${dayKey}`,
        label: formatDateSeparator(date),
      })
      lastDayKey = dayKey
    }

    items.push({
      kind: 'message',
      key: `msg-${message.id}`,
      message,
    })
  }

  return items
})

// Batch fetch user statuses when messages change
watch(() => messages.value, (newMessages) => {
  if (newMessages.length > 0) {
    const userIds = extractUserIds(newMessages)
    if (userIds.length > 0) {
      presence.fetchMissing(userIds)
    }
  }
}, { immediate: true })

// Handle scroll events
async function handleScroll() {
  if (!containerRef.value || messageStore.loading) return
  
  const { scrollTop, scrollHeight, clientHeight } = containerRef.value
  const distanceToBottom = scrollHeight - scrollTop - clientHeight
  
  // Auto-scroll logic
  const atBottom = distanceToBottom < 50
  shouldAutoScroll.value = atBottom
  
  if (atBottom) {
    showNewMessagesBtn.value = false
    
    // Mark as read if there are unreads
    if (unreadStore.getChannelUnreadCount(props.channelId) > 0) {
      unreadStore.markAsRead(props.channelId)
    }
  }

  // Reverse infinite scroll (load older)
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

// Watch for NEW messages
watch(() => messages.value.length, (newLen, oldLen) => {
  if (newLen > oldLen) {
    if (shouldAutoScroll.value) {
      nextTick(() => scrollToBottom('smooth'))
    } else {
      showNewMessagesBtn.value = true
    }
  }
})

// Watch for loading state change
watch(() => messageStore.loading, (loading) => {
  if (!loading && !messageStore.isLoadingOlder) {
    nextTick(() => scrollToBottom('auto'))
  }
})

// Watch for channel change
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

function handleOpenProfile(userId: string) {
  emit('openProfile', userId)
}
</script>

<template>
  <div 
    class="flex-1 overflow-y-auto custom-scrollbar relative bg-bg-surface-1" 
    ref="containerRef"
    @scroll="handleScroll"
  >
    <div class="max-w-[var(--msg-max-width)] mx-auto px-[var(--msg-gutter)] py-4">
      
      <!-- New Messages Button -->
      <Transition
        enter-active-class="transition-all duration-200 ease-out"
        enter-from-class="opacity-0 translate-y-4"
        enter-to-class="opacity-100 translate-y-0"
        leave-active-class="transition-all duration-150 ease-in"
        leave-from-class="opacity-100 translate-y-0"
        leave-to-class="opacity-0 translate-y-4"
      >
        <button
          v-if="showNewMessagesBtn"
          @click="scrollToBottom('smooth')"
          class="fixed bottom-24 left-1/2 z-20 flex -translate-x-1/2 items-center gap-2 rounded-full bg-brand px-4 py-2 text-sm font-medium text-brand-foreground shadow-2 transition-standard hover:bg-brand-hover"
        >
          <ArrowDown class="w-4 h-4" />
          <span>New messages</span>
        </button>
      </Transition>

      <!-- Loading State -->
      <div v-if="messageStore.loading" class="flex flex-col items-center justify-center py-16 text-text-3">
        <div class="w-full max-w-lg rounded-r-3 border border-border-1 bg-[radial-gradient(circle_at_top,_color-mix(in_srgb,_var(--brand)_8%,transparent),transparent_55%)] px-6 py-8 text-center shadow-1">
          <div class="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-r-2 bg-brand/10 text-brand">
            <div class="h-8 w-8 animate-spin rounded-full border-2 border-current border-t-transparent" />
          </div>
          <p class="text-lg font-semibold text-text-1">Syncing conversation</p>
          <p class="mt-1 text-sm text-text-3">Pulling the latest messages and read state into this channel.</p>
        </div>
      </div>

      <!-- Empty State -->
      <div v-else-if="messages.length === 0" class="flex flex-col items-center justify-center py-20 text-text-3">
        <div class="w-full max-w-xl rounded-r-3 border border-border-1 bg-[radial-gradient(circle_at_top,_color-mix(in_srgb,_var(--brand)_10%,transparent),transparent_58%)] px-6 py-9 text-center shadow-1">
          <div class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-r-2 bg-brand/10 text-brand shadow-1">
            <span class="text-3xl">#</span>
          </div>
          <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-3">
            Quiet channel
          </p>
          <p class="mt-2 text-xl font-semibold tracking-[-0.03em] text-text-1">
            Kick off {{ currentChannel?.display_name || currentChannel?.name || 'the conversation' }}
          </p>
          <p class="mx-auto mt-2 max-w-md text-sm leading-6 text-text-3">
            Share the first update, question, or handoff so this space starts carrying real team context instead of staying blank.
          </p>
          <div class="mt-5 inline-flex items-center rounded-full border border-border-1 bg-bg-surface-1 px-3 py-1 text-xs font-medium text-text-2">
            First message wins the tone of the channel
          </div>
        </div>
      </div>

      <!-- Message List -->
      <div v-else class="space-y-[var(--msg-spacing)]">
        <template v-for="item in timelineItems" :key="item.key">
          <!-- Date Separator -->
          <div
            v-if="item.kind === 'date'"
            class="flex items-center my-4 sticky top-0 z-[5]"
          >
            <div class="flex-1 h-px bg-border-1"></div>
            <span class="px-4 py-1 mx-4 text-[11px] font-semibold uppercase tracking-wider text-text-3 bg-bg-surface-1 rounded-full border border-border-1">
              {{ item.label }}
            </span>
            <div class="flex-1 h-px bg-border-1"></div>
          </div>

          <template v-else>
            <!-- New Messages Divider -->
            <div 
              v-if="readState?.first_unread_message_id && Number(item.message.seq) === Number(readState.first_unread_message_id)" 
              class="flex items-center my-3"
            >
              <div class="flex-1 h-px bg-danger/30"></div>
              <div class="px-3 flex items-center gap-2">
                <span class="text-[10px] font-bold text-danger uppercase tracking-wider">New Messages</span>
              </div>
              <div class="flex-1 h-px bg-danger/30"></div>
            </div>

            <!-- Message Item -->
            <MessageItem 
              :message="item.message" 
              :data-message-id="item.message.id"
              :class="{ 'ring-1 ring-brand/20 bg-brand/5': highlightedMessageId === item.message.id }"
              class="transition-standard rounded-r-1"
              @reply="handleReply"
              @delete="handleDelete"
              @edit="handleEdit"
              @openProfile="handleOpenProfile"
            />
          </template>
        </template>
      </div>
      
      <!-- Bottom Spacer -->
      <div class="h-4"></div>
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
  background-color: var(--border-1);
  border-radius: 6px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background-color: var(--border-2);
}
</style>
