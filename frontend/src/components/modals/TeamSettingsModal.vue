<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { X, Settings, Users, Shield, Trash2, Camera, Copy, Check, Search, Plus, UserMinus, LogOut } from 'lucide-vue-next'
import BaseButton from '../atomic/BaseButton.vue'
import BaseInput from '../atomic/BaseInput.vue'
import { teamsApi, type Team } from '../../api/teams'
import { usersApi, type User } from '../../api/users'
import { useTeamStore } from '../../stores/teams'
import { useToast } from '../../composables/useToast'
import { useAuthStore } from '../../stores/auth'
import { useCurrentTeamManagementPermission } from '../../features/permissions/capabilities'

const props = defineProps<{
  isOpen: boolean
  team: Team | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated', team: Team): void
  (e: 'deleted'): void
}>()

const teamStore = useTeamStore()
const authStore = useAuthStore()
const toast = useToast()
const { canManageTeam: canManageCurrentTeam } = useCurrentTeamManagementPermission(
  () => props.team?.id ?? null,
)

const activeTab = ref('general')
const loading = ref(false)
const leaving = ref(false)
const deleting = ref(false)
const copied = ref(false)

// Form fields
const displayName = ref('')
const description = ref('')
const isPublic = ref(false)
const allowOpenInvite = ref(false)

// Members Tab
const searchQuery = ref('')
const searchResults = ref<User[]>([])
const searching = ref(false)
const addingMember = ref<string | null>(null)
const removingMember = ref<string | null>(null)

const tabs = [
  { id: 'general', label: 'General', icon: Settings },
  { id: 'members', label: 'Members', icon: Users },
  { id: 'permissions', label: 'Permissions', icon: Shield },
]

watch(() => props.isOpen, (isOpen) => {
  if (isOpen && props.team) {
    displayName.value = props.team.display_name || ''
    description.value = props.team.description || ''
    isPublic.value = props.team.is_public || false
    allowOpenInvite.value = props.team.allow_open_invite || false
    activeTab.value = 'general'
    // Reset search
    searchQuery.value = ''
    searchResults.value = []
  }
})

watch(activeTab, (tab) => {
  if (tab === 'members' && props.team && canManageCurrentTeam.value) {
    teamStore.fetchMembers(props.team.id)
  }
})

const inviteLink = computed(() => {
  if (!props.team) return ''
  return `${window.location.origin}/join/${props.team.invite_id || props.team.id}`
})

async function handleSave() {
  if (!props.team || !canManageCurrentTeam.value) return
  
  loading.value = true
  try {
    const response = await teamsApi.update(props.team.id, {
      display_name: displayName.value.trim() || undefined,
      description: description.value.trim() || undefined,
      is_public: isPublic.value,
      allow_open_invite: allowOpenInvite.value,
    })
    
    teamStore.updateTeam(response.data)
    toast.success('Team updated', 'Settings saved successfully')
    emit('updated', response.data)
    emit('close')
  } catch (e: any) {
    toast.error('Update failed', e.response?.data?.message || 'Could not update team')
  } finally {
    loading.value = false
  }
}

async function handleDelete() {
  if (!props.team || !canManageCurrentTeam.value) return
  if (!confirm(`Are you sure you want to delete "${props.team.display_name || props.team.name}"? This will delete all channels and messages. This cannot be undone.`)) return
  
  deleting.value = true
  try {
    await teamsApi.delete(props.team.id)
    toast.success('Team deleted', `${props.team.display_name || props.team.name} has been removed`)
    emit('deleted')
    emit('close')
  } catch (e: any) {
    toast.error('Delete failed', e.response?.data?.message || 'Could not delete team')
  } finally {
    deleting.value = false
  }
}

async function handleLeave() {
  if (!props.team) return
  if (!confirm(`Are you sure you want to leave "${props.team.display_name || props.team.name}"?`)) return
  
  leaving.value = true
  try {
    await teamStore.leaveTeam(props.team.id)
    toast.success('Left team', `You have left ${props.team.display_name || props.team.name}`)
    emit('close')
  } catch (e: any) {
    toast.error('Failed to leave', e.response?.data?.message || 'Could not leave team')
  } finally {
    leaving.value = false
  }
}

function copyInviteLink() {
  navigator.clipboard.writeText(inviteLink.value)
  copied.value = true
  toast.success('Copied!', 'Invite link copied to clipboard')
  setTimeout(() => { copied.value = false }, 2000)
}

async function handleSearch() {
  if (!canManageCurrentTeam.value) {
    searchResults.value = []
    return
  }

  if (!searchQuery.value.trim()) {
    searchResults.value = []
    return
  }
  
  searching.value = true
  try {
    const response = await usersApi.list({ q: searchQuery.value, per_page: 5 })
    // Filter out existing members
    const memberIds = new Set(teamStore.members.map(m => m.user_id))
    searchResults.value = response.data.filter(u => !memberIds.has(u.id))
  } catch (e) {
    console.error('Search failed', e)
  } finally {
    searching.value = false
  }
}

