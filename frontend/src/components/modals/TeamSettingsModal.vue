<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { X, Settings, Users, Shield, Trash2, Camera, Copy, Check } from 'lucide-vue-next'
import BaseButton from '../atomic/BaseButton.vue'
import BaseInput from '../atomic/BaseInput.vue'
import { teamsApi, type Team } from '../../api/teams'
import { useTeamStore } from '../../stores/teams'
import { useToast } from '../../composables/useToast'

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
const toast = useToast()

const activeTab = ref('general')
const loading = ref(false)
const deleting = ref(false)
const copied = ref(false)

// Form fields
const displayName = ref('')
const description = ref('')
const isPublic = ref(false)
const allowOpenInvite = ref(false)

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
  }
})

const inviteLink = computed(() => {
  if (!props.team) return ''
  return `${window.location.origin}/join/${props.team.invite_id || props.team.id}`
})

async function handleSave() {
  if (!props.team) return
  
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
  if (!props.team) return
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

function copyInviteLink() {
  navigator.clipboard.writeText(inviteLink.value)
  copied.value = true
  toast.success('Copied!', 'Invite link copied to clipboard')
  setTimeout(() => { copied.value = false }, 2000)
}
</script>

<template>
  <Teleport to="body">
    <div v-if="isOpen && team" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="fixed inset-0 bg-black/50" @click="$emit('close')"></div>
      
      <!-- Modal -->
      <div class="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700 shrink-0">
          <div class="flex items-center space-x-3">
            <div class="w-10 h-10 rounded-lg bg-primary flex items-center justify-center text-white font-bold text-lg">
              {{ (team.display_name || team.name).charAt(0).toUpperCase() }}
            </div>
            <div>
              <h2 class="text-lg font-semibold text-gray-900 dark:text-white">{{ team.display_name || team.name }}</h2>
              <p class="text-sm text-gray-500">Team Settings</p>
            </div>
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
            <!-- Team Icon -->
            <div class="flex items-center space-x-4">
              <div class="relative">
                <div class="w-16 h-16 rounded-lg bg-primary flex items-center justify-center text-white font-bold text-2xl">
                  {{ (team.display_name || team.name).charAt(0).toUpperCase() }}
                </div>
                <button 
                  type="button"
                  class="absolute -bottom-1 -right-1 w-6 h-6 bg-gray-800 dark:bg-gray-600 rounded-full flex items-center justify-center border-2 border-white dark:border-gray-800"
                >
                  <Camera class="w-3 h-3 text-white" />
                </button>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-900 dark:text-white">{{ team.name }}</p>
                <p class="text-xs text-gray-500">Team identifier cannot be changed</p>
              </div>
            </div>
            
            <BaseInput 
              label="Display Name" 
              v-model="displayName" 
              placeholder="My Team"
              :disabled="loading"
            />
            
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description</label>
              <textarea
                v-model="description"
                rows="3"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white resize-none focus:ring-2 focus:ring-primary focus:border-transparent text-sm"
                placeholder="What is this team about?"
                :disabled="loading"
              ></textarea>
            </div>

            <!-- Visibility Settings -->
            <div class="space-y-3 pt-2">
                <div class="flex items-center justify-between">
                    <div>
                        <h4 class="text-sm font-medium text-gray-900 dark:text-white">Team Visibility</h4>
                        <p class="text-xs text-gray-500">Public teams can be discovered by anyone in the organization.</p>
                    </div>
                    <label class="relative inline-flex items-center cursor-pointer">
                        <input type="checkbox" v-model="isPublic" class="sr-only peer">
                        <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none ring-offset-2 peer-focus:ring-2 peer-focus:ring-indigo-500 dark:peer-focus:ring-indigo-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-indigo-600"></div>
                        <span class="ml-3 text-sm font-medium text-gray-900 dark:text-gray-300">{{ isPublic ? 'Public' : 'Private' }}</span>
                    </label>
                </div>

                <div class="flex items-center justify-between">
                    <div>
                        <h4 class="text-sm font-medium text-gray-900 dark:text-white">Allow Open Invite</h4>
                        <p class="text-xs text-gray-500">Allow users to join via invite link without approval.</p>
                    </div>
                    <label class="relative inline-flex items-center cursor-pointer">
                        <input type="checkbox" v-model="allowOpenInvite" class="sr-only peer">
                        <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none ring-offset-2 peer-focus:ring-2 peer-focus:ring-indigo-500 dark:peer-focus:ring-indigo-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-indigo-600"></div>
                    </label>
                </div>
            </div>
            
            <!-- Invite Link -->
            <div class="p-4 bg-gray-50 dark:bg-gray-900 rounded-lg">
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Invite Link</label>
              <div class="flex items-center space-x-2">
                <input
                  type="text"
                  :value="inviteLink"
                  readonly
                  class="flex-1 px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-600 dark:text-gray-400"
                />
                <button
                  @click="copyInviteLink"
                  class="px-3 py-2 bg-primary text-white rounded-lg hover:bg-blue-600 transition-colors flex items-center space-x-1"
                >
                  <component :is="copied ? Check : Copy" class="w-4 h-4" />
                  <span class="text-sm">{{ copied ? 'Copied' : 'Copy' }}</span>
                </button>
              </div>
              <p class="mt-2 text-xs text-gray-500">Share this link to invite people to your team</p>
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
                {{ deleting ? 'Deleting...' : 'Delete Team' }}
              </button>
              <p class="mt-2 text-xs text-gray-500">This will permanently delete all channels, messages, and files in this team.</p>
            </div>
          </div>
          
          <!-- Members Tab -->
          <div v-else-if="activeTab === 'members'" class="text-center py-10 text-gray-500">
            <Users class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>Member management coming soon</p>
            <p class="text-sm mt-1">Add, remove, and manage team members</p>
          </div>
          
          <!-- Permissions Tab -->
          <div v-else-if="activeTab === 'permissions'" class="text-center py-10 text-gray-500">
            <Shield class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>Permission settings coming soon</p>
            <p class="text-sm mt-1">Configure roles and access control</p>
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
