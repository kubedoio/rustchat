<script setup lang="ts">
import { ref, watch } from 'vue'
import { X, Hash, Lock, Settings, Users, Trash2 } from 'lucide-vue-next'
import BaseButton from '../atomic/BaseButton.vue'
import BaseInput from '../atomic/BaseInput.vue'
import { channelsApi, type Channel } from '../../api/channels'
import { useChannelStore } from '../../stores/channels'
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
const toast = useToast()

const activeTab = ref('general')
const loading = ref(false)
const deleting = ref(false)

// Form fields
const displayName = ref('')
const purpose = ref('')
const header = ref('')

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
  }
})

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
          <div v-else-if="activeTab === 'members'" class="text-center py-10 text-gray-500">
            <Users class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>Member management coming soon</p>
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
