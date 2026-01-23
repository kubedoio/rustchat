<script setup lang="ts">
import { ref, watch } from 'vue'
import { X, User, Moon, Bell, Shield, Camera, LogOut } from 'lucide-vue-next'
import BaseButton from '../atomic/BaseButton.vue'
import BaseInput from '../atomic/BaseInput.vue'
import { useAuthStore } from '../../stores/auth'
import { usersApi } from '../../api/users'
import { filesApi } from '../../api/files'

const props = defineProps<{
  isOpen: boolean
}>()

const emit = defineEmits(['close'])

const auth = useAuthStore()
const activeTab = ref('profile')
const loading = ref(false)
const error = ref('')
const success = ref('')
const fileInput = ref<HTMLInputElement | null>(null)

// Profile form fields
const username = ref('')
const displayName = ref('')
const avatarUrl = ref('')

const tabs = [
  { id: 'profile', label: 'Profile', icon: User },
  { id: 'appearance', label: 'Appearance', icon: Moon },
  { id: 'notifications', label: 'Notifications', icon: Bell },
  { id: 'security', label: 'Security', icon: Shield },
]

const isDarkMode = ref(document.documentElement.classList.contains('dark'))

// Security form fields
const newPassword = ref('')
const confirmPassword = ref('')
const passwordPolicy = ref<any>(null)

// Fetch auth policy
async function fetchPolicy() {
    try {
        const { data } = await usersApi.getAuthPolicy()
        passwordPolicy.value = data
    } catch (e) {
        console.error('Failed to fetch auth policy', e)
    }
}

// Populate form when modal opens
watch(() => props.isOpen, (isOpen) => {
  if (isOpen && auth.user) {
    username.value = auth.user.username || ''
    displayName.value = auth.user.display_name || ''
    avatarUrl.value = auth.user.avatar_url || ''
    error.value = ''
    success.value = ''
  }
})

function toggleDarkMode() {
  isDarkMode.value = !isDarkMode.value
  if (isDarkMode.value) {
    document.documentElement.classList.add('dark')
    localStorage.setItem('theme', 'dark')
  } else {
    document.documentElement.classList.remove('dark')
    localStorage.setItem('theme', 'light')
  }
}

async function handleFileUpload(event: Event) {
  const input = event.target as HTMLInputElement
  if (input.files && input.files[0]) {
    const file = input.files[0]
    
    // Validate file type
    if (!file.type.startsWith('image/')) {
        error.value = 'Please select a valid image file'
        return
    }

    // Validate size (e.g. 5MB)
    if (file.size > 5 * 1024 * 1024) {
        error.value = 'Image size must be less than 5MB'
        return
    }

    loading.value = true
    error.value = ''
    
    try {
        const response = await filesApi.upload(file)
        avatarUrl.value = response.data.url
        success.value = 'Avatar uploaded successfully! Click Save to apply.'
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to upload avatar'
    } finally {
        loading.value = false
        // Reset input
        input.value = ''
    }
  }
}

// Watch tab changes to fetch policy
watch(activeTab, (tab) => {
    if (tab === 'security' && !passwordPolicy.value) {
        fetchPolicy()
    }
})

async function handlePasswordChange() {
    if (!auth.user) return
    
    if (newPassword.value !== confirmPassword.value) {
        error.value = 'New passwords do not match'
        return
    }

    loading.value = true
    error.value = ''
    success.value = ''

    try {
        await usersApi.changePassword(auth.user.id, {
            new_password: newPassword.value,
        })
        success.value = 'Password changed successfully!'
        newPassword.value = ''
        confirmPassword.value = ''
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to change password'
    } finally {
        loading.value = false
    }
}

async function handleSave() {
  if (activeTab.value === 'profile' && auth.user) {
    loading.value = true
    error.value = ''
    success.value = ''

    try {
      const response = await usersApi.update(auth.user.id, {
        username: username.value.trim() || undefined,
        display_name: displayName.value.trim() || undefined,
        avatar_url: avatarUrl.value.trim() || undefined,
      })
      
      // Update the auth store user with new data
      auth.user = {
        ...auth.user,
        username: response.data.username,
        display_name: response.data.display_name,
        avatar_url: response.data.avatar_url,
      }
      success.value = 'Profile updated successfully!'
      
      setTimeout(() => {
        success.value = ''
      }, 3000)
    } catch (e: any) {
      error.value = e.response?.data?.message || 'Failed to update profile'
    } finally {
      loading.value = false
    }
  } else {
    emit('close')
  }
}

