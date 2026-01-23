<script setup lang="ts">
import { ref, watch } from 'vue'
import { X, Search, Hash, Clock, MessageSquare } from 'lucide-vue-next'
import { format } from 'date-fns'
import { searchApi, type SearchResult } from '../../api/search'
import { useChannelStore } from '../../stores/channels'

const props = defineProps<{
    show: boolean
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'navigate', channelId: string, postId: string): void
}>()

const channelStore = useChannelStore()

const query = ref('')
const results = ref<SearchResult | null>(null)
const loading = ref(false)
const error = ref('')
const recentSearches = ref<string[]>([])

// Debounced search
let searchTimeout: ReturnType<typeof setTimeout> | null = null

watch(query, (newQuery) => {
    if (searchTimeout) {
        clearTimeout(searchTimeout)
    }
    
    if (newQuery.trim().length < 2) {
        results.value = null
        return
    }
    
    searchTimeout = setTimeout(() => {
        performSearch()
    }, 300)
})

async function performSearch() {
    if (query.value.trim().length < 2) return
    
    loading.value = true
    error.value = ''
    
    try {
        const response = await searchApi.search({
            q: query.value.trim(),
            per_page: 20,
        })
        results.value = response.data
        
        // Save to recent searches
        const search = query.value.trim()
        if (!recentSearches.value.includes(search)) {
            recentSearches.value = [search, ...recentSearches.value.slice(0, 4)]
        }
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Search failed'
    } finally {
        loading.value = false
    }
}

function handleResultClick(channelId: string, postId: string) {
    emit('navigate', channelId, postId)
    emit('close')
}

function handleRecentClick(search: string) {
    query.value = search
    performSearch()
}

function handleClose() {
    query.value = ''
    results.value = null
    error.value = ''
    emit('close')
}

function getChannelName(channelId: string): string {
    const channel = channelStore.channels.find(c => c.id === channelId)
    return channel?.display_name || channel?.name || 'Unknown'
}
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="fixed inset-0 z-50 flex items-start justify-center pt-[10vh]">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/50" @click="handleClose"></div>
      
      <!-- Search Modal -->
      <div class="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-2xl mx-4 overflow-hidden">
        <!-- Search Input -->
        <div class="flex items-center px-4 py-3 border-b border-gray-200 dark:border-gray-700">
          <Search class="w-5 h-5 text-gray-400 mr-3" />
          <input
            v-model="query"
            type="text"
            placeholder="Search messages, files, and more..."
            class="flex-1 bg-transparent text-lg text-gray-900 dark:text-white placeholder-gray-400 focus:outline-none"
            autofocus
          />
          <div class="flex items-center space-x-2">
            <kbd class="hidden sm:block px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700 text-gray-500 rounded">ESC</kbd>
            <button @click="handleClose" class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded">
              <X class="w-5 h-5 text-gray-400" />
            </button>
          </div>
        </div>

        <!-- Results Area -->
        <div class="max-h-[60vh] overflow-y-auto">
          <!-- Loading -->
          <div v-if="loading" class="p-8 text-center text-gray-500">
            <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full mx-auto"></div>
            <p class="mt-2">Searching...</p>
          </div>

          <!-- Error -->
          <div v-else-if="error" class="p-8 text-center text-red-500">
            {{ error }}
          </div>

          <!-- Results -->
          <div v-else-if="results && results.posts.length > 0" class="py-2">
            <div class="px-4 py-2 text-xs font-semibold text-gray-500 uppercase">
              {{ results.total }} Results
            </div>
            <div 
              v-for="post in results.posts"
              :key="post.id"
              @click="handleResultClick(post.channel_id, post.id)"
              class="px-4 py-3 hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer border-b border-gray-100 dark:border-gray-700 last:border-0"
            >
              <div class="flex items-center space-x-2 text-xs text-gray-500 mb-1">
                <Hash class="w-3 h-3" />
                <span>{{ getChannelName(post.channel_id) }}</span>
                <span>•</span>
                <Clock class="w-3 h-3" />
                <span>{{ format(new Date(post.created_at), 'MMM d, yyyy') }}</span>
              </div>
              <div class="text-sm text-gray-900 dark:text-white line-clamp-2">
                {{ post.message }}
              </div>
            </div>
          </div>

          <!-- No Results -->
          <div v-else-if="results && results.posts.length === 0" class="p-8 text-center text-gray-500">
            <MessageSquare class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>No results found for "{{ query }}"</p>
          </div>

          <!-- Empty State / Recent Searches -->
          <div v-else class="p-4">
            <div v-if="recentSearches.length > 0">
              <div class="text-xs font-semibold text-gray-500 uppercase mb-2">Recent Searches</div>
              <div class="space-y-1">
                <button
                  v-for="search in recentSearches"
                  :key="search"
                  @click="handleRecentClick(search)"
                  class="flex items-center w-full px-3 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg"
                >
                  <Clock class="w-4 h-4 mr-2 text-gray-400" />
                  {{ search }}
                </button>
              </div>
            </div>
            <div v-else class="text-center text-gray-500 py-6">
              <Search class="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>Start typing to search messages</p>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="px-4 py-2 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900 text-xs text-gray-500 flex items-center justify-between">
          <span>Search powered by PostgreSQL full-text search</span>
          <span>↵ to select</span>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
