<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { X, Send, MessageSquare } from 'lucide-vue-next'
import { format } from 'date-fns'
import { useMessageStore } from '../../stores/messages'
import { useUIStore } from '../../stores/ui'
import { useAuthStore } from '../../stores/auth'
import { useWebSocket } from '../../composables/useWebSocket'
import RcAvatar from '../ui/RcAvatar.vue'
import FilePreview from '../atomic/FilePreview.vue'
import ImageGallery from '../atomic/ImageGallery.vue'
import type { FileUploadResponse } from '../../api/files'
import { renderMarkdown } from '../../utils/markdown'

const messageStore = useMessageStore()
const uiStore = useUIStore()
const authStore = useAuthStore()

const { sendMessage } = useWebSocket()

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
  <aside 
    v-if="uiStore.rhsView === 'thread' && parentMessage"
    class="w-[400px] h-full bg-white dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 flex flex-col shrink-0 z-20"
  >
    <!-- Header -->
    <div class="h-12 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between px-4">
      <div class="flex items-center space-x-2">
        <MessageSquare class="w-5 h-5 text-gray-400" />
        <span class="font-semibold text-gray-900 dark:text-white">Thread</span>
      </div>
      <button 
        @click="uiStore.closeRhs()"
        class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
      >
        <X class="w-5 h-5 text-gray-400" />
      </button>
    </div>

    <!-- Parent Message -->
    <div class="p-4 bg-gray-50/50 dark:bg-gray-900/30 border-b border-gray-200 dark:border-gray-700">
      <div class="flex items-start space-x-3">
        <RcAvatar 
          :userId="parentMessage.userId" 
          :username="parentMessage.username" 
          :src="parentMessage.avatarUrl" 
          size="sm"
          class="w-8 h-8 rounded shrink-0"
        />
        <div class="flex-1 min-w-0">
          <div class="flex items-baseline space-x-2">
            <span class="font-bold text-sm text-gray-900 dark:text-white leading-tight">{{ parentMessage.username }}</span>
            <span class="text-[10px] text-gray-500">{{ format(new Date(parentMessage.timestamp), 'MMM d, h:mm a') }}</span>
          </div>
          <div 
            class="text-sm text-gray-800 dark:text-gray-200 mt-1 leading-relaxed markdown-content"
            v-html="renderMarkdown(parentMessage.content, authStore.user?.username || undefined)"
          ></div>

          <!-- Parent Files -->
          <div v-if="parentMessage.files && parentMessage.files.length > 0" class="mt-3 flex flex-wrap gap-2">
            <template v-for="file in parentMessage.files" :key="file.id">
              <FilePreview :file="file" @preview="(f) => openGallery(f, parentMessage!.files || [])" />
            </template>
          </div>
        </div>
      </div>
    </div>

    <!-- Replies Count -->
    <div v-if="replies.length > 0" class="px-4 py-2 border-b border-gray-100 dark:border-gray-800 text-[11px] font-medium text-gray-500 uppercase tracking-wider">
      {{ replies.length }} {{ replies.length === 1 ? 'reply' : 'replies' }}
    </div>

    <!-- Replies List -->
    <div class="flex-1 overflow-y-auto p-4 space-y-5">
      <div v-if="loading" class="text-center py-10 text-gray-500">
        <div class="animate-spin w-5 h-5 border-2 border-primary border-t-transparent rounded-full mx-auto mb-2"></div>
        <p class="text-xs">Loading replies...</p>
      </div>
      
      <div v-else-if="replies.length === 0" class="text-center py-10">
          <div class="w-12 h-12 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mx-auto mb-3">
             <MessageSquare class="w-6 h-6 text-gray-400" />
          </div>
          <p class="text-sm text-gray-500">No replies yet.</p>
          <p class="text-xs text-gray-400 mt-1">Be the first to reply!</p>
      </div>

      <div 
        v-else
        v-for="reply in replies"
        :key="reply.id"
        class="flex items-start space-x-3 group"
      >
        <RcAvatar 
          :userId="reply.userId" 
          :username="reply.username" 
          :src="reply.avatarUrl" 
          size="sm"
          class="w-7 h-7 rounded shrink-0"
        />
        <div class="flex-1 min-w-0">
          <div class="flex items-baseline space-x-2">
            <span class="font-bold text-sm text-gray-900 dark:text-gray-100 leading-tight">{{ reply.username }}</span>
            <span class="text-[10px] text-gray-500">{{ format(new Date(reply.timestamp), 'h:mm a') }}</span>
          </div>
          <div 
            class="text-[13px] text-gray-700 dark:text-gray-300 mt-0.5 leading-normal markdown-content"
            v-html="renderMarkdown(reply.content, authStore.user?.username || undefined)"
          ></div>

          <!-- Reply Files -->
          <div v-if="reply.files && reply.files.length > 0" class="mt-2 flex flex-wrap gap-2">
            <template v-for="file in reply.files" :key="file.id">
              <FilePreview :file="file" @preview="(f) => openGallery(f, reply.files || [])" />
            </template>
          </div>
        </div>
      </div>
    </div>

    <!-- Reply Composer -->
    <div class="p-3 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
      <div class="flex items-end space-x-2 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg focus-within:ring-1 focus-within:ring-primary focus-within:border-primary transition-all p-1">
        <textarea
          v-model="replyContent"
          @keydown="handleKeydown"
          rows="2"
          class="flex-1 px-3 py-2 bg-transparent text-gray-900 dark:text-white resize-none border-none focus:ring-0 text-[13px]"
          placeholder="Reply to thread..."
        ></textarea>
        <button
          @click="sendReply"
          :disabled="!replyContent.trim()"
          class="p-2 bg-primary text-white rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-blue-600 transition-colors mb-1 mr-1"
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
  </aside>
</template>
