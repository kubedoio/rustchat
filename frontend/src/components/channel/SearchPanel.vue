<script setup lang="ts">
import { ref, watch } from 'vue'
import { X, Search, ExternalLink } from 'lucide-vue-next'
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

const searchQuery = ref('')
const searchResults = ref<Message[]>([])
const loading = ref(false)

async function handleSearch() {
    if (!searchQuery.value.trim() || !channelStore.currentChannelId) {
        searchResults.value = []
        return
    }
    
    loading.value = true
    try {
        searchResults.value = await messageStore.searchMessages(
            channelStore.currentChannelId, 
            searchQuery.value
        )
    } catch (e) {
        console.error('Search failed', e)
    } finally {
        loading.value = false
    }
}

// Debounce search
let timeout: any
watch(searchQuery, () => {
    clearTimeout(timeout)
    timeout = setTimeout(() => {
        handleSearch()
    }, 300)
})

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
        <Search class="w-5 h-5 text-gray-500" />
        <span class="font-semibold text-gray-900 dark:text-white">Search</span>
      </div>
      <button 
        @click="$emit('close')"
        class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
      >
        <X class="w-5 h-5 text-gray-400" />
      </button>
    </div>

    <!-- Search Input -->
    <div class="p-4 border-b border-gray-100 dark:border-gray-700">
        <div class="relative">
            <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
            <input 
                v-model="searchQuery"
                type="text"
                placeholder="Search in channel..."
                class="w-full pl-9 pr-4 py-2 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-primary/20 focus:border-primary transition-all"
                autofocus
            />
        </div>
    </div>

    <!-- Results List -->
    <div class="flex-1 overflow-y-auto p-0">
      <div v-if="loading" class="text-center py-8 text-gray-500">
        <div class="animate-spin w-6 h-6 border-2 border-primary border-t-transparent rounded-full mx-auto mb-2"></div>
        Searching...
      </div>
      
      <div v-else-if="searchQuery && searchResults.length === 0" class="text-center py-8 text-gray-500 px-4">
        <p>No results found for "{{ searchQuery }}"</p>
      </div>

      <div v-else-if="!searchQuery" class="text-center py-12 text-gray-400 px-4">
        <Search class="w-12 h-12 mx-auto mb-3 opacity-20" />
        <p class="text-sm">Search for messages in this channel</p>
      </div>

      <div v-else class="divide-y divide-gray-100 dark:divide-gray-800">
        <div 
            v-for="message in searchResults" 
            :key="message.id"
            class="px-4 py-4 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors group relative cursor-pointer"
            @click="jumpToMessage(message)"
        >
            <div class="flex items-start justify-between mb-1">
                <div class="flex items-center space-x-2">
                    <span class="font-bold text-sm text-gray-900 dark:text-gray-100">{{ message.username }}</span>
                    <span class="text-[10px] text-gray-400">{{ format(new Date(message.timestamp), 'MMM d, h:mm a') }}</span>
                </div>
                <div class="opacity-0 group-hover:opacity-100 transition-opacity">
                    <button class="text-gray-400 hover:text-blue-500" title="Jump to message">
                        <ExternalLink class="w-3.5 h-3.5" />
                    </button>
                </div>
            </div>
            <div class="text-sm text-gray-700 dark:text-gray-300 line-clamp-3 mt-1">
                {{ message.content }}
            </div>
        </div>
      </div>
    </div>
  </aside>
</template>
