<script setup lang="ts">
import { ref, watch } from 'vue'
import { X, LogOut, Bell, Monitor, Layout, Settings, Phone, User } from 'lucide-vue-next'
import DisplayTab from './display/DisplayTab.vue'
import SidebarTab from './sidebar/SidebarTab.vue'
import AdvancedTab from './advanced/AdvancedTab.vue'
import CallsTab from './calls/CallsTab.vue'
import NotificationsTab from './notifications/NotificationsTab.vue'
import ProfileTab from './profile/ProfileTab.vue'
import { useAuthStore } from '../../stores/auth'
import { useUIStore, type SettingsTab } from '../../stores/ui'

const props = defineProps<{
  isOpen: boolean
}>()

const emit = defineEmits(['close'])

const auth = useAuthStore()
const ui = useUIStore()
const activeTab = ref<SettingsTab>('notifications')
const error = ref('')
const success = ref('')

const tabs: Array<{ id: SettingsTab; label: string; icon: unknown }> = [
  { id: 'notifications', label: 'Notifications', icon: Bell },
  { id: 'display', label: 'Display', icon: Monitor },
  { id: 'sidebar', label: 'Sidebar', icon: Layout },
  { id: 'advanced', label: 'Advanced', icon: Settings },
]

const pluginTabs: Array<{ id: SettingsTab; label: string; icon: unknown }> = [
  { id: 'calls', label: 'Calls', icon: Phone },
]

const allTabs = [...tabs, ...pluginTabs, { id: 'profile' as SettingsTab, label: 'Profile', icon: User }]

// Reset state when modal opens
watch(() => props.isOpen, (isOpen) => {
  if (isOpen) {
    error.value = ''
    success.value = ''
    activeTab.value = allTabs.some((tab) => tab.id === ui.settingsTab) ? ui.settingsTab : 'notifications'
  }
})

function setTab(tab: SettingsTab) {
  activeTab.value = tab
  ui.settingsTab = tab
}

function handleLogout() {
  emit('close')
  auth.logout()
}
</script>

