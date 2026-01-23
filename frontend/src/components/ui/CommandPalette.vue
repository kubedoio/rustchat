<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { Search, Hash, User } from 'lucide-vue-next'

const isOpen = ref(false)
const searchQuery = ref('')
const selectedIndex = ref(0)
const inputRef = ref<HTMLInputElement | null>(null)
const router = useRouter()

// Placeholder data - replace with store/API later
const items = [
  { id: '1', type: 'channel', name: 'general', group: 'Channels' },
  { id: '2', type: 'channel', name: 'random', group: 'Channels' },
  { id: '3', type: 'channel', name: 'devops', group: 'Channels' },
  { id: '4', type: 'user', name: 'Alice', group: 'Direct Messages' },
  { id: '5', type: 'user', name: 'Bob', group: 'Direct Messages' },
]

const filteredItems = computed(() => {
  if (!searchQuery.value) return items.slice(0, 5) // Recent
  return items.filter(item => 
    item.name.toLowerCase().includes(searchQuery.value.toLowerCase())
  )
})

function open() {
  isOpen.value = true
  searchQuery.value = ''
  selectedIndex.value = 0
  setTimeout(() => inputRef.value?.focus(), 50)
}

function close() {
  isOpen.value = false
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
    e.preventDefault()
    if (isOpen.value) close()
    else open()
  } else if (e.key === 'Escape' && isOpen.value) {
    close()
  } else if (isOpen.value) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      selectedIndex.value = (selectedIndex.value + 1) % filteredItems.value.length
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      selectedIndex.value = (selectedIndex.value - 1 + filteredItems.value.length) % filteredItems.value.length
    } else if (e.key === 'Enter') {
      e.preventDefault()
      selectItem(filteredItems.value[selectedIndex.value])
    }
  }
}

function selectItem(item: any) {
  console.log('Selected:', item)
  if (item.type === 'channel') {
      router.push(`/channels/${item.id}`)
  }
  // Navigate
  close()
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onUnmounted(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 overflow-y-auto p-4 sm:p-6 md:p-20" role="dialog" @click="close">
    <!-- Backdrop -->
    <div class="fixed inset-0 bg-gray-500 bg-opacity-25 transition-opacity backdrop-blur-sm"></div>

    <!-- Modal -->
    <div 
        class="mx-auto max-w-xl transform divide-y divide-gray-100 overflow-hidden rounded-xl bg-white dark:bg-gray-800 shadow-2xl ring-1 ring-black ring-opacity-5 transition-all"
        @click.stop
    >
      <div class="relative">
        <Search class="pointer-events-none absolute top-3.5 left-4 h-5 w-5 text-gray-400" />
        <input
          ref="inputRef"
          type="text"
          class="h-12 w-full border-0 bg-transparent pl-11 pr-4 text-gray-800 dark:text-white placeholder-gray-400 focus:ring-0 sm:text-sm"
          placeholder="Search items..."
          v-model="searchQuery"
        />
      </div>

      <div v-if="filteredItems.length > 0" class="max-h-96 scroll-py-3 overflow-y-auto p-3">
         <div v-for="(item, index) in filteredItems" :key="item.id">
             <!-- Group Header (Optional logic needed) -->
             
             <div
                @click="selectItem(item)"
                @mouseenter="selectedIndex = index"
                class="group flex cursor-default select-none rounded-xl p-3"
                :class="selectedIndex === index ? 'bg-gray-100 dark:bg-gray-700' : ''"
             >
                <div 
                    class="flex h-10 w-10 flex-none items-center justify-center rounded-lg"
                    :class="selectedIndex === index ? 'bg-white dark:bg-gray-600' : 'bg-gray-100 dark:bg-gray-700'"
                >
                    <Hash v-if="item.type === 'channel'" class="h-6 w-6 text-gray-500" />
                    <User v-else class="h-6 w-6 text-gray-500" />
                </div>
                <div class="ml-4 flex-auto">
                    <p class="text-sm font-medium text-gray-700 dark:text-gray-200" :class="selectedIndex === index ? 'text-gray-900 dark:text-white' : ''">
                        {{ item.name }}
                    </p>
                    <p class="text-sm text-gray-500 dark:text-gray-400">
                        {{ item.group }}
                    </p>
                </div>
             </div>
         </div>
      </div>
      
      <div v-else class="py-14 px-6 text-center text-sm sm:px-14">
          <p class="mt-4 font-semibold text-gray-900 dark:text-white">No results found</p>
          <p class="mt-2 text-gray-500">No components found for this search term. Please try again.</p>
      </div>
    </div>
  </div>
</template>
