<script setup lang="ts">
interface SelectOption {
  value: string
  label: string
}

defineProps<{
  modelValue: string
  options: SelectOption[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const selectClass =
  'w-full rounded-lg border border-border-1 bg-bg-surface-1 px-3 py-2 text-sm text-text-1 outline-none transition-standard focus:border-brand focus:ring-2 focus:ring-brand/15'
</script>

<template>
  <div class="space-y-3">
    <select
      :value="modelValue"
      :class="selectClass"
      @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option v-for="opt in options" :key="opt.value" :value="opt.value">
        {{ opt.label }}
      </option>
    </select>
  </div>
</template>
