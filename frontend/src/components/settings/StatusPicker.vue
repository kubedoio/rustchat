<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { X, Clock, Check } from 'lucide-vue-next'
import { usePreferencesStore } from '../../stores/preferences'
import EmojiPicker from '../atomic/EmojiPicker.vue'
import BaseButton from '../atomic/BaseButton.vue'

const props = defineProps<{
  isOpen: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const preferencesStore = usePreferencesStore()

const statusText = ref('')
const statusEmoji = ref('')
const duration = ref<number | null>(null)
const showEmojiPicker = ref(false)

const durationOptions = [
  { value: null, label: "Don't clear" },
  { value: 30, label: '30 minutes' },
  { value: 60, label: '1 hour' },
  { value: 240, label: '4 hours' },
  { value: 480, label: 'Today' },
  { value: 1440, label: 'This week' },
]

onMounted(() => {
  preferencesStore.fetchStatusPresets()
  if (preferencesStore.status) {
    statusText.value = preferencesStore.status.text || ''
    statusEmoji.value = preferencesStore.status.emoji || ''
  }
})

const presets = computed(() => preferencesStore.statusPresets)

function selectEmoji(emoji: string) {
  statusEmoji.value = emoji
  showEmojiPicker.value = false
}

function applyPreset(preset: { emoji: string; text: string; duration_minutes: number | null }) {
  statusEmoji.value = preset.emoji
  statusText.value = preset.text
  duration.value = preset.duration_minutes
}

async function handleSave() {
  await preferencesStore.updateStatus({
    text: statusText.value || undefined,
    emoji: statusEmoji.value || undefined,
    duration_minutes: duration.value || undefined,
  })
  emit('close')
}

async function handleClear() {
  await preferencesStore.clearStatus()
  statusText.value = ''
  statusEmoji.value = ''
  duration.value = null
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="fixed inset-0 bg-black/50" @click="$emit('close')"></div>
      
      <!-- Modal -->
      <div class="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-md overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Set a status</h2>
          <button @click="$emit('close')" class="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded">
            <X class="w-5 h-5 text-gray-400" />
          </button>
        </div>
        
        <!-- Content -->
        <div class="p-6 space-y-5">
          <!-- Status Input -->
          <div class="flex items-center space-x-3">
            <button 
              @click="showEmojiPicker = !showEmojiPicker"
              class="w-10 h-10 rounded-lg bg-gray-100 dark:bg-gray-700 flex items-center justify-center text-xl hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
            >
              {{ statusEmoji || '😀' }}
            </button>
            <input
              v-model="statusText"
              type="text"
              placeholder="What's your status?"
              class="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary focus:border-transparent"
            />
          </div>
          
          <!-- Emoji Picker -->
          <div v-if="showEmojiPicker" class="relative">
            <EmojiPicker :show="showEmojiPicker" @select="selectEmoji" @close="showEmojiPicker = false" />
          </div>
          
          <!-- Quick Presets -->
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Quick select
            </label>
            <div class="flex flex-wrap gap-2">
              <button
                v-for="preset in presets"
                :key="preset.id"
                @click="applyPreset(preset)"
                class="px-3 py-1.5 text-sm rounded-full border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors flex items-center space-x-1"
              >
                <span>{{ preset.emoji }}</span>
                <span class="text-gray-700 dark:text-gray-300">{{ preset.text }}</span>
              </button>
            </div>
          </div>
          
          <!-- Duration -->
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              <Clock class="w-4 h-4 inline mr-1" />
              Clear after
            </label>
            <select
              v-model="duration"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            >
              <option v-for="opt in durationOptions" :key="opt.label" :value="opt.value">
                {{ opt.label }}
              </option>
            </select>
          </div>
        </div>
        
        <!-- Footer -->
        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-between">
          <BaseButton v-if="preferencesStore.hasStatus" variant="secondary" @click="handleClear">
            Clear Status
          </BaseButton>
          <div v-else></div>
          <div class="flex space-x-3">
            <BaseButton variant="secondary" @click="$emit('close')">Cancel</BaseButton>
            <BaseButton @click="handleSave" :loading="preferencesStore.loading">
              <Check class="w-4 h-4 mr-1" />
              Save
            </BaseButton>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
