<script setup lang="ts">
interface RadioOption {
  value: string | number | boolean
  label: string
  description?: string
  style?: Record<string, string>
}

defineProps<{
  modelValue: string | number | boolean
  options: RadioOption[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string | number | boolean]
}>()

const optionCardClass =
  'flex items-center gap-3 rounded-lg border border-border-1 bg-bg-surface-1 p-3 text-text-1 transition-standard hover:bg-bg-surface-2'
const optionTitleClass = 'text-sm font-medium text-text-1'
const optionDescriptionClass = 'text-xs text-text-3'
const radioClass = 'h-4 w-4 cursor-pointer accent-brand'
</script>

<template>
  <div class="space-y-3">
    <label
      v-for="opt in options"
      :key="String(opt.value)"
      :class="optionCardClass"
    >
      <input
        type="radio"
        :value="opt.value"
        :checked="modelValue === opt.value"
        :class="radioClass"
        @change="emit('update:modelValue', opt.value)"
      />
      <div class="flex-1">
        <div :class="optionTitleClass">{{ opt.label }}</div>
        <div
          v-if="opt.description"
          :class="optionDescriptionClass"
          :style="opt.style"
        >
          {{ opt.description }}
        </div>
      </div>
    </label>
  </div>
</template>
