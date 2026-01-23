<script setup lang="ts">
import { ref, watch } from 'vue'
import { X, Pin, ExternalLink } from 'lucide-vue-next'
import { format } from 'date-fns'
import type { Message } from '../../stores/messages'
import { useMessageStore } from '../../stores/messages'
import { useChannelStore } from '../../stores/channels'

const props = defineProps<{
    show: boolean
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'jump', messageId: string): void
}>()

const messageStore = useMessageStore()
const channelStore = useChannelStore()

const pinnedMessages = ref<Message[]>([])
const loading = ref(false)

async function loadPinnedMessages() {
    if (!channelStore.currentChannelId) return
    loading.value = true
    try {
        pinnedMessages.value = await messageStore.fetchPinnedMessages(channelStore.currentChannelId)
    } catch (e) {
        console.error('Failed to fetch pinned messages', e)
    } finally {
        loading.value = false
    }
}

watch(() => props.show, (isOpen) => {
    if (isOpen) {
        loadPinnedMessages()
    }
})

// Also reload if channel changes while open
watch(() => channelStore.currentChannelId, (newId) => {
    if (props.show && newId) {
        loadPinnedMessages()
    }
})

async function handleUnpin(message: Message) {
    try {
        await messageStore.unpinMessage(message.id, message.channelId)
        pinnedMessages.value = pinnedMessages.value.filter(m => m.id !== message.id)
    } catch (e) {
        console.error('Failed to unpin message', e)
    }
}

function jumpToMessage(message: Message) {
    emit('jump', message.id)
}
</script>

<template>
  <aside 
    v-if="show"
    class="w-[400px] bg-white dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 flex flex-col shrink-0 z-20"
  >
    <!-- Header -->
    <div class="h-12 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between px-4">
      <div class="flex items-center space-x-2">
        <Pin class="w-5 h-5 text-gray-500 fill-current" />
        <span class="font-semibold text-gray-900 dark:text-white">Pinned Items</span>
      </div>
      <button 
        @click="$emit('close')"
        class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
      >
        <X class="w-5 h-5 text-gray-400" />
      </button>
    </div>

    <!-- Pinned List -->
    <div class="flex-1 overflow-y-auto p-0">
      <div v-if="loading" class="text-center py-8 text-gray-500">
        <div class="animate-spin w-6 h-6 border-2 border-primary border-t-transparent rounded-full mx-auto mb-2"></div>
        Loading pinned items...
      </div>
      
      <div v-else-if="pinnedMessages.length === 0" class="text-center py-8 text-gray-500 px-4">
        <div class="mb-2 text-gray-400">
            <Pin class="w-12 h-12 mx-auto mb-3 opacity-20" />
            No pinned items yet
        </div>
        <div class="text-xs">Pin important messages to find them easily here</div>
      </div>

      <div v-else class="divide-y divide-gray-100 dark:divide-gray-800">
        <div 
            v-for="message in pinnedMessages" 
            :key="message.id"
            class="px-4 py-4 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors group relative"
        >
            <div class="flex items-start justify-between mb-1">
                <div class="flex items-center space-x-2">
                    <span class="font-bold text-sm text-gray-900 dark:text-gray-100">{{ message.username }}</span>
                    <span class="text-[10px] text-gray-400">{{ format(new Date(message.timestamp), 'MMM d, h:mm a') }}</span>
                </div>
                <!-- Actions -->
                <div class="flex items-center space-x-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button 
                        @click="handleUnpin(message)"
                        class="p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded text-gray-400 hover:text-red-500 transition-colors"
                        title="Unpin"
                    >
                        <Pin class="w-3.5 h-3.5 rotate-45" />
                    </button>
                    <button 
                        @click="jumpToMessage(message)"
                        class="p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded text-gray-400 hover:text-blue-500 transition-colors"
                        title="Jump to message"
                    >
                        <ExternalLink class="w-3.5 h-3.5" />
                    </button>
                </div>
            </div>
            <div class="text-sm text-gray-700 dark:text-gray-300 line-clamp-4 mt-1">
                {{ message.content }}
            </div>
            <div v-if="message.files && message.files.length > 0" class="mt-2 text-[10px] text-blue-500 flex items-center">
                <span class="mr-1">{{ message.files.length }} attachment{{ message.files.length > 1 ? 's' : '' }}</span>
            </div>
        </div>
      </div>
    </div>
  </aside>
</template>
