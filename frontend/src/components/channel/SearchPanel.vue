<script setup lang="ts">
import { ref, watch } from 'vue'
import { Search, ExternalLink } from 'lucide-vue-next'
import { format } from 'date-fns'
import type { Message } from '../../stores/messages'
import { useMessageStore } from '../../stores/messages'


const props = defineProps<{
    channelId: string
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'jump', messageId: string): void
}>()

const messageStore = useMessageStore()

const searchQuery = ref('')
const searchResults = ref<Message[]>([])
const loading = ref(false)

async function handleSearch() {
    if (!searchQuery.value.trim() || !props.channelId) {
        searchResults.value = []
        return
    }
    
    loading.value = true
    try {
        searchResults.value = await messageStore.searchMessages(
            props.channelId, 
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
  <div 
    class="flex-1 flex flex-col min-h-0 bg-bg-surface-1"
  >
    <!-- Search Input Area -->
    <div class="p-sp-4 border-b border-border-1 bg-bg-surface-2/30">
        <div class="relative group">
            <Search class="absolute left-sp-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-3 group-focus-within:text-brand transition-standard" />
            <input 
                v-model="searchQuery"
                type="text"
                placeholder="Search messages..."
                class="w-full pl-sp-7 pr-4 py-2 bg-bg-surface-1 border border-border-1 rounded-r-2 text-sm focus-ring shadow-1 transition-standard"
                autofocus
                aria-label="Search messages in this channel"
            />
        </div>
    </div>

    <!-- Results List -->
    <div class="flex-1 overflow-y-auto custom-scrollbar">
      <div v-if="loading" class="text-center py-12 text-gray-500">
        <div class="animate-spin w-6 h-6 border-2 border-primary border-t-transparent rounded-full mx-auto mb-3"></div>
        <p class="text-xs font-medium uppercase tracking-widest">Searching...</p>
      </div>
      
      <div v-else-if="searchQuery && searchResults.length === 0" class="text-center py-12 text-gray-500 px-6">
        <div class="w-16 h-16 bg-surface-dim rounded-full flex items-center justify-center mx-auto mb-4 border border-border-dim">
             <Search class="w-8 h-8 text-gray-300" />
        </div>
        <p class="text-[15px] font-semibold text-gray-700">No results found</p>
        <p class="text-xs text-gray-500 mt-1">We couldn't find anything matching "{{ searchQuery }}"</p>
      </div>

      <div v-else-if="!searchQuery" class="text-center py-16 text-gray-400 px-6">
        <Search class="w-16 h-16 mx-auto mb-4 opacity-10" />
        <p class="text-sm font-medium">Search for messages and files in this channel</p>
      </div>

      <div v-else class="divide-y divide-border-1">
        <div 
            v-for="message in searchResults" 
            :key="message.id"
            class="px-sp-5 py-sp-5 hover:bg-bg-surface-2 transition-standard group relative cursor-pointer border-l-2 border-transparent hover:border-brand/50"
            @click="jumpToMessage(message)"
        >
            <div class="flex items-start justify-between mb-sp-1">
                <div class="flex items-center space-x-sp-2">
                    <span class="font-semibold text-sm text-text-1 hover:text-brand transition-standard">{{ message.username }}</span>
                    <span class="text-xs text-text-3 font-medium tracking-tight">{{ format(new Date(message.timestamp), 'MMM d, h:mm a') }}</span>
                </div>
                <div class="opacity-0 group-hover:opacity-100 transition-standard transform translate-x-1 group-hover:translate-x-0">
                    <button class="text-text-3 hover:text-brand p-1.5 rounded-r-1 hover:bg-brand/5 transition-standard" title="Jump to message">
                        <ExternalLink class="w-4 h-4" />
                    </button>
                </div>
            </div>
            <div class="text-[14px] text-text-2 line-clamp-3 mt-1 leading-relaxed">
                {{ message.content }}
            </div>
        </div>
      </div>
    </div>
  </div>

</template>
