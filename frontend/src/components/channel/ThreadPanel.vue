<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { Send, MessageSquare } from 'lucide-vue-next'
import { format } from 'date-fns'
import { useMessageStore } from '../../stores/messages'
import { useUIStore } from '../../stores/ui'
import { useAuthStore } from '../../stores/auth'
import { useTeamStore } from '../../stores/teams'
import { useWebSocket } from '../../composables/useWebSocket'
import RcAvatar from '../ui/RcAvatar.vue'
import FilePreview from '../atomic/FilePreview.vue'
import ImageGallery from '../atomic/ImageGallery.vue'
import type { FileUploadResponse } from '../../api/files'
import { threadsApi } from '../../api/threads'
import { useMarkdownRenderer } from '../../composables/useMarkdownRenderer'

const messageStore = useMessageStore()
const uiStore = useUIStore()
const authStore = useAuthStore()
const teamStore = useTeamStore()

const { sendMessage } = useWebSocket()
const { renderMarkdown } = useMarkdownRenderer()

const replyContent = ref('')
const loading = ref(false)

// Gallery state
const showGallery = ref(false)
const galleryInitialIndex = ref(0)
const galleryCurrentImages = ref<FileUploadResponse[]>([])

function openGallery(file: FileUploadResponse, allFiles: FileUploadResponse[]) {
  const images = allFiles.filter(f => f.mime_type.startsWith('image/'))
  const index = images.findIndex(f => f.id === file.id)
  if (index !== -1) {
    galleryCurrentImages.value = images
    galleryInitialIndex.value = index
    showGallery.value = true
  }
}

const parentMessage = computed(() => {
    if (!uiStore.rhsContextId) return null
    for (const channelId in messageStore.messagesByChannel) {
        const messages = messageStore.messagesByChannel[channelId]
        if (!messages) continue;
        const msg = messages.find(m => m.id === uiStore.rhsContextId)
        if (msg) return msg
    }
    return null
})

const replies = computed(() => {
    if (!uiStore.rhsContextId) return []
    return messageStore.repliesByThread[uiStore.rhsContextId] || []
})

watch(() => uiStore.rhsContextId, async (newId) => {
    if (newId && uiStore.rhsView === 'thread') {
        loading.value = true
        try {
            await messageStore.fetchThread(newId)
            // Mark thread as read when opened
            if (teamStore.currentTeamId) {
                try {
                    await threadsApi.markAsRead(newId, teamStore.currentTeamId)
                } catch (e) {
                    console.error('Failed to mark thread as read:', e)
                }
            }
        } catch (e) {
            console.error('Failed to fetch thread:', e)
        } finally {
            loading.value = false
        }
    }
}, { immediate: true })

async function sendReply() {
    if (!replyContent.value.trim() || !parentMessage.value) return
    
    const rootId = parentMessage.value.id
    const content = replyContent.value
    replyContent.value = ''

    try {
        await sendMessage(parentMessage.value.channelId, content, rootId)
    } catch (e) {
        console.error('Failed to send reply:', e)
        replyContent.value = content
    }
}

function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault()
        sendReply()
    }
}
</script>

