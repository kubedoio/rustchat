<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { X, Hash, Lock, Settings, Users, Trash2, Search, Plus, UserMinus } from 'lucide-vue-next'
import BaseButton from '../atomic/BaseButton.vue'
import BaseInput from '../atomic/BaseInput.vue'
import { channelsApi, type Channel } from '../../api/channels'
import { useChannelStore } from '../../stores/channels'
import { useTeamStore } from '../../stores/teams'
import { useAuthStore } from '../../stores/auth'
import { useToast } from '../../composables/useToast'

const props = defineProps<{
  isOpen: boolean
  channel: Channel | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated', channel: Channel): void
  (e: 'deleted'): void
}>()

const channelStore = useChannelStore()
const teamStore = useTeamStore()
const authStore = useAuthStore()
const toast = useToast()

const activeTab = ref('general')
const loading = ref(false)
const deleting = ref(false)

// Form fields
const displayName = ref('')
const purpose = ref('')
const header = ref('')

// Members Tab
const channelMembers = ref<any[]>([])
const searchQuery = ref('')
const addingMember = ref<string | null>(null)
const removingMember = ref<string | null>(null)
const membersLoading = ref(false)

const tabs = [
  { id: 'general', label: 'General', icon: Settings },
  { id: 'members', label: 'Members', icon: Users },
]

watch(() => props.isOpen, (isOpen) => {
  if (isOpen && props.channel) {
    displayName.value = props.channel.display_name || ''
    purpose.value = props.channel.purpose || ''
    header.value = props.channel.header || ''
    activeTab.value = 'general'
    searchQuery.value = ''
    channelMembers.value = []
  }
})

watch(activeTab, (tab) => {
  if (tab === 'members' && props.channel) {
    fetchMembers()
  }
})

async function fetchMembers() {
  if (!props.channel) return
  membersLoading.value = true
  try {
    const response = await channelsApi.getMembers(props.channel.id)
    channelMembers.value = response.data
  } catch (e) {
    console.error('Failed to fetch channel members', e)
  } finally {
    membersLoading.value = false
  }
}

// Search results: Users in the team who are NOT in the channel
const searchResults = computed(() => {
  if (!searchQuery.value.trim()) return []

  const query = searchQuery.value.toLowerCase()
  const currentMemberIds = new Set(channelMembers.value.map(m => m.user_id))

  return teamStore.members.filter(member => {
    // Exclude existing members
    if (currentMemberIds.has(member.user_id)) return false

    // Match name or username
    const name = (member.display_name || '').toLowerCase()
    const username = member.username.toLowerCase()

    return name.includes(query) || username.includes(query)
  }).slice(0, 5) // Limit to 5 results
})

async function addMember(userId: string) {
  if (!props.channel) return

  addingMember.value = userId
  try {
    await channelsApi.join(props.channel.id, userId)
    await fetchMembers()
    toast.success('Member added', 'User added to channel')
    searchQuery.value = '' // Clear search
  } catch (e: any) {
    toast.error('Failed to add member', e.response?.data?.message)
  } finally {
    addingMember.value = null
  }
}

async function removeMember(userId: string) {
  if (!props.channel) return
  if (!confirm('Are you sure you want to remove this member?')) return

  removingMember.value = userId
  try {
    await channelsApi.removeMember(props.channel.id, userId)
    await fetchMembers()
    toast.success('Member removed', 'User removed from channel')
  } catch (e: any) {
    toast.error('Failed to remove member', e.response?.data?.message)
  } finally {
    removingMember.value = null
  }
}

async function handleSave() {
  if (!props.channel) return
  
  loading.value = true
  try {
    const response = await channelsApi.update(props.channel.id, {
      display_name: displayName.value.trim() || undefined,
      purpose: purpose.value.trim() || undefined,
      header: header.value.trim() || undefined,
    })
    
    // Update local store
    channelStore.updateChannel(response.data)
    
    toast.success('Channel updated', 'Settings saved successfully')
    emit('updated', response.data)
    emit('close')
  } catch (e: any) {
    toast.error('Update failed', e.response?.data?.message || 'Could not update channel')
  } finally {
    loading.value = false
  }
}

