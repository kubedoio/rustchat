<script setup lang="ts">
import { computed } from 'vue'
import { format } from 'date-fns'
import { X, MessageSquare } from 'lucide-vue-next'
import type { Post } from '../../api/posts'
import { useAuthStore } from '../../stores/auth'
import RcAvatar from '../ui/RcAvatar.vue'
import FilePreview from '../atomic/FilePreview.vue'
import { useMarkdownRenderer } from '../../composables/useMarkdownRenderer'

interface Props {
  parentPost: Post | null
  replyCount: number
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const authStore = useAuthStore()
const { renderMarkdown } = useMarkdownRenderer()

const formattedTime = computed(() => {
  if (!props.parentPost) return ''
  return format(new Date(props.parentPost.created_at), 'MMM d, h:mm a')
})

const formattedContent = computed(() => {
  if (!props.parentPost) return ''
  return renderMarkdown(props.parentPost.message, authStore.user?.username || undefined)
})

const isDeleted = computed(() => !props.parentPost)
</script>

<template>
  <div class="border-b border-border-1 bg-bg-surface-2">
    <!-- Header Row -->
    <div class="h-12 flex items-center justify-between px-4 border-b border-border-1">
      <div class="flex items-center gap-2">
        <MessageSquare class="w-5 h-5 text-text-2" />
        <h3 class="font-bold text-[15px] text-text-1 uppercase tracking-wider">
          Thread
        </h3>
        <span v-if="replyCount > 0" class="text-sm text-text-3">
          ({{ replyCount }} {{ replyCount === 1 ? 'reply' : 'replies' }})
        </span>
      </div>

      <button
        @click="$emit('close')"
        class="p-1.5 hover:bg-bg-surface-1 rounded-lg text-text-3 hover:text-text-1 transition-standard focus-ring"
        aria-label="Close thread"
        title="Close thread"
      >
        <X class="w-5 h-5" />
      </button>
    </div>

    <!-- Parent Message Display -->
    <div v-if="isDeleted" class="p-5">
      <div class="flex items-center gap-3 text-text-3">
        <div class="w-10 h-10 rounded-lg bg-bg-surface-1 flex items-center justify-center">
          <MessageSquare class="w-5 h-5" />
        </div>
        <div>
          <p class="text-sm font-medium text-text-2">Message deleted</p>
          <p class="text-xs">This message is no longer available</p>
        </div>
      </div>
    </div>

    <div v-else-if="parentPost" class="p-5">
      <div class="flex items-start space-x-3">
        <!-- User Avatar -->
        <RcAvatar
          :userId="parentPost.user_id"
          :username="parentPost.username"
          :src="parentPost.avatar_url"
          size="md"
          class="w-10 h-10 rounded-lg shrink-0 mt-0.5"
        />

        <!-- Content -->
        <div class="flex-1 min-w-0">
          <div class="flex items-baseline space-x-2 mb-1">
            <span class="font-bold text-[15px] text-text-1 leading-tight">
              {{ parentPost.username || 'Unknown User' }}
            </span>
            <span class="text-[11px] text-text-3 font-medium">
              {{ formattedTime }}
            </span>
          </div>

          <div
            class="text-[15px] text-text-1 leading-relaxed markdown-content break-words"
            v-html="formattedContent"
          ></div>

          <!-- Files -->
          <div v-if="parentPost.files && parentPost.files.length > 0" class="mt-4 flex flex-wrap gap-2">
            <FilePreview
              v-for="file in parentPost.files"
              :key="file.id"
              :file="file"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
