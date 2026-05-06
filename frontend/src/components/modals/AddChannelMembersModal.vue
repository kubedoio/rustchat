<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { X, Search, User, UserPlus } from 'lucide-vue-next';
import { useTeamStore } from '../../stores/teams';
import { useAuthStore } from '../../stores/auth';
import BaseButton from '../atomic/BaseButton.vue';
import { channelRepository } from '../../features/channels/repositories/channelRepository';

const props = defineProps<{
    show: boolean
    channelId?: string
    channelName?: string
}>()

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'members-added', count: number): void
}>()

const teamStore = useTeamStore()
const authStore = useAuthStore()

const search = ref('')
const addingMembers = ref<Set<string>>(new Set())
const error = ref('')
const success = ref('')

// Get current channel members to exclude
const currentMembers = ref<Set<string>>(new Set())

watch(() => props.show, async (isShown) => {
    if (isShown) {
        search.value = ''
        error.value = ''
        success.value = ''
        if (teamStore.currentTeamId) {
            await teamStore.fetchMembers(teamStore.currentTeamId)
        }
        // Fetch current channel members
        if (props.channelId) {
            try {
                const members = await channelRepository.getMembers(props.channelId)
                currentMembers.value = new Set(members.map(m => m.userId))
            } catch (e) {
                console.error('Failed to fetch channel members:', e)
            }
        }
    }
})

const filteredMembers = computed(() => {
    if (!teamStore.members) return []
    
    return teamStore.members.filter((m: any) => {
        // Don't show current user or existing members
        if (m.user_id === authStore.user?.id) return false
        if (currentMembers.value.has(m.user_id)) return false
        
        const searchLower = search.value.toLowerCase()
        return (
            m.username.toLowerCase().includes(searchLower) ||
            (m.display_name && m.display_name.toLowerCase().includes(searchLower))
        )
    })
})

async function addMember(member: any) {
    if (!props.channelId || addingMembers.value.has(member.user_id)) return

    addingMembers.value.add(member.user_id)
    error.value = ''
    success.value = ''

    try {
        await channelRepository.addMember(props.channelId, member.user_id)
        currentMembers.value.add(member.user_id)
        success.value = `Added ${member.display_name || member.username}`
        
        // Clear success message after 2 seconds
        setTimeout(() => {
            success.value = ''
        }, 2000)
    } catch (e: any) {
        error.value = e.response?.data?.message || `Failed to add ${member.username}`
    } finally {
        addingMembers.value.delete(member.user_id)
    }
}

function handleClose() {
    const addedCount = success.value ? 1 : 0
    search.value = ''
    error.value = ''
    success.value = ''
    emit('close')
    if (addedCount > 0) {
        emit('members-added', addedCount)
    }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" @click="handleClose"></div>
      
      <!-- Modal -->
      <div class="relative bg-white rounded-xl shadow-2xl w-full max-w-md mx-4 overflow-hidden flex flex-col max-h-[80vh]">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200">
          <div>
            <h2 class="text-xl font-bold text-gray-900">Add Members</h2>
            <p v-if="channelName" class="text-sm text-gray-500 mt-0.5">To #{{ channelName }}</p>
          </div>
          <button @click="handleClose" class="p-1 hover:bg-gray-100 rounded-lg transition-colors">
            <X class="w-5 h-5 text-gray-500" />
          </button>
        </div>

        <!-- Search -->
        <div class="px-6 py-4 border-b border-gray-100">
          <div class="relative">
            <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
            <input 
              v-model="search"
              type="text"
              placeholder="Search for team members..."
              class="w-full pl-10 pr-4 py-2 bg-gray-50 border border-gray-200 rounded-lg text-sm text-gray-900 focus:ring-2 focus:ring-indigo-500 focus:border-transparent outline-none transition-all"
              autofocus
            />
          </div>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-2 custom-scrollbar min-h-[200px]">
          <!-- Error -->
          <div v-if="error" class="m-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm">
            {{ error }}
          </div>

          <!-- Success -->
          <div v-if="success" class="m-4 p-3 bg-green-50 border border-green-200 rounded-lg text-green-600 text-sm">
            {{ success }}
          </div>

          <!-- Loading -->
          <div v-if="teamStore.loading && teamStore.members.length === 0" class="flex flex-col items-center justify-center py-12 text-gray-500">
            <div class="w-8 h-8 border-4 border-indigo-500/20 border-t-indigo-500 rounded-full animate-spin mb-4"></div>
            <p class="text-sm">Loading members...</p>
          </div>

          <!-- Members List -->
          <div v-else-if="filteredMembers.length > 0" class="space-y-1">
            <button
              v-for="member in filteredMembers"
              :key="member.user_id"
              @click="addMember(member)"
              :disabled="addingMembers.has(member.user_id)"
              class="w-full flex items-center justify-between px-4 py-3 rounded-lg hover:bg-indigo-50 transition-colors group text-left"
            >
              <div class="flex items-center">
                <!-- Avatar -->
                <div class="relative mr-4">
                  <img 
                    v-if="member.avatar_url"
                    :src="member.avatar_url"
                    class="w-10 h-10 rounded-full object-cover ring-2 ring-white shadow-sm"
                  />
                  <div v-else class="w-10 h-10 rounded-full bg-indigo-100 flex items-center justify-center ring-2 ring-white shadow-sm">
                    <User class="w-6 h-6 text-indigo-500" />
                  </div>
                </div>

                <!-- Name -->
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-semibold text-gray-900 truncate">
                    {{ member.display_name || member.username }}
                  </p>
                  <p class="text-xs text-gray-500 truncate">
                    @{{ member.username }}
                  </p>
                </div>
              </div>

              <!-- Add button -->
              <div v-if="addingMembers.has(member.user_id)" class="w-8 h-8 flex items-center justify-center">
                <div class="w-5 h-5 border-2 border-indigo-500/20 border-t-indigo-500 rounded-full animate-spin"></div>
              </div>
              <div v-else class="w-8 h-8 flex items-center justify-center rounded-full bg-indigo-100 text-indigo-600 opacity-0 group-hover:opacity-100 transition-opacity">
                <UserPlus class="w-4 h-4" />
              </div>
            </button>
          </div>

          <!-- Empty State -->
          <div v-else class="flex flex-col items-center justify-center py-12 text-gray-500 px-6 text-center">
            <div class="w-12 h-12 rounded-full bg-gray-100 flex items-center justify-center mb-4">
              <UserPlus class="w-6 h-6 text-gray-400" />
            </div>
            <p class="text-sm font-medium">No members to add</p>
            <p class="text-xs mt-1">All team members are already in this channel</p>
          </div>
        </div>

        <!-- Footer -->
        <div class="px-6 py-4 border-t border-gray-200 bg-gray-50 flex justify-end">
          <BaseButton variant="secondary" @click="handleClose">
            Done
          </BaseButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 4px;
}
.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: #475569;
}
</style>
