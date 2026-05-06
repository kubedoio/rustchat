<script setup lang="ts">
import { ref, computed, watch, onMounted, nextTick } from 'vue'
import { Loader2, MessageSquare } from 'lucide-vue-next'
import type { Post } from '../../api/posts'
import ThreadReplyItem from './ThreadReplyItem.vue'

interface Props {
  replies: Post[]
  hasMore: boolean
  isLoading: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'loadMore'): void
}>()

const listRef = ref<HTMLDivElement | null>(null)
const isScrolledToBottom = ref(true)

const showLoadMore = computed(() => props.hasMore && !props.isLoading)

function handleScroll() {
  if (!listRef.value) return

  const { scrollTop, scrollHeight, clientHeight } = listRef.value
  isScrolledToBottom.value = scrollHeight - scrollTop - clientHeight < 50

  // Auto-load more when scrolling near top
  if (scrollTop < 100 && props.hasMore && !props.isLoading) {
    emit('loadMore')
  }
}

function scrollToBottom(smooth = true) {
  nextTick(() => {
    if (listRef.value) {
      listRef.value.scrollTo({
        top: listRef.value.scrollHeight,
        behavior: smooth ? 'smooth' : 'auto'
      })
    }
  })
}

// Scroll to bottom when new replies are added (if already at bottom)
watch(() => props.replies.length, (newLength, oldLength) => {
  if (newLength > oldLength && isScrolledToBottom.value) {
    scrollToBottom()
  }
})

onMounted(() => {
  // Scroll to bottom on initial mount
  scrollToBottom(false)
})

// Expose scrollToBottom for parent component
defineExpose({
  scrollToBottom
})
</script>

<template>
  <div
    ref="listRef"
    class="flex-1 overflow-y-auto p-5 space-y-5 custom-scrollbar"
    @scroll="handleScroll"
  >
    <!-- Load More Button -->
    <div v-if="showLoadMore" class="flex justify-center py-2">
      <button
        @click="$emit('loadMore')"
        :disabled="isLoading"
        class="px-4 py-2 text-sm text-text-2 hover:text-text-1 bg-bg-surface-2 hover:bg-bg-surface-1 border border-border-1 rounded-lg transition-standard flex items-center gap-2"
      >
        <Loader2 v-if="isLoading" class="w-4 h-4 animate-spin" />
        <span>Load more replies</span>
      </button>
    </div>

    <!-- Loading State -->
    <div v-if="isLoading && replies.length === 0" class="flex flex-col items-center justify-center py-12">
      <Loader2 class="w-8 h-8 text-text-3 animate-spin mb-3" />
      <p class="text-sm text-text-3">Loading replies...</p>
    </div>

    <!-- Empty State -->
    <div v-else-if="replies.length === 0" class="flex flex-col items-center justify-center py-12 text-center">
      <div class="w-16 h-16 bg-bg-surface-2 rounded-full flex items-center justify-center mb-4">
        <MessageSquare class="w-8 h-8 text-text-3" />
      </div>
      <p class="text-[15px] font-semibold text-text-1 mb-1">No replies yet</p>
      <p class="text-sm text-text-3">Be the first to share your thoughts!</p>
    </div>

    <!-- Replies List -->
    <template v-else>
      <ThreadReplyItem
        v-for="reply in replies"
        :key="reply.id"
        :reply="reply"
      />
    </template>
  </div>
</template>
