<script setup lang="ts">
import { ref, watch } from 'vue'
import { Camera } from 'lucide-vue-next'
import BaseButton from '../../atomic/BaseButton.vue'
import BaseInput from '../../atomic/BaseInput.vue'
import api from '../../../api/client'
import { useAuthStore } from '../../../stores/auth'
import { usersApi } from '../../../api/users'

const auth = useAuthStore()
const loading = ref(false)
const error = ref('')
const success = ref('')
const fileInput = ref<HTMLInputElement | null>(null)

// Profile form fields
const username = ref('')
const displayName = ref('')
const avatarUrl = ref('')
const firstName = ref('')
const lastName = ref('')
const nickname = ref('')
const position = ref('')

// Status fields
const statusText = ref('')
const statusEmoji = ref('')

const errorClass = 'rounded-r-2 border border-danger/20 bg-danger/10 p-3 text-sm text-danger'
const successClass = 'rounded-r-2 border border-success/20 bg-success/10 p-3 text-sm text-success'
const sectionClass = 'rounded-r-3 border border-border-1 bg-bg-surface-1 p-5 shadow-1'
const sectionHeaderClass = 'border-b border-border-1 pb-3'
const sectionTitleClass = 'text-sm font-semibold tracking-[0.01em] text-text-1'
const sectionBodyClass = 'mt-4 space-y-4'
const helperTextClass = 'text-xs text-text-3'
const readOnlyCardClass = 'rounded-r-2 border border-border-1 bg-bg-surface-2 px-3 py-2 text-sm text-text-3 break-all'

// Initialize from auth user
watch(() => auth.user, (user) => {
  if (user) {
    username.value = user.username || ''
    displayName.value = user.display_name || ''
    avatarUrl.value = user.avatar_url || ''
    firstName.value = user.first_name || ''
    lastName.value = user.last_name || ''
    nickname.value = user.nickname || ''
    position.value = user.position || ''
    statusText.value = user.status_text || ''
    statusEmoji.value = user.status_emoji || ''
  }
}, { immediate: true })

async function handleFileUpload(event: Event) {
  const input = event.target as HTMLInputElement
  if (input.files && input.files[0]) {
    const file = input.files[0]
    
    if (!file.type.startsWith('image/')) {
      error.value = 'Please select a valid image file'
      return
    }

    if (file.size > 5 * 1024 * 1024) {
      error.value = 'Image size must be less than 5MB'
      return
    }

    loading.value = true
    error.value = ''
    
    try {
      const formData = new FormData()
      formData.append('image', file)

      await api.post(`/users/${auth.user?.id}/image`, formData, {
        baseURL: '/api/v4',
      })
      await auth.fetchMe()
      avatarUrl.value = auth.user?.avatar_url || ''
      success.value = 'Avatar uploaded successfully!'
    } catch (e: any) {
      error.value = e.response?.data?.message || 'Failed to upload avatar'
    } finally {
      loading.value = false
      input.value = ''
    }
  }
}

async function handleSaveProfile() {
  if (!auth.user) return
  
  loading.value = true
  error.value = ''
  success.value = ''

  try {
    const response = await usersApi.update(auth.user.id, {
      username: username.value,
      display_name: displayName.value,
      avatar_url: avatarUrl.value,
      first_name: firstName.value,
      last_name: lastName.value,
      nickname: nickname.value,
      position: position.value,
    })
    
    // Update local auth state
    auth.user = {
      ...auth.user,
      username: response.data.username,
      display_name: response.data.display_name,
      avatar_url: response.data.avatar_url,
      first_name: firstName.value,
      last_name: lastName.value,
      nickname: nickname.value,
      position: position.value,
    }
    success.value = 'Profile updated successfully!'
    setTimeout(() => success.value = '', 3000)
  } catch (e: any) {
    error.value = e.response?.data?.message || 'Failed to update profile'
  } finally {
    loading.value = false
  }
}

