<script setup lang="ts">
import { X } from 'lucide-vue-next'
import BaseButton from '../atomic/BaseButton.vue'

defineProps<{
  label: string
  description?: string
  loading?: boolean
  saveDisabled?: boolean
}>()

defineEmits<{
  save: []
  cancel: []
}>()
</script>

<template>
  <div class="rounded-r-2 border border-border-1 bg-bg-surface-2">
    <!-- Header -->
    <div class="flex items-center justify-between border-b border-border-1 px-4 py-3">
      <div>
        <h4 class="text-sm font-semibold text-text-1">{{ label }}</h4>
        <p v-if="description" class="mt-0.5 text-xs text-text-3">{{ description }}</p>
      </div>
      <button
        type="button"
        class="rounded-r-1 p-1 text-text-3 transition-standard hover:bg-bg-surface-1 hover:text-text-2"
        @click="$emit('cancel')"
      >
        <X class="w-4 h-4" />
      </button>
    </div>

    <!-- Content slot -->
    <div class="p-4">
      <slot />
    </div>

    <!-- Actions -->
    <div class="flex items-center justify-end gap-2 border-t border-border-1 px-4 py-3">
      <BaseButton size="sm" variant="secondary" :disabled="loading" @click="$emit('cancel')">
        Cancel
      </BaseButton>
      <BaseButton size="sm" :loading="loading" :disabled="saveDisabled" @click="$emit('save')">
        Save
      </BaseButton>
    </div>
  </div>
</template>
