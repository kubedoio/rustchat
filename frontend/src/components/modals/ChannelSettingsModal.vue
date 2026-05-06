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
import { useChannelManagementPermission } from '../../features/permissions/capabilities'

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
const { canManageChannel: canManageCurrentChannel } = useChannelManagementPermission(
  () => props.channel?.id ?? null,
  () => props.channel?.creator_id ?? null,
)

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
  if (tab === 'members' && props.channel && canManageCurrentChannel.value) {
    fetchMembers()
  }
})

async function fetchMembers() {
  if (!props.channel || !canManageCurrentChannel.value) return
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
  if (!props.channel || !canManageCurrentChannel.value) return
  
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
  if (!props.channel || !canManageCurrentChannel.value) return
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
  if (!props.channel || !canManageCurrentChannel.value) return
  
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
  if (!props.channel || !canManageCurrentChannel.value) return
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
      <div class="relative bg-bg-surface-1 rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-border-1 shrink-0">
          <div class="flex items-center space-x-2">
            <component :is="channel.channel_type === 'private' ? Lock : Hash" class="w-5 h-5 text-text-3" />
            <h2 class="text-lg font-semibold text-text-1">{{ channel.name }}</h2>
          </div>
          <button @click="$emit('close')" class="p-1 hover:bg-bg-surface-2 rounded">
            <X class="w-5 h-5 text-text-4" />
          </button>
        </div>
        
        <!-- Tabs -->
        <div
          v-if="canManageCurrentChannel"
          class="flex border-b border-border-1 px-6 shrink-0"
        >
          <button
            v-for="tab in tabs"
            :key="tab.id"
            @click="activeTab = tab.id"
            class="flex items-center px-4 py-3 text-sm font-medium border-b-2 -mb-px transition-colors"
            :class="activeTab === tab.id 
              ? 'border-primary text-primary' 
              : 'border-transparent text-text-3 hover:text-text-2'"
          >
            <component :is="tab.icon" class="w-4 h-4 mr-2" />
            {{ tab.label }}
          </button>
        </div>
        
        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-6">
          <div
            v-if="!canManageCurrentChannel"
            class="rounded-xl border bg-warning/10 p-4 text-sm text-warning border border-warning/20"
          >
            You do not have permission to manage this channel.
          </div>

          <!-- General Tab -->
          <div v-else-if="activeTab === 'general'" class="space-y-5">
            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Channel Name</label>
              <div class="px-3 py-2 bg-bg-surface-2 rounded-lg text-text-3 text-sm">
                #{{ channel.name }}
              </div>
              <p class="mt-1 text-xs text-text-3">Channel names cannot be changed after creation.</p>
            </div>
            
            <BaseInput 
              label="Display Name" 
              v-model="displayName" 
              placeholder="Optional display name"
              :disabled="loading"
            />
            
            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Purpose</label>
              <textarea
                v-model="purpose"
                rows="2"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 resize-none focus:ring-2 focus:ring-primary focus:border-transparent text-sm"
                placeholder="What is this channel about?"
                :disabled="loading"
              ></textarea>
            </div>
            
            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Header</label>
              <textarea
                v-model="header"
                rows="2"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 resize-none focus:ring-2 focus:ring-primary focus:border-transparent text-sm"
                placeholder="Channel header (shown at the top)"
                :disabled="loading"
              ></textarea>
            </div>
            
            <!-- Danger Zone -->
            <div class="pt-6 border-t border-border-1">
              <h4 class="text-sm font-semibold text-danger mb-3">Danger Zone</h4>
              <button
                @click="handleDelete"
                :disabled="deleting"
                class="flex items-center px-4 py-2 text-sm font-medium text-danger border border-danger/30 rounded-lg hover:bg-danger/10 transition-colors disabled:opacity-50"
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
              <h4 class="text-sm font-medium text-text-1">Add Member</h4>
              <div class="relative">
                <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                  <Search class="h-4 w-4 text-text-4" />
                </div>
                <input
                  type="text"
                  v-model="searchQuery"
                  placeholder="Search team members to add"
                  class="block w-full pl-10 pr-3 py-2 border border-border-2 rounded-lg leading-5 bg-bg-surface-1 placeholder-text-4 focus:outline-none focus:placeholder-text-4 focus:ring-1 focus:ring-primary focus:border-primary sm:text-sm transition duration-150 ease-in-out"
                />
              </div>

              <!-- Search Results -->
              <div v-if="searchQuery && searchResults.length > 0" class="bg-bg-surface-1 rounded-lg border border-border-1 divide-y divide-border-1 max-h-48 overflow-y-auto">
                <div v-for="user in searchResults" :key="user.user_id" class="flex items-center justify-between p-3 hover:bg-bg-surface-2 transition-colors">
                  <div class="flex items-center space-x-3">
                    <div class="w-8 h-8 rounded-full bg-brand/10 flex items-center justify-center text-brand font-medium text-sm">
                      {{ (user.display_name || user.username).charAt(0).toUpperCase() }}
                    </div>
                    <div>
                      <p class="text-sm font-medium text-text-1">{{ user.display_name || user.username }}</p>
                      <p class="text-xs text-text-3">@{{ user.username }}</p>
                    </div>
                  </div>
                  <button
                    @click="addMember(user.user_id)"
                    :disabled="addingMember === user.user_id"
                    class="p-1.5 bg-brand/10 text-brand rounded-lg hover:bg-brand/20 transition-colors disabled:opacity-50"
                  >
                    <Plus class="w-4 h-4" />
                  </button>
                </div>
              </div>
              <div v-else-if="searchQuery && searchResults.length === 0" class="text-center py-4 text-sm text-text-3">
                No matching team members found
              </div>
            </div>

            <!-- Member List -->
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <h4 class="text-sm font-medium text-text-1">Channel Members</h4>
                <span class="text-xs text-text-3">{{ channelMembers.length }} members</span>
              </div>
              
              <div class="bg-bg-surface-2 rounded-lg border border-border-1 divide-y divide-border-1 max-h-64 overflow-y-auto">
                <div v-if="membersLoading" class="p-4 text-center text-text-3 text-sm">
                  Loading members...
                </div>
                <template v-else>
                  <div v-for="member in channelMembers" :key="member.user_id" class="flex items-center justify-between p-3">
                    <div class="flex items-center space-x-3">
                      <div class="w-8 h-8 rounded-full bg-bg-surface-2 flex items-center justify-center text-text-2 font-medium text-sm">
                        {{ (member.display_name || member.username).charAt(0).toUpperCase() }}
                      </div>
                      <div>
                        <div class="flex items-center space-x-2">
                          <p class="text-sm font-medium text-text-1">{{ member.display_name || member.username }}</p>
                          <span v-if="member.role === 'admin' || member.role === 'owner'" class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-warning/10 text-warning border border-warning/20">
                            {{ member.role }}
                          </span>
                        </div>
                        <p class="text-xs text-text-3">@{{ member.username }}</p>
                      </div>
                    </div>
                    
                    <div v-if="member.user_id !== authStore.user?.id" class="flex items-center">
                      <button
                        @click="removeMember(member.user_id)"
                        :disabled="removingMember === member.user_id"
                        class="p-1.5 text-text-4 hover:text-danger hover:bg-danger/10 rounded-lg transition-colors disabled:opacity-50"
                        title="Remove member"
                      >
                        <UserMinus class="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                  
                  <div v-if="channelMembers.length === 0" class="p-8 text-center text-text-3 text-sm">
                    No members found
                  </div>
                </template>
              </div>
            </div>
          </div>
        </div>
        
        <!-- Footer -->
        <div class="px-6 py-4 border-t border-border-1 flex justify-end space-x-3 shrink-0">
          <BaseButton variant="secondary" @click="$emit('close')">Cancel</BaseButton>
          <BaseButton v-if="canManageCurrentChannel" @click="handleSave" :loading="loading">Save Changes</BaseButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>