<template>
  <div 
    v-if="parentMessage"
    class="flex-1 flex flex-col min-h-0 bg-surface"
  >
    <!-- Thread Header (Simplified since RightSidebar provides the main header) -->
    <!-- Parent Message -->
    <div class="p-5 border-b border-border-dim bg-surface-dim/30">
      <div class="flex items-start space-x-3">
        <RcAvatar 
          :userId="parentMessage.userId" 
          :username="parentMessage.username" 
          :src="parentMessage.avatarUrl" 
          size="md"
          class="w-10 h-10 rounded-lg shrink-0 mt-0.5"
        />
        <div class="flex-1 min-w-0">
          <div class="flex items-baseline space-x-2 mb-1">
            <span class="font-bold text-[15px] text-gray-900 leading-tight">{{ parentMessage.username }}</span>
            <span class="text-[11px] text-gray-500 font-medium">{{ format(new Date(parentMessage.timestamp), 'MMM d, h:mm a') }}</span>
          </div>
          <div 
            class="text-[15px] text-gray-800 leading-relaxed markdown-content"
            v-html="renderMarkdown(parentMessage.content, authStore.user?.username || undefined)"
          ></div>

          <!-- Parent Files -->
          <div v-if="parentMessage.files && parentMessage.files.length > 0" class="mt-4 flex flex-wrap gap-2">
            <template v-for="file in parentMessage.files" :key="file.id">
              <FilePreview :file="file" @preview="(f) => openGallery(f, parentMessage!.files || [])" />
            </template>
          </div>
        </div>
      </div>
    </div>

    <!-- Replies Count -->
    <div v-if="replies.length > 0" class="px-5 py-3 border-b border-border-dim text-[11px] font-bold text-gray-500 uppercase tracking-widest bg-surface/50">
      {{ replies.length }} {{ replies.length === 1 ? 'reply' : 'replies' }}
    </div>

    <!-- Replies List -->
    <div class="flex-1 overflow-y-auto p-5 space-y-6 custom-scrollbar">
      <div v-if="loading" class="text-center py-10 text-gray-500">
        <div class="animate-spin w-6 h-6 border-2 border-primary border-t-transparent rounded-full mx-auto mb-3"></div>
        <p class="text-xs font-medium uppercase tracking-wider">Loading replies...</p>
      </div>
      
      <div v-else-if="replies.length === 0" class="text-center py-12">
          <div class="w-16 h-16 bg-surface-dim rounded-full flex items-center justify-center mx-auto mb-4 border border-border-dim">
             <MessageSquare class="w-8 h-8 text-gray-400" />
          </div>
          <p class="text-[15px] font-semibold text-gray-700">No replies yet.</p>
          <p class="text-xs text-gray-500 mt-1">Be the first to share your thoughts!</p>
      </div>

      <div 
        v-else
        v-for="reply in replies"
        :key="reply.id"
        class="flex items-start space-x-3 group relative transition-all"
      >
        <RcAvatar 
          :userId="reply.userId" 
          :username="reply.username" 
          :src="reply.avatarUrl" 
          size="sm"
          class="w-8 h-8 rounded-md shrink-0 mt-0.5"
        />
        <div class="flex-1 min-w-0">
          <div class="flex items-baseline space-x-2 mb-0.5">
            <span class="font-bold text-sm text-gray-900 leading-tight">{{ reply.username }}</span>
            <span class="text-[10px] text-gray-500 font-medium">{{ format(new Date(reply.timestamp), 'h:mm a') }}</span>
          </div>
          <div 
            class="text-[14px] text-gray-700 leading-normal markdown-content"
            v-html="renderMarkdown(reply.content, authStore.user?.username || undefined)"
          ></div>

          <!-- Reply Files -->
          <div v-if="reply.files && reply.files.length > 0" class="mt-3 flex flex-wrap gap-2">
            <template v-for="file in reply.files" :key="file.id">
              <FilePreview :file="file" @preview="(f) => openGallery(f, reply.files || [])" />
            </template>
          </div>
        </div>
      </div>
    </div>

    <!-- Reply Composer -->
    <div class="p-4 border-t border-border-dim bg-surface-dim/30">
      <div class="flex items-end space-x-2 bg-surface border border-border-dim rounded-xl focus-within:ring-2 focus-within:ring-primary/40 focus-within:border-primary/50 transition-all p-1.5 shadow-sm">
        <textarea
          v-model="replyContent"
          @keydown="handleKeydown"
          rows="2"
          class="flex-1 px-3 py-2 bg-transparent text-gray-900 resize-none border-none focus:ring-0 text-[14px] scrollbar-none"
          placeholder="Reply to thread..."
        ></textarea>
        <button
          @click="sendReply"
          :disabled="!replyContent.trim()"
          class="mb-1 mr-1 rounded-lg bg-primary p-2.5 text-brand-foreground shadow-lg shadow-primary/20 transition-all hover:bg-brand-hover active:scale-95 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Send class="w-4 h-4" />
        </button>
      </div>
    </div>

    <!-- Image Gallery Lightbox -->
    <Teleport to="body">
      <ImageGallery 
        v-if="showGallery" 
        :images="galleryCurrentImages" 
        :initialIndex="galleryInitialIndex" 
        @close="showGallery = false" 
      />
    </Teleport>
  </div>

</template>
