<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Bell, Moon, Monitor } from 'lucide-vue-next'
import { usePreferencesStore } from '../../stores/preferences'
import { useToast } from '../../composables/useToast'
import BaseButton from '../atomic/BaseButton.vue'

const preferencesStore = usePreferencesStore()
const toast = useToast()

// Local form state
const notifyDesktop = ref('all')
const notifyPush = ref('all')
const notifySounds = ref(true)
const dndEnabled = ref(false)
const dndStartTime = ref('22:00')
const dndEndTime = ref('08:00')
const messageDisplay = ref('standard')
const timeFormat = ref('12h')

onMounted(async () => {
  await preferencesStore.fetchPreferences()
  if (preferencesStore.preferences) {
    const p = preferencesStore.preferences
    notifyDesktop.value = p.notify_desktop
    notifyPush.value = p.notify_push
    notifySounds.value = p.notify_sounds
    dndEnabled.value = p.dnd_enabled
    dndStartTime.value = p.dnd_start_time || '22:00'
    dndEndTime.value = p.dnd_end_time || '08:00'
    messageDisplay.value = p.message_display
    timeFormat.value = p.time_format
  }
})

async function handleSave() {
  try {
    await preferencesStore.updatePreferences({
      notify_desktop: notifyDesktop.value,
      notify_push: notifyPush.value,
      notify_sounds: notifySounds.value,
      dnd_enabled: dndEnabled.value,
      dnd_start_time: dndEnabled.value ? dndStartTime.value : undefined,
      dnd_end_time: dndEnabled.value ? dndEndTime.value : undefined,
      message_display: messageDisplay.value,
      time_format: timeFormat.value,
    })
    toast.success('Preferences saved', 'Your settings have been updated')
  } catch (e) {
    toast.error('Save failed', 'Could not save preferences')
  }
}

const notifyOptions = [
  { value: 'all', label: 'All messages' },
  { value: 'mentions', label: 'Mentions & DMs only' },
  { value: 'none', label: 'Nothing' },
]
</script>

<template>
  <div class="space-y-6">
    <!-- Notifications Section -->
    <div>
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white flex items-center mb-4">
        <Bell class="w-5 h-5 mr-2 text-primary" />
        Notifications
      </h3>
      
      <div class="space-y-4">
        <!-- Desktop Notifications -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Desktop notifications
          </label>
          <select
            v-model="notifyDesktop"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
          >
            <option v-for="opt in notifyOptions" :key="opt.value" :value="opt.value">
              {{ opt.label }}
            </option>
          </select>
        </div>
        
        <!-- Mobile Push -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Mobile push notifications
          </label>
          <select
            v-model="notifyPush"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
          >
            <option v-for="opt in notifyOptions" :key="opt.value" :value="opt.value">
              {{ opt.label }}
            </option>
          </select>
        </div>
        
        <!-- Sounds -->
        <div class="flex items-center justify-between">
          <span class="text-sm text-gray-700 dark:text-gray-300">Notification sounds</span>
          <button
            @click="notifySounds = !notifySounds"
            class="relative w-11 h-6 rounded-full transition-colors"
            :class="notifySounds ? 'bg-primary' : 'bg-gray-300 dark:bg-gray-600'"
          >
            <span
              class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform"
              :class="notifySounds ? 'translate-x-5' : ''"
            ></span>
          </button>
        </div>
      </div>
    </div>
    
    <!-- Do Not Disturb -->
    <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white flex items-center mb-4">
        <Moon class="w-5 h-5 mr-2 text-primary" />
        Do Not Disturb
      </h3>
      
      <div class="space-y-4">
        <div class="flex items-center justify-between">
          <span class="text-sm text-gray-700 dark:text-gray-300">Enable scheduled DND</span>
          <button
            @click="dndEnabled = !dndEnabled"
            class="relative w-11 h-6 rounded-full transition-colors"
            :class="dndEnabled ? 'bg-primary' : 'bg-gray-300 dark:bg-gray-600'"
          >
            <span
              class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform"
              :class="dndEnabled ? 'translate-x-5' : ''"
            ></span>
          </button>
        </div>
        
        <div v-if="dndEnabled" class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Start time
            </label>
            <input
              v-model="dndStartTime"
              type="time"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              End time
            </label>
            <input
              v-model="dndEndTime"
              type="time"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
        </div>
      </div>
    </div>
    
    <!-- Display Settings -->
    <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white flex items-center mb-4">
        <Monitor class="w-5 h-5 mr-2 text-primary" />
        Display
      </h3>
      
      <div class="space-y-4">
        <!-- Message Display -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Message display
          </label>
          <div class="flex space-x-3">
            <button
              @click="messageDisplay = 'standard'"
              class="flex-1 p-3 border rounded-lg transition-colors text-sm"
              :class="messageDisplay === 'standard' 
                ? 'border-primary bg-primary/10 text-primary' 
                : 'border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300'"
            >
              Standard
            </button>
            <button
              @click="messageDisplay = 'compact'"
              class="flex-1 p-3 border rounded-lg transition-colors text-sm"
              :class="messageDisplay === 'compact' 
                ? 'border-primary bg-primary/10 text-primary' 
                : 'border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300'"
            >
              Compact
            </button>
          </div>
        </div>
        
        <!-- Time Format -->
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Time format
          </label>
          <div class="flex space-x-3">
            <button
              @click="timeFormat = '12h'"
              class="flex-1 p-3 border rounded-lg transition-colors text-sm"
              :class="timeFormat === '12h' 
                ? 'border-primary bg-primary/10 text-primary' 
                : 'border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300'"
            >
              12-hour (3:00 PM)
            </button>
            <button
              @click="timeFormat = '24h'"
              class="flex-1 p-3 border rounded-lg transition-colors text-sm"
              :class="timeFormat === '24h' 
                ? 'border-primary bg-primary/10 text-primary' 
                : 'border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300'"
            >
              24-hour (15:00)
            </button>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Save Button -->
    <div class="pt-4">
      <BaseButton @click="handleSave" :loading="preferencesStore.loading" class="w-full">
        Save Preferences
      </BaseButton>
    </div>
  </div>
</template>