async function handleDelete() {
  if (!props.channel) return
  if (!confirm(`Are you sure you want to delete #${props.channel.name}? This cannot be undone.`)) return
  
  deleting.value = true
  try {
    await channelsApi.delete(props.channel.id)
    toast.success('Channel deleted', `#${props.channel.name} has been removed`)
    emit('deleted')
    emit('close')
  } catch (e: any) {
    toast.error('Delete failed', e.response?.data?.message || 'Could not delete channel')
  } finally {
    deleting.value = false
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="isOpen && channel" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="fixed inset-0 bg-black/50" @click="$emit('close')"></div>
      
      <!-- Modal -->
      <div class="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700 shrink-0">
          <div class="flex items-center space-x-2">
            <component :is="channel.channel_type === 'private' ? Lock : Hash" class="w-5 h-5 text-gray-500" />
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white">{{ channel.name }}</h2>
          </div>
          <button @click="$emit('close')" class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded">
            <X class="w-5 h-5 text-gray-400" />
          </button>
        </div>
        
        <!-- Tabs -->
        <div class="flex border-b border-gray-200 dark:border-gray-700 px-6 shrink-0">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            @click="activeTab = tab.id"
            class="flex items-center px-4 py-3 text-sm font-medium border-b-2 -mb-px transition-colors"
            :class="activeTab === tab.id 
              ? 'border-primary text-primary' 
              : 'border-transparent text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'"
          >
            <component :is="tab.icon" class="w-4 h-4 mr-2" />
            {{ tab.label }}
          </button>
        </div>
        
        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-6">
          <!-- General Tab -->
          <div v-if="activeTab === 'general'" class="space-y-5">
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Channel Name</label>
              <div class="px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg text-gray-600 dark:text-gray-400 text-sm">
                #{{ channel.name }}
              </div>
              <p class="mt-1 text-xs text-gray-500">Channel names cannot be changed after creation.</p>
            </div>
            
            <BaseInput 
              label="Display Name" 
              v-model="displayName" 
              placeholder="Optional display name"
              :disabled="loading"
            />
            
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Purpose</label>
              <textarea
                v-model="purpose"
                rows="2"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white resize-none focus:ring-2 focus:ring-primary focus:border-transparent text-sm"
                placeholder="What is this channel about?"
                :disabled="loading"
              ></textarea>
            </div>
            
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Header</label>
              <textarea
                v-model="header"
                rows="2"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white resize-none focus:ring-2 focus:ring-primary focus:border-transparent text-sm"
                placeholder="Channel header (shown at the top)"
                :disabled="loading"
              ></textarea>
            </div>
            
            <!-- Danger Zone -->
            <div class="pt-6 border-t border-gray-200 dark:border-gray-700">
              <h4 class="text-sm font-semibold text-red-600 dark:text-red-400 mb-3">Danger Zone</h4>
              <button
                @click="handleDelete"
                :disabled="deleting"
                class="flex items-center px-4 py-2 text-sm font-medium text-red-600 dark:text-red-400 border border-red-300 dark:border-red-800 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors disabled:opacity-50"
              >
                <Trash2 class="w-4 h-4 mr-2" />
                {{ deleting ? 'Deleting...' : 'Delete Channel' }}
              </button>
            </div>
          </div>
          
          <!-- Members Tab -->
          <div v-else-if="activeTab === 'members'" class="space-y-6">
            <!-- Add Member -->
            <div class="space-y-3">
              <h4 class="text-sm font-medium text-gray-900 dark:text-white">Add Member</h4>
              <div class="relative">
                <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                  <Search class="h-4 w-4 text-gray-400" />
                </div>
                <input
                  type="text"
                  v-model="searchQuery"
                  placeholder="Search team members to add"
                  class="block w-full pl-10 pr-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg leading-5 bg-white dark:bg-gray-700 placeholder-gray-500 focus:outline-none focus:placeholder-gray-400 focus:ring-1 focus:ring-primary focus:border-primary sm:text-sm transition duration-150 ease-in-out dark:text-white"
                />
              </div>

              <!-- Search Results -->
              <div v-if="searchQuery && searchResults.length > 0" class="bg-white dark:bg-gray-700 rounded-lg border border-gray-200 dark:border-gray-600 divide-y divide-gray-200 dark:divide-gray-600 max-h-48 overflow-y-auto">
                <div v-for="user in searchResults" :key="user.user_id" class="flex items-center justify-between p-3 hover:bg-gray-50 dark:hover:bg-gray-600/50 transition-colors">
                  <div class="flex items-center space-x-3">
                    <div class="w-8 h-8 rounded-full bg-indigo-100 dark:bg-indigo-900/50 flex items-center justify-center text-indigo-600 dark:text-indigo-400 font-medium text-sm">
                      {{ (user.display_name || user.username).charAt(0).toUpperCase() }}
                    </div>
                    <div>
                      <p class="text-sm font-medium text-gray-900 dark:text-white">{{ user.display_name || user.username }}</p>
                      <p class="text-xs text-gray-500">@{{ user.username }}</p>
                    </div>
                  </div>
                  <button
                    @click="addMember(user.user_id)"
                    :disabled="addingMember === user.user_id"
                    class="p-1.5 bg-indigo-50 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400 rounded-lg hover:bg-indigo-100 dark:hover:bg-indigo-900/50 transition-colors disabled:opacity-50"
                  >
                    <Plus class="w-4 h-4" />
                  </button>
                </div>
              </div>
              <div v-else-if="searchQuery && searchResults.length === 0" class="text-center py-4 text-sm text-gray-500">
                No matching team members found
              </div>
            </div>

            <!-- Member List -->
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <h4 class="text-sm font-medium text-gray-900 dark:text-white">Channel Members</h4>
                <span class="text-xs text-gray-500">{{ channelMembers.length }} members</span>
              </div>

              <div class="bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700 max-h-64 overflow-y-auto">
                <div v-if="membersLoading" class="p-4 text-center text-gray-500 text-sm">
                  Loading members...
                </div>
                <template v-else>
                  <div v-for="member in channelMembers" :key="member.user_id" class="flex items-center justify-between p-3">
                    <div class="flex items-center space-x-3">
                      <div class="w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center text-gray-600 dark:text-gray-300 font-medium text-sm">
                        {{ (member.display_name || member.username).charAt(0).toUpperCase() }}
                      </div>
                      <div>
                        <div class="flex items-center space-x-2">
                          <p class="text-sm font-medium text-gray-900 dark:text-white">{{ member.display_name || member.username }}</p>
                          <span v-if="member.role === 'admin' || member.role === 'owner'" class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-400 border border-amber-200 dark:border-amber-800">
                            {{ member.role }}
                          </span>
                        </div>
                        <p class="text-xs text-gray-500">@{{ member.username }}</p>
                      </div>
                    </div>

                    <div v-if="member.user_id !== authStore.user?.id" class="flex items-center">
                      <button
                        @click="removeMember(member.user_id)"
                        :disabled="removingMember === member.user_id"
                        class="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors disabled:opacity-50"
                        title="Remove member"
                      >
                        <UserMinus class="w-4 h-4" />
                      </button>
                    </div>
                  </div>

                  <div v-if="channelMembers.length === 0" class="p-8 text-center text-gray-500 text-sm">
                    No members found
                  </div>
                </template>
              </div>
            </div>
          </div>
        </div>
        
        <!-- Footer -->
        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end space-x-3 shrink-0">
          <BaseButton variant="secondary" @click="$emit('close')">Cancel</BaseButton>
          <BaseButton @click="handleSave" :loading="loading">Save Changes</BaseButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>
