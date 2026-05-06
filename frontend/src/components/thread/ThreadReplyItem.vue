<script setup lang="ts">
import { computed } from 'vue'
import { format } from 'date-fns'
import type { Post } from '../../api/posts'
import { useAuthStore } from '../../stores/auth'
import RcAvatar from '../ui/RcAvatar.vue'
import FilePreview from '../atomic/FilePreview.vue'
import { useMarkdownRenderer } from '../../composables/useMarkdownRenderer'
import { getEmojiChar } from '../../utils/emoji'

interface Props {
  reply: Post
}

const props = defineProps<Props>()

const authStore = useAuthStore()
const { renderMarkdown } = useMarkdownRenderer()

const formattedTime = computed(() => {
  return format(new Date(props.reply.created_at), 'h:mm a')
})

const formattedContent = computed(() => {
  return renderMarkdown(props.reply.message, authStore.user?.username || undefined)
})

const hasReactions = computed(() => {
  return props.reply.reactions && props.reply.reactions.length > 0
})
</script>

<template>
  <div class="flex items-start space-x-3 group">
    <!-- User Avatar -->
    <div class="shrink-0 mt-0.5">
      <RcAvatar
        :userId="reply.user_id"
        :username="reply.username"
        :src="reply.avatar_url"
        size="sm"
        class="w-8 h-8 rounded-lg"
      />
    </div>

    <!-- Content -->
    <div class="flex-1 min-w-0">
      <!-- Header -->
      <div class="flex items-baseline space-x-2 mb-0.5">
        <span class="font-bold text-sm text-text-1 leading-tight">
          {{ reply.username || 'Unknown User' }}
        </span>
        <span class="text-[11px] text-text-3 font-medium">
          {{ formattedTime }}
        </span>
      </div>

      <!-- Message Content -->
      <div
        class="text-[14px] text-text-2 leading-normal markdown-content break-words"
        v-html="formattedContent"
      ></div>

      <!-- Files -->
      <div v-if="reply.files && reply.files.length > 0" class="mt-3 flex flex-wrap gap-2">
        <FilePreview
          v-for="file in reply.files"
          :key="file.id"
          :file="file"
        />
      </div>

      <!-- Reactions -->
      <div v-if="hasReactions" class="flex items-center mt-2 gap-1.5 flex-wrap">
        <div
          v-for="reaction in reply.reactions"
          :key="reaction.emoji"
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border"
          :class="reaction.users.includes(authStore.user?.id || '')
            ? 'bg-brand/10 border-brand/30 text-brand'
            : 'bg-bg-surface-2 border-border-1 text-text-2'"
        >
          <span>{{ getEmojiChar(reaction.emoji) }}</span>
          <span class="font-medium">{{ reaction.count }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
