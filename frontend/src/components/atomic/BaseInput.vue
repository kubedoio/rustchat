<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  modelValue: string
  label?: string
  id?: string
  type?: string
  placeholder?: string
  required?: boolean
  error?: string
  disabled?: boolean
}>()

defineEmits(['update:modelValue'])

const inputId = computed(() => props.id || `input-${Math.random().toString(36).slice(2, 9)}`)
</script>

<template>
  <div class="space-y-1">
    <label v-if="label" :for="inputId" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
      {{ label }}
    </label>
    <div class="relative rounded-md shadow-sm">
      <input
        :id="inputId"
        :type="type || 'text'"
        :value="modelValue"
        @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        :placeholder="placeholder"
        :required="required"
        :disabled="disabled"
        class="block w-full px-3 py-2 rounded-lg border border-gray-300 focus:border-primary focus:ring-2 focus:ring-primary/20 sm:text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white transition-colors"
        :class="[
          error ? 'border-red-300 text-red-900 placeholder-red-300 focus:border-red-500 focus:ring-red-500' : '',
          disabled ? 'bg-gray-100 dark:bg-gray-800 cursor-not-allowed' : ''
        ]"
      />
    </div>
    <p v-if="error" class="mt-2 text-sm text-red-600" :id="`${inputId}-error`">{{ error }}</p>
  </div>
</template>
