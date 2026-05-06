<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { Send, Loader2 } from 'lucide-vue-next'
import { useThreadStore } from '../../features/messages/stores/threadStore'
import { useUIStore } from '../../stores/ui'
import ThreadHeader from './ThreadHeader.vue'
import ThreadReplyList from './ThreadReplyList.vue'

const threadStore = useThreadStore()
const uiStore = useUIStore()

const replyListRef = ref<InstanceType<typeof ThreadReplyList> | null>(null)
const composerRef = ref<HTMLTextAreaElement | null>(null)

// Handle keyboard shortcuts
function handleKeydown(e: KeyboardEvent) {
  // Close on Escape
  if (e.key === 'Escape' && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
    e.preventDefault()
    closeThread()
    return
  }

  // Send on Enter (without shift)
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    sendReply()
  }
}

// Handle click outside to close
function handleClickOutside(e: MouseEvent) {
  const target = e.target as HTMLElement
  // Only close if clicking on the overlay background, not the panel itself
  if (target.classList.contains('thread-panel-overlay')) {
    closeThread()
  }
}

function closeThread() {
  threadStore.closeThread()
  uiStore.closeRhs()
}

async function sendReply() {
  if (!threadStore.draft.trim() || threadStore.isSending) return

  try {
    await threadStore.sendReply(threadStore.draft.trim())
    // Scroll to bottom after sending
    replyListRef.value?.scrollToBottom()
  } catch (error) {
    console.error('Failed to send reply:', error)
  }
}

function handleLoadMore() {
  threadStore.loadMoreReplies()
}

// Focus composer when thread opens
watch(() => threadStore.isOpen, (isOpen) => {
  if (isOpen) {
    setTimeout(() => {
      composerRef.value?.focus()
    }, 100)
  }
})

// Handle global escape key
function handleGlobalKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && threadStore.isOpen) {
    // Only close if not in an input/textarea (unless it's our composer)
    const activeElement = document.activeElement
    const isInComposer = activeElement === composerRef.value
    const isInOtherInput = activeElement instanceof HTMLInputElement ||
                           (activeElement instanceof HTMLTextAreaElement && !isInComposer)

    if (!isInOtherInput || isInComposer) {
      closeThread()
    }
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleGlobalKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<template>
  <!-- Overlay for mobile -->
  <div
    v-if="threadStore.isOpen"
    class="thread-panel-overlay fixed inset-0 bg-black/40 backdrop-blur-sm z-30 lg:hidden"
    @click="handleClickOutside"
  ></div>

  <!-- Thread Panel -->
  <div
    v-if="threadStore.isOpen"
    class="fixed lg:relative top-0 right-0 h-full w-full sm:w-[400px] lg:w-[var(--rhs-width)] bg-bg-surface-1 border-l border-border-1 z-40 flex flex-col shadow-2xl"
    @keydown="handleKeydown"
  >
    <!-- Header with Parent Message -->
    <ThreadHeader
      :parentPost="threadStore.parentPost"
      :replyCount="threadStore.replyCount"
      @close="closeThread"
    />

    <!-- Replies List -->
    <ThreadReplyList
      ref="replyListRef"
      :replies="threadStore.replies"
      :hasMore="threadStore.hasMore"
      :isLoading="threadStore.isLoading"
      @loadMore="handleLoadMore"
    />

    <!-- Reply Composer -->
    <div class="p-4 border-t border-border-1 bg-bg-surface-2">
      <div
        class="flex items-end space-x-2 bg-bg-surface-1 border border-border-1 rounded-xl focus-within:ring-2 focus-within:ring-brand/40 focus-within:border-brand/50 transition-all p-1.5 shadow-sm"
      >
        <textarea
          ref="composerRef"
          v-model="threadStore.draft"
          @keydown="handleKeydown"
          rows="2"
          class="flex-1 px-3 py-2 bg-transparent text-text-1 resize-none border-none focus:ring-0 text-[14px] scrollbar-none"
          placeholder="Reply to thread..."
          :disabled="threadStore.isSending"
        ></textarea>

        <button
          @click="sendReply"
          :disabled="!threadStore.draft.trim() || threadStore.isSending"
          class="mb-1 mr-1 flex items-center justify-center rounded-lg bg-brand p-2.5 text-brand-foreground shadow-lg shadow-brand/20 transition-all hover:bg-brand-hover active:scale-95 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Loader2 v-if="threadStore.isSending" class="w-4 h-4 animate-spin" />
          <Send v-else class="w-4 h-4" />
        </button>
      </div>

      <!-- Keyboard hint -->
      <div class="mt-2 text-[11px] text-text-3 text-right">
        <span>Press </span>
        <kbd class="px-1.5 py-0.5 bg-bg-surface-1 border border-border-1 rounded text-[10px] font-mono">Enter</kbd>
        <span> to send, </span>
        <kbd class="px-1.5 py-0.5 bg-bg-surface-1 border border-border-1 rounded text-[10px] font-mono">Shift+Enter</kbd>
        <span> for new line</span>
      </div>
    </div>
  </div>
</template>