// Debounce search
let searchTimeout: ReturnType<typeof setTimeout>
function onSearchInput() {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(handleSearch, 300)
}

async function addMember(user: User) {
  if (!props.team || !canManageCurrentTeam.value) return
  
  addingMember.value = user.id
  try {
    await teamsApi.addMember(props.team.id, user.id)
    await teamStore.fetchMembers(props.team.id)
    // Remove from search results
    searchResults.value = searchResults.value.filter(u => u.id !== user.id)
    toast.success('Member added', `${user.display_name || user.username} added to the team`)
  } catch (e: any) {
    toast.error('Failed to add member', e.response?.data?.message)
  } finally {
    addingMember.value = null
  }
}

async function removeMember(userId: string) {
  if (!props.team || !canManageCurrentTeam.value) return
  if (!confirm('Are you sure you want to remove this member?')) return
  
  removingMember.value = userId
  try {
    // Assuming delete method exists or using generic api call if strictly typed client is missing it
    // Using explicit fetch for now if missing in teamsApi
    await teamsApi.removeMember(props.team.id, userId)
    await teamStore.fetchMembers(props.team.id)
    toast.success('Member removed', 'User removed from the team')
  } catch (e: any) {
    toast.error('Failed to remove member', e.response?.data?.message)
  } finally {
    removingMember.value = null
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="isOpen && team" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="fixed inset-0 bg-black/50" @click="$emit('close')"></div>
      
      <!-- Modal -->
      <div class="relative bg-bg-surface-1 rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-border-1 shrink-0">
          <div class="flex items-center space-x-3">
            <div class="flex h-10 w-10 items-center justify-center rounded-lg bg-primary text-lg font-bold text-brand-foreground">
              {{ (team.display_name || team.name).charAt(0).toUpperCase() }}
            </div>
            <div>
              <h2 class="text-lg font-semibold text-text-1">{{ team.display_name || team.name }}</h2>
              <p class="text-sm text-text-3">Team Settings</p>
            </div>
          </div>
          <button @click="$emit('close')" class="p-1 hover:bg-bg-surface-2 rounded">
            <X class="w-5 h-5 text-text-4" />
          </button>
        </div>
        
        <!-- Tabs -->
        <div
          v-if="canManageCurrentTeam"
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
            v-if="!canManageCurrentTeam"
            class="rounded-xl border bg-warning/10 p-4 text-sm text-warning border border-warning/20"
          >
            You do not have permission to manage this team.
          </div>

          <!-- General Tab -->
          <div v-else-if="activeTab === 'general'" class="space-y-5">
            <!-- Team Icon -->
            <div class="flex items-center space-x-4">
              <div class="relative">
                <div class="flex h-16 w-16 items-center justify-center rounded-lg bg-primary text-2xl font-bold text-brand-foreground">
                  {{ (team.display_name || team.name).charAt(0).toUpperCase() }}
                </div>
                <button 
                  type="button"
                  class="absolute -bottom-1 -right-1 w-6 h-6 bg-bg-surface-2 rounded-full flex items-center justify-center border-2 border-bg-surface-1"
                >
                  <Camera class="w-3 h-3 text-white" />
                </button>
              </div>
              <div>
                <p class="text-sm font-medium text-text-1">{{ team.name }}</p>
                <p class="text-xs text-text-3">Team identifier cannot be changed</p>
              </div>
            </div>
            
            <BaseInput 
              label="Display Name" 
              v-model="displayName" 
              placeholder="My Team"
              :disabled="loading"
            />
            
            <div>
              <label class="block text-sm font-medium text-text-2 mb-1">Description</label>
              <textarea
                v-model="description"
                rows="3"
                class="w-full px-3 py-2 border border-border-2 rounded-lg bg-bg-surface-1 text-text-1 resize-none focus:ring-2 focus:ring-primary focus:border-transparent text-sm"
                placeholder="What is this team about?"
                :disabled="loading"
              ></textarea>
            </div>

            <!-- Visibility Settings -->
            <div class="space-y-3 pt-2">
                <div class="flex items-center justify-between">
                    <div>
                        <h4 class="text-sm font-medium text-text-1">Team Visibility</h4>
                        <p class="text-xs text-text-3">Public teams can be discovered by anyone in the organization.</p>
                    </div>
                    <label class="relative inline-flex items-center cursor-pointer">
                        <input type="checkbox" v-model="isPublic" class="sr-only peer">
                        <div class="w-11 h-6 bg-bg-surface-2 peer-focus:outline-none ring-offset-2 peer-focus:ring-2 peer-focus:ring-brand rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-border-2 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-brand"></div>
                        <span class="ml-3 text-sm font-medium text-text-2">{{ isPublic ? 'Public' : 'Private' }}</span>
                    </label>
                </div>

                <div class="flex items-center justify-between">
                    <div>
                        <h4 class="text-sm font-medium text-text-1">Allow Open Invite</h4>
                        <p class="text-xs text-text-3">Allow users to join via invite link without approval.</p>
                    </div>
                    <label class="relative inline-flex items-center cursor-pointer">
                        <input type="checkbox" v-model="allowOpenInvite" class="sr-only peer">
                        <div class="w-11 h-6 bg-bg-surface-2 peer-focus:outline-none ring-offset-2 peer-focus:ring-2 peer-focus:ring-brand rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-border-2 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-brand"></div>
                    </label>
                </div>
            </div>
            
            <!-- Invite Link -->
            <div class="p-4 bg-bg-surface-2 rounded-lg">
              <label class="block text-sm font-medium text-text-2 mb-2">Invite Link</label>
              <div class="flex items-center space-x-2">
                <input
                  type="text"
                  :value="inviteLink"
                  readonly
                  class="flex-1 px-3 py-2 bg-bg-surface-1 border border-border-2 rounded-lg text-sm text-text-3"
                />
                <button
                  @click="copyInviteLink"
                  class="flex items-center space-x-1 rounded-lg bg-primary px-3 py-2 text-brand-foreground transition-colors hover:bg-brand-hover"
                >
                  <component :is="copied ? Check : Copy" class="w-4 h-4" />
                  <span class="text-sm">{{ copied ? 'Copied' : 'Copy' }}</span>
                </button>
              </div>
              <p class="mt-2 text-xs text-text-3">Share this link to invite people to your team</p>
            </div>
            
            <!-- Danger Zone -->
            <div class="pt-6 border-t border-border-1">
              <h4 class="text-sm font-semibold text-danger mb-3">Danger Zone</h4>
              <div class="space-y-3">
                <button
                  @click="handleLeave"
                  :disabled="leaving || deleting"
                  class="flex items-center px-4 py-2 text-sm font-medium text-danger border border-danger/30 rounded-lg hover:bg-danger/10 transition-colors disabled:opacity-50"
                >
                  <LogOut class="w-4 h-4 mr-2" />
                  {{ leaving ? 'Leaving...' : 'Leave Team' }}
                </button>

                <button
                  v-if="canManageCurrentTeam"
                  @click="handleDelete"
                  :disabled="deleting || leaving"
                  class="flex items-center px-4 py-2 text-sm font-medium text-danger border border-red-300 rounded-lg hover:bg-red-50 transition-colors disabled:opacity-50"
                >
                  <Trash2 class="w-4 h-4 mr-2" />
                  {{ deleting ? 'Deleting...' : 'Delete Team' }}
                </button>
              </div>
              <p class="mt-2 text-xs text-text-3">Leaving a team will remove your access to its channels. Deleting a team is permanent.</p>
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
                  @input="onSearchInput"
                  placeholder="Search users by name or username"
                  class="block w-full pl-10 pr-3 py-2 border border-border-2 rounded-lg leading-5 bg-bg-surface-1 placeholder-text-4 focus:outline-none focus:placeholder-text-4 focus:ring-1 focus:ring-primary focus:border-primary sm:text-sm transition duration-150 ease-in-out"
                />
                <div v-if="searching" class="absolute inset-y-0 right-0 pr-3 flex items-center pointer-events-none">
                  <div class="animate-spin h-4 w-4 border-2 border-text-4 border-t-transparent rounded-full"></div>
                </div>
              </div>

              <!-- Search Results -->
              <div v-if="searchResults.length > 0" class="bg-bg-surface-1 rounded-lg border border-border-1 divide-y divide-border-1 max-h-48 overflow-y-auto">
                <div v-for="user in searchResults" :key="user.id" class="flex items-center justify-between p-3 hover:bg-bg-surface-2 transition-colors">
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
                    @click="addMember(user)"
                    :disabled="addingMember === user.id"
                    class="p-1.5 bg-brand/10 text-brand rounded-lg hover:bg-brand/20 transition-colors disabled:opacity-50"
                  >
                    <Plus class="w-4 h-4" />
                  </button>
                </div>
              </div>
              <div v-else-if="searchQuery && !searching" class="text-center py-4 text-sm text-text-3">
                No users found
              </div>
            </div>

            <!-- Member List -->
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <h4 class="text-sm font-medium text-text-1">Team Members</h4>
                <span class="text-xs text-text-3">{{ teamStore.members.length }} members</span>
              </div>
              
              <div class="bg-bg-surface-2 rounded-lg border border-border-1 divide-y divide-border-1">
                <div v-for="member in teamStore.members" :key="member.user_id" class="flex items-center justify-between p-3">
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
                
                <div v-if="teamStore.members.length === 0" class="p-8 text-center text-text-3 text-sm">
                  No members found
                </div>
              </div>
            </div>
          </div>
          
          <!-- Permissions Tab -->
          <div v-else-if="activeTab === 'permissions'" class="text-center py-10 text-text-3">
            <Shield class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>Permission settings coming soon</p>
            <p class="text-sm mt-1">Configure roles and access control</p>
          </div>
        </div>
        
        <!-- Footer -->
        <div class="px-6 py-4 border-t border-border-1 flex justify-end space-x-3 shrink-0">
          <BaseButton variant="secondary" @click="$emit('close')">Cancel</BaseButton>
          <BaseButton v-if="canManageCurrentTeam" @click="handleSave" :loading="loading">Save Changes</BaseButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>
