<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import RcAvatar from '../ui/RcAvatar.vue'
import { useTeamStore } from '../../stores/teams'

const props = defineProps<{
  query: string
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'select', username: string): void
  (e: 'close'): void
}>()

const teamStore = useTeamStore()
const selectedIndex = ref(0)

const filteredMembers = computed(() => {
  const q = props.query.toLowerCase()
  return teamStore.members
    .filter(m => m.username.toLowerCase().includes(q))
    .slice(0, 8) // Limit to top 8
})

watch(() => props.query, () => {
  selectedIndex.value = 0
})

function select(username: string) {
  emit('select', username)
}

function handleKeydown(e: KeyboardEvent) {
  if (!props.show || filteredMembers.value.length === 0) return

  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedIndex.value = (selectedIndex.value + 1) % filteredMembers.value.length
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedIndex.value = (selectedIndex.value - 1 + filteredMembers.value.length) % filteredMembers.value.length
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const member = filteredMembers.value[selectedIndex.value]
    if (member) {
      select(member.username)
    }
  } else if (e.key === 'Escape') {
    emit('close')
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div 
    v-if="show && filteredMembers.length > 0"
    class="absolute bottom-full left-0 mb-2 w-64 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl overflow-hidden z-50 animate-in fade-in slide-in-from-bottom-2 duration-200"
  >
    <div class="p-2 border-b border-gray-100 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50">
      <span class="text-[10px] font-bold text-gray-500 uppercase tracking-wider">Channel Members</span>
    </div>
    
    <div class="max-h-64 overflow-y-auto">
      <button
        v-for="(member, index) in filteredMembers"
        :key="member.user_id"
        @click="select(member.username)"
        @mouseenter="selectedIndex = index"
        class="w-full px-3 py-2 flex items-center space-x-3 transition-colors text-left"
        :class="selectedIndex === index ? 'bg-blue-50 dark:bg-blue-900/40' : 'hover:bg-gray-50 dark:hover:bg-gray-700/30'"
      >
        <RcAvatar 
          :userId="member.user_id" 
          :username="member.username"
          :src="member.avatar_url" 
          size="sm"
          class="w-6 h-6 rounded"
        />
        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between">
            <span class="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate">
              {{ member.username }}
            </span>
            <span v-if="selectedIndex === index" class="text-[10px] text-blue-500 font-medium">Enter</span>
          </div>
          <p class="text-[11px] text-gray-500 truncate">{{ member.display_name || member.username }}</p>
        </div>
      </button>
    </div>
  </div>
</template>