function handleLogout() {
  emit('close')
  auth.logout()
}
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4" role="dialog">
     <!-- Backdrop -->
    <div class="fixed inset-0 bg-gray-500/75 dark:bg-black/70" @click="$emit('close')"></div>
    
    <!-- Modal Panel -->
    <div class="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl ring-1 ring-black/5 w-full max-w-3xl max-h-[90vh] flex flex-col sm:flex-row overflow-hidden">
        
        <!-- Sidebar -->
        <div class="w-full sm:w-56 bg-gray-50 dark:bg-gray-900 border-b sm:border-b-0 sm:border-r border-gray-200 dark:border-gray-700 flex flex-col shrink-0">
            <div class="p-4 sm:p-6 font-bold text-lg dark:text-white">Settings</div>
            <nav class="flex sm:flex-col gap-1 px-2 sm:px-3 pb-2 sm:pb-0 overflow-x-auto sm:overflow-x-visible">
                <button
                    v-for="tab in tabs"
                    :key="tab.id"
                    @click="activeTab = tab.id"
                    class="flex items-center px-3 py-2 text-sm font-medium rounded-md whitespace-nowrap"
                    :class="activeTab === tab.id 
                        ? 'bg-gray-200 dark:bg-gray-800 text-gray-900 dark:text-white' 
                        : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'"
                >
                    <component :is="tab.icon" class="mr-2 flex-shrink-0 h-4 w-4 sm:h-5 sm:w-5" />
                    {{ tab.label }}
                </button>
            </nav>
            
            <!-- Logout Button - Desktop -->
            <div class="hidden sm:block mt-auto p-3 border-t border-gray-200 dark:border-gray-700">
              <button
                @click="handleLogout"
                class="flex items-center w-full px-3 py-2 text-sm font-medium text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-md transition-colors"
              >
                <LogOut class="mr-2 h-5 w-5" />
                Log out
              </button>
            </div>
        </div>

        <!-- Content -->
        <div class="flex-1 flex flex-col min-w-0 min-h-0">
            <div class="flex items-center justify-between px-4 sm:px-6 py-3 sm:py-4 border-b border-gray-200 dark:border-gray-700 shrink-0">
                <h3 class="text-base sm:text-lg font-medium leading-6 text-gray-900 dark:text-white capitalize">
                    {{ activeTab }} Settings
                </h3>
                <button @click="$emit('close')" class="rounded-md bg-white dark:bg-gray-800 text-gray-400 hover:text-gray-500 focus:outline-none p-1">
                    <X class="h-5 w-5 sm:h-6 sm:w-6" />
                </button>
            </div>
            
            <div class="flex-1 overflow-y-auto p-4 sm:p-6">
                <!-- Profile Tab -->
                <div v-if="activeTab === 'profile'" class="space-y-4 sm:space-y-6">
                    <!-- Messages -->
                    <div v-if="error" class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-600 dark:text-red-400 text-sm">
                      {{ error }}
                    </div>
                    <div v-if="success" class="p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg text-green-600 dark:text-green-400 text-sm">
                      {{ success }}
                    </div>

                    <!-- Avatar -->
                    <div class="flex items-center space-x-4">
                        <div class="relative group">
                          <div class="h-16 w-16 sm:h-20 sm:w-20 rounded-full bg-primary flex items-center justify-center text-xl sm:text-2xl text-white font-bold overflow-hidden ring-2 ring-transparent group-hover:ring-primary/50 transition-all">
                            <img v-if="avatarUrl" :src="avatarUrl" alt="Avatar" class="w-full h-full object-cover" />
                            <span v-else>{{ auth.user?.username?.charAt(0).toUpperCase() || 'U' }}</span>
                          </div>
                          <button 
                            type="button"
                            @click="fileInput?.click()"
                            class="absolute bottom-0 right-0 w-6 h-6 sm:w-7 sm:h-7 bg-gray-800 dark:bg-gray-600 rounded-full flex items-center justify-center border-2 border-white dark:border-gray-800 hover:bg-gray-700 dark:hover:bg-gray-500 transition-colors"
                          >
                            <Camera class="w-3 h-3 sm:w-3.5 sm:h-3.5 text-white" />
                          </button>
                          <!-- Hidden file input -->
                          <input 
                            ref="fileInput"
                            type="file" 
                            accept="image/*" 
                            class="hidden" 
                            @change="handleFileUpload"
                          />
                        </div>
                        <div>
                          <p class="text-sm font-medium text-gray-900 dark:text-white">{{ auth.user?.username }}</p>
                          <p class="text-xs text-gray-500">
                             <button type="button" @click="fileInput?.click()" class="text-primary hover:underline">Click to upload</button> or paste URL
                          </p>
                        </div>
                    </div>

                    <BaseInput label="Username" v-model="username" placeholder="your_username" :disabled="loading" />
                    <BaseInput label="Display Name" v-model="displayName" placeholder="Your Name" :disabled="loading" />
                    <BaseInput label="Avatar URL" v-model="avatarUrl" placeholder="https://example.com/avatar.jpg" :disabled="loading" />
                    
                    <!-- Email (read-only) -->
                    <div class="space-y-1">
                      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Email</label>
                      <div class="px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg text-gray-600 dark:text-gray-400 text-sm break-all">
                        {{ auth.user?.email }}
                      </div>
                    </div>
                    
                    <!-- Logout Button - Mobile -->
                    <div class="sm:hidden pt-4 border-t border-gray-200 dark:border-gray-700">
                      <button
                        @click="handleLogout"
                        class="flex items-center justify-center w-full px-4 py-2 text-sm font-medium text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 hover:bg-red-100 dark:hover:bg-red-900/30 rounded-lg transition-colors"
                      >
                        <LogOut class="mr-2 h-4 w-4" />
                        Log out
                      </button>
                    </div>
                </div>

                <!-- Appearance Tab -->
                <div v-else-if="activeTab === 'appearance'" class="space-y-6">
                    <div class="flex items-center justify-between py-4 px-4 bg-gray-50 dark:bg-gray-900 rounded-lg">
                        <div>
                            <h4 class="text-base font-medium text-gray-900 dark:text-white">Dark Mode</h4>
                            <p class="text-sm text-gray-500">Use dark theme for the application</p>
                        </div>
                        <button 
                          @click="toggleDarkMode"
                          class="relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none"
                          :class="isDarkMode ? 'bg-primary' : 'bg-gray-300'"
                        >
                          <span 
                            class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out"
                            :class="isDarkMode ? 'translate-x-5' : 'translate-x-0'"
                          ></span>
                        </button>
                    </div>
                </div>

                <!-- Security Tab -->
                <div v-else-if="activeTab === 'security'" class="space-y-6">
                    <!-- Messages -->
                    <div v-if="error" class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-600 dark:text-red-400 text-sm">
                      {{ error }}
                    </div>
                    <div v-if="success" class="p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg text-green-600 dark:text-green-400 text-sm">
                      {{ success }}
                    </div>

                    <div class="space-y-4">
                        <h4 class="text-sm font-semibold text-gray-900 dark:text-white uppercase tracking-wider">Change Password</h4>
                        
                        <div class="space-y-4 bg-gray-50 dark:bg-gray-900/50 p-4 rounded-xl border border-gray-200 dark:border-gray-700">
                            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                <BaseInput 
                                    label="New Password" 
                                    v-model="newPassword" 
                                    type="password" 
                                    placeholder="••••••••" 
                                    :disabled="loading" 
                                />
                                <BaseInput 
                                    label="Confirm New Password" 
                                    v-model="confirmPassword" 
                                    type="password" 
                                    placeholder="••••••••" 
                                    :disabled="loading" 
                                />
                            </div>

                            <!-- Password Requirements -->
                            <div v-if="passwordPolicy" class="mt-4 p-3 bg-indigo-50 dark:bg-indigo-900/20 rounded-lg">
                                <p class="text-xs font-medium text-indigo-700 dark:text-indigo-300 mb-2">Password Requirements:</p>
                                <ul class="text-[11px] space-y-1 text-indigo-600 dark:text-indigo-400">
                                    <li class="flex items-center">
                                        <div class="w-1 h-1 rounded-full bg-indigo-400 mr-2"></div>
                                        Minimum length: {{ passwordPolicy.password_min_length }} characters
                                    </li>
                                    <li v-if="passwordPolicy.password_require_uppercase" class="flex items-center">
                                        <div class="w-1 h-1 rounded-full bg-indigo-400 mr-2"></div>
                                        Must contain an uppercase letter
                                    </li>
                                    <li v-if="passwordPolicy.password_require_number" class="flex items-center">
                                        <div class="w-1 h-1 rounded-full bg-indigo-400 mr-2"></div>
                                        Must contain a number
                                    </li>
                                    <li v-if="passwordPolicy.password_require_symbol" class="flex items-center">
                                        <div class="w-1 h-1 rounded-full bg-indigo-400 mr-2"></div>
                                        Must contain a special character
                                    </li>
                                </ul>
                            </div>

                            <div class="flex justify-end pt-2">
                                <BaseButton 
                                    size="sm" 
                                    @click="handlePasswordChange" 
                                    :loading="loading"
                                    :disabled="!newPassword || !confirmPassword"
                                >
                                    Update Password
                                </BaseButton>
                            </div>
                        </div>
                    </div>

                    <div class="pt-6 border-t border-gray-200 dark:border-gray-700">
                        <h4 class="text-sm font-semibold text-gray-900 dark:text-white uppercase tracking-wider mb-4">Two-Factor Authentication</h4>
                        <div class="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-900/50 rounded-xl border border-gray-200 dark:border-gray-700">
                            <div>
                                <p class="text-sm font-medium text-gray-900 dark:text-white">Authenticator App</p>
                                <p class="text-xs text-gray-500">Secure your account with TOTP</p>
                            </div>
                            <BaseButton variant="secondary" size="sm" disabled>Configure</BaseButton>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Footer -->
            <div class="px-4 sm:px-6 py-3 sm:py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end space-x-3 shrink-0">
                <BaseButton variant="secondary" @click="$emit('close')">Cancel</BaseButton>
                <BaseButton @click="handleSave" :loading="loading">Save Changes</BaseButton>
            </div>
        </div>
    </div>
  </div>
</template>
