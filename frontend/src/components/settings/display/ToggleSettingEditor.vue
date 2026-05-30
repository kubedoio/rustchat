<script setup lang="ts">
defineProps<{
  modelValue: boolean
  title: string
  description: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const optionToggleCardClass =
  'flex items-center justify-between rounded-lg border border-border-1 bg-bg-surface-1 p-3 text-text-1 transition-standard hover:bg-bg-surface-2'
const optionTitleClass = 'text-sm font-medium text-text-1'
const optionDescriptionClass = 'text-xs text-text-3'
const checkboxClass = 'h-5 w-5 cursor-pointer rounded accent-brand'
</script>

<template>
  <div class="space-y-3">
    <label :class="optionToggleCardClass">
      <div>
        <div :class="optionTitleClass">{{ title }}</div>
        <div :class="optionDescriptionClass">{{ description }}</div>
      </div>
      <input
        type="checkbox"
        :checked="modelValue"
        :class="checkboxClass"
        @change="emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
      />
    </label>
  </div>
</template>