<template>
  <Transition
    enter-active-class="transition-opacity duration-200"
    enter-from-class="opacity-0"
    enter-to-class="opacity-100"
    leave-active-class="transition-opacity duration-150"
    leave-from-class="opacity-100"
    leave-to-class="opacity-0"
  >
    <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4" role="dialog">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="$emit('close')"></div>
      
      <!-- Modal Panel -->
      <div class="relative bg-bg-surface-1 rounded-r-3 shadow-2xl ring-1 ring-border-1 w-full max-w-5xl max-h-[calc(100svh-1rem)] sm:max-h-[90vh] flex flex-col overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-4 sm:px-6 py-4 border-b border-border-1 shrink-0">
          <h2 class="text-xl sm:text-2xl font-semibold text-text-1">Settings</h2>
          <button 
            @click="$emit('close')" 
            class="flex h-11 w-11 items-center justify-center rounded-r-2 text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-standard focus-ring"
          >
            <X class="h-5 w-5" />
          </button>
        </div>

        <div class="flex-1 min-h-0 flex flex-col sm:flex-row overflow-hidden">
          <!-- Sidebar -->
          <div class="w-full sm:w-64 bg-bg-surface-2 border-b sm:border-b-0 sm:border-r border-border-1 flex flex-col shrink-0 overflow-y-auto">
            <div class="border-b border-border-1 px-4 py-4">
              <p class="text-[10px] font-semibold uppercase tracking-[0.2em] text-text-3">Personal settings</p>
              <p class="mt-2 truncate text-sm font-semibold text-text-1">{{ auth.user?.display_name || auth.user?.username || 'Account' }}</p>
              <p class="truncate text-xs text-text-3">@{{ auth.user?.username || 'user' }}</p>
            </div>
            <!-- Main Tabs -->
            <nav class="grid grid-cols-2 gap-2 p-3 sm:flex sm:flex-col sm:gap-0.5 sm:p-2">
              <button
                v-for="tab in tabs"
                :key="tab.id"
                @click="setTab(tab.id)"
                class="flex min-h-11 items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-r-2 whitespace-nowrap transition-standard"
                :class="activeTab === tab.id 
                  ? 'bg-bg-surface-1 text-brand shadow-sm ring-1 ring-border-1' 
                  : 'text-text-2 hover:bg-bg-surface-1 hover:text-text-1'"
              >
                <component :is="tab.icon" class="w-4 h-4 shrink-0" />
                {{ tab.label }}
              </button>
            </nav>

            <!-- Plugin Section -->
            <div class="px-3 pt-0 pb-2 sm:px-4 sm:py-2 text-[10px] font-semibold uppercase tracking-wider text-text-3">
              Plugin Preferences
            </div>
            <nav class="grid grid-cols-2 gap-2 px-3 pb-3 sm:flex sm:flex-col sm:gap-0.5 sm:px-2 sm:pb-2">
              <button
                v-for="tab in pluginTabs"
                :key="tab.id"
                @click="setTab(tab.id)"
                class="flex min-h-11 items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-r-2 whitespace-nowrap transition-standard"
                :class="activeTab === tab.id
                  ? 'bg-bg-surface-1 text-brand shadow-sm ring-1 ring-border-1'
                  : 'text-text-2 hover:bg-bg-surface-1 hover:text-text-1'"
              >
                <component :is="tab.icon" class="w-4 h-4 shrink-0" />
                {{ tab.label }}
              </button>
            </nav>
            
            <!-- Profile & Logout -->
            <div class="grid grid-cols-2 gap-2 p-3 border-t border-border-1 sm:mt-auto sm:block sm:p-2">
              <button
                @click="setTab('profile')"
                class="w-full flex min-h-11 items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-r-2 transition-standard sm:mb-1"
                :class="activeTab === 'profile'
                  ? 'bg-bg-surface-1 text-brand shadow-sm ring-1 ring-border-1'
                  : 'text-text-2 hover:bg-bg-surface-1 hover:text-text-1'"
              >
                <User class="w-4 h-4 shrink-0" />
                Profile
              </button>
              <button
                @click="handleLogout"
                class="w-full flex min-h-11 items-center gap-3 px-3 py-2.5 text-sm font-medium text-danger hover:bg-danger/5 rounded-r-2 transition-standard"
              >
                <LogOut class="w-4 h-4 shrink-0" />
                Log out
              </button>
            </div>
          </div>

          <!-- Content -->
          <div class="flex-1 min-w-0 overflow-y-auto p-4 sm:p-6 bg-bg-surface-1">
            <!-- Messages -->
            <div v-if="error" class="mb-4 p-3 bg-danger/10 border border-danger/20 rounded-r-2 text-danger text-sm">
              {{ error }}
            </div>
            <div v-if="success" class="mb-4 p-3 bg-success/10 border border-success/20 rounded-r-2 text-success text-sm">
              {{ success }}
            </div>

            <!-- Tab Content -->
            <div class="max-w-2xl">
              <!-- Profile Tab -->
              <div v-if="activeTab === 'profile'">
                <h3 class="text-lg font-semibold text-text-1 mb-4">Profile</h3>
                <ProfileTab />
              </div>

              <!-- Notifications Tab -->
              <div v-else-if="activeTab === 'notifications'">
                <h3 class="text-lg font-semibold text-text-1 mb-1">Notifications</h3>
                <p class="text-sm text-text-3 mb-6">Manage how you receive notifications.</p>
                <NotificationsTab />
              </div>

              <!-- Display Tab -->
              <div v-else-if="activeTab === 'display'">
                <h3 class="text-lg font-semibold text-text-1 mb-1">Display</h3>
                <p class="text-sm text-text-3 mb-6">Customize your display preferences.</p>
                <DisplayTab />
              </div>

              <!-- Sidebar Tab -->
              <div v-else-if="activeTab === 'sidebar'">
                <h3 class="text-lg font-semibold text-text-1 mb-1">Sidebar</h3>
                <p class="text-sm text-text-3 mb-6">Configure your sidebar preferences.</p>
                <SidebarTab />
              </div>

              <!-- Advanced Tab -->
              <div v-else-if="activeTab === 'advanced'">
                <h3 class="text-lg font-semibold text-text-1 mb-1">Advanced</h3>
                <p class="text-sm text-text-3 mb-6">Advanced settings and options.</p>
                <AdvancedTab />
              </div>

              <!-- Calls Tab -->
              <div v-else-if="activeTab === 'calls'">
                <h3 class="text-lg font-semibold text-text-1 mb-1">Calls</h3>
                <p class="text-sm text-text-3 mb-6">Configure your call preferences.</p>
                <CallsTab />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>