async function handleSaveStatus() {
  loading.value = true
  error.value = ''
  success.value = ''
  
  try {
    await auth.updateStatus({
      text: statusText.value,
      emoji: statusEmoji.value
    })
    success.value = 'Status updated successfully!'
    setTimeout(() => success.value = '', 3000)
  } catch (e: any) {
    error.value = 'Failed to update status'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="space-y-6">
    <!-- Messages -->
    <div v-if="error" :class="errorClass">
      {{ error }}
    </div>
    <div v-if="success" :class="successClass">
      {{ success }}
    </div>

    <!-- Profile Section -->
    <div :class="sectionClass">
      <div :class="sectionHeaderClass">
        <h4 :class="sectionTitleClass">Profile Information</h4>
        <p class="mt-1 text-xs text-text-3">Keep your identity details current so teammates can recognize and trust who is speaking.</p>
      </div>
      
      <div :class="sectionBodyClass">
        <!-- Avatar -->
        <div class="flex items-center space-x-4">
          <div class="relative group">
            <div class="flex h-16 w-16 items-center justify-center overflow-hidden rounded-full bg-brand text-xl font-bold text-brand-foreground ring-2 ring-transparent transition-all group-hover:ring-brand/35 sm:h-20 sm:w-20 sm:text-2xl">
              <img v-if="avatarUrl" :src="avatarUrl" alt="Avatar" class="h-full w-full object-cover" />
              <span v-else>{{ auth.user?.username?.charAt(0).toUpperCase() || 'U' }}</span>
            </div>
            <button
              type="button"
              @click="fileInput?.click()"
              class="absolute bottom-0 right-0 flex h-7 w-7 items-center justify-center rounded-full border-2 border-bg-surface-1 bg-brand text-brand-foreground shadow-1 transition-standard hover:bg-brand-hover sm:h-8 sm:w-8"
            >
              <Camera class="h-3.5 w-3.5 sm:h-4 sm:w-4" />
            </button>
            <input ref="fileInput" type="file" accept="image/*" class="hidden" @change="handleFileUpload" />
          </div>
          <div>
            <p class="text-sm font-semibold text-text-1">{{ auth.user?.username }}</p>
            <p :class="helperTextClass">
              <button type="button" @click="fileInput?.click()" class="font-medium text-brand transition-standard hover:text-brand-hover">Upload a new photo</button>
            </p>
          </div>
        </div>

        <!-- Form Fields -->
        <div class="grid grid-cols-1 gap-4">
          <BaseInput label="Username" v-model="username" placeholder="your_username" :disabled="loading" />
          <BaseInput label="Display Name" v-model="displayName" placeholder="Your Name" :disabled="loading" />
          <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <BaseInput label="First Name" v-model="firstName" placeholder="John" :disabled="loading" />
            <BaseInput label="Last Name" v-model="lastName" placeholder="Doe" :disabled="loading" />
          </div>
          <BaseInput label="Nickname" v-model="nickname" placeholder="Johnny" :disabled="loading" />
          <BaseInput label="Position" v-model="position" placeholder="Software Engineer" :disabled="loading" />
          <BaseInput label="Avatar URL" v-model="avatarUrl" placeholder="https://example.com/avatar.jpg" :disabled="loading" />
          <div class="space-y-1">
            <label class="block text-sm font-medium text-text-2">Email</label>
            <div :class="readOnlyCardClass">
              {{ auth.user?.email }}
            </div>
          </div>
        </div>
      
        <div class="flex justify-end">
          <BaseButton @click="handleSaveProfile" :loading="loading">Save Profile</BaseButton>
        </div>
      </div>
    </div>

    <!-- Status Section -->
    <div :class="sectionClass">
      <div :class="sectionHeaderClass">
        <h4 :class="sectionTitleClass">Custom Status</h4>
        <p class="mt-1 text-xs text-text-3">Share quick context with teammates when you are heads-down, away, or on call.</p>
      </div>
      
      <div :class="sectionBodyClass">
        <div class="space-y-4">
          <div class="flex space-x-2">
            <div class="w-12">
              <BaseInput v-model="statusEmoji" placeholder="😀" class="text-center" :disabled="loading" />
            </div>
            <div class="flex-1">
              <BaseInput v-model="statusText" placeholder="What's your status?" :disabled="loading" />
            </div>
          </div>
          <p :class="helperTextClass">Enter an emoji and a short message to describe your status.</p>
        </div>

        <div class="flex justify-end">
          <BaseButton @click="handleSaveStatus" :loading="loading">Update Status</BaseButton>
        </div>
      </div>
    </div>
  </div>
</template>
