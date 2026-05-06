<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { X, Loader2 } from 'lucide-vue-next'
import { channelService } from '../../features/channels'
import type { Channel } from '../../core/entities/Channel'

interface FormData {
  name: string
  displayName: string
  purpose: string
}

const props = defineProps<{
  isOpen: boolean
  channel: Channel | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated', channel: Channel): void
}>()

const formData = ref<FormData>({
  name: '',
  displayName: '',
  purpose: ''
})

const isLoading = ref(false)
const errors = ref<Record<string, string>>({})

// Reset form when channel changes or modal opens
watch(() => props.channel, (channel) => {
  if (channel) {
    formData.value = {
      name: channel.name || '',
      displayName: channel.displayName || '',
      purpose: channel.purpose || ''
    }
  }
  errors.value = {}
}, { immediate: true })

// Validate name format (lowercase, no spaces, alphanumeric + hyphen + underscore)
const nameError = computed(() => {
  const name = formData.value.name
  if (!name) return 'Channel name is required'
  if (name.length < 2) return 'Channel name must be at least 2 characters'
  if (name.length > 64) return 'Channel name must be at most 64 characters'
  if (name.includes(' ')) return 'Channel name cannot contain spaces'
  if (!/^[a-z0-9_-]+$/.test(name)) return 'Channel name can only contain lowercase letters, numbers, hyphens, and underscores'
  return null
})

const displayNameError = computed(() => {
  const displayName = formData.value.displayName
  if (!displayName?.trim()) return 'Display name is required'
  if (displayName.length > 64) return 'Display name must be at most 64 characters'
  return null
})

const isValid = computed(() => {
  return !nameError.value && !displayNameError.value
})

function handleClose() {
  if (!isLoading.value) {
    emit('close')
  }
}

async function handleSubmit() {
  if (!props.channel || !isValid.value || isLoading.value) return

  isLoading.value = true
  errors.value = {}

  try {
    const updatedChannel = await channelService.updateChannel(props.channel.id, {
      name: formData.value.name.toLowerCase().trim(),
      displayName: formData.value.displayName.trim(),
      purpose: formData.value.purpose.trim() || undefined
    })
    
    emit('updated', updatedChannel)
    emit('close')
  } catch (error: any) {
    const message = error?.message || error?.response?.data?.message || 'Failed to update channel'
    errors.value.submit = message
  } finally {
    isLoading.value = false
  }
}

function handleBackdropClick(event: MouseEvent) {
  if (event.target === event.currentTarget) {
    handleClose()
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div
        v-if="isOpen"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm"
        @click="handleBackdropClick"
      >
        <div
          class="w-full max-w-md bg-bg-surface-1 rounded-xl shadow-2xl border border-border-1 overflow-hidden"
          @click.stop
        >
          <!-- Header -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-border-1">
            <h2 class="text-lg font-semibold text-text-1">Edit Channel</h2>
            <button
              @click="handleClose"
              class="p-1.5 rounded-lg text-text-3 hover:text-text-1 hover:bg-bg-surface-2 transition-colors"
              :disabled="isLoading"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Form -->
          <form @submit.prevent="handleSubmit" class="p-6 space-y-4">
            <!-- Name Field -->
            <div class="space-y-1.5">
              <label for="channel-name" class="block text-sm font-medium text-text-2">
                Channel Name <span class="text-danger">*</span>
                <span class="text-xs text-text-3 font-normal ml-1">(URL-friendly, lowercase)</span>
              </label>
              <input
                id="channel-name"
                v-model="formData.name"
                type="text"
                placeholder="e.g., general"
                class="w-full px-3 py-2 bg-bg-surface-2 border border-border-1 rounded-lg text-text-1 placeholder:text-text-3 focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent transition-all"
                :class="{ 'border-danger': nameError && formData.name }"
                :disabled="isLoading"
              />
              <p v-if="nameError && formData.name" class="text-xs text-danger">{{ nameError }}</p>
              <p v-else class="text-xs text-text-3">
                Used in the channel URL. Only lowercase letters, numbers, hyphens, and underscores.
              </p>
            </div>

            <!-- Display Name Field -->
            <div class="space-y-1.5">
              <label for="channel-display-name" class="block text-sm font-medium text-text-2">
                Display Name <span class="text-danger">*</span>
              </label>
              <input
                id="channel-display-name"
                v-model="formData.displayName"
                type="text"
                placeholder="e.g., General Discussion"
                class="w-full px-3 py-2 bg-bg-surface-2 border border-border-1 rounded-lg text-text-1 placeholder:text-text-3 focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent transition-all"
                :class="{ 'border-danger': displayNameError && formData.displayName }"
                :disabled="isLoading"
              />
              <p v-if="displayNameError && formData.displayName" class="text-xs text-danger">{{ displayNameError }}</p>
            </div>

            <!-- Purpose Field -->
            <div class="space-y-1.5">
              <label for="channel-purpose" class="block text-sm font-medium text-text-2">
                Description
              </label>
              <textarea
                id="channel-purpose"
                v-model="formData.purpose"
                rows="3"
                placeholder="What is this channel about?"
                class="w-full px-3 py-2 bg-bg-surface-2 border border-border-1 rounded-lg text-text-1 placeholder:text-text-3 focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent transition-all resize-none"
                :disabled="isLoading"
              />
              <p class="text-xs text-text-3">
                Optional description to help others understand the channel's purpose.
              </p>
            </div>

            <!-- Error Message -->
            <div v-if="errors.submit" class="p-3 bg-danger/10 border border-danger/20 rounded-lg">
              <p class="text-sm text-danger">{{ errors.submit }}</p>
            </div>

            <!-- Actions -->
            <div class="flex items-center justify-end gap-3 pt-2">
              <button
                type="button"
                @click="handleClose"
                class="px-4 py-2 text-sm font-medium text-text-2 hover:text-text-1 bg-bg-surface-2 hover:bg-bg-surface-3 rounded-lg transition-colors"
                :disabled="isLoading"
              >
                Cancel
              </button>
              <button
                type="submit"
                class="flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-accent-primary hover:bg-accent-primary/90 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                :disabled="!isValid || isLoading"
              >
                <Loader2 v-if="isLoading" class="w-4 h-4 animate-spin" />
                <span>{{ isLoading ? 'Saving...' : 'Save Changes' }}</span>
              </button>
            </div>
          </form>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-fade-enter-active,
.modal-fade-leave-active {
  transition: opacity 0.2s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}

.modal-fade-enter-active .bg-bg-surface-1,
.modal-fade-leave-active .bg-bg-surface-1 {
  transition: transform 0.2s ease;
}

.modal-fade-enter-from .bg-bg-surface-1,
.modal-fade-leave-to .bg-bg-surface-1 {
  transform: scale(0.95);
}
</style>
