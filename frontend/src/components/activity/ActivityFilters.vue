<template>
  <div class="flex gap-1.5 p-3 border-b border-gray-200 overflow-x-auto">
    <button
      v-for="filter in filters"
      :key="filter.value ?? 'all'"
      class="px-3 py-1 text-xs font-medium rounded-full whitespace-nowrap transition-colors"
      :class="[
        modelValue === filter.value
          ? 'bg-gray-900 text-white'
          : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
      ]"
      @click="$emit('update:modelValue', filter.value)"
    >
      {{ filter.label }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { ActivityType } from '../../features/activity/types'

interface FilterOption {
  label: string
  value: ActivityType | null
}

const filters: FilterOption[] = [
  { label: 'All', value: null },
  { label: 'Mentions', value: ActivityType.MENTION },
  { label: 'Replies', value: ActivityType.REPLY },
  { label: 'Threads', value: ActivityType.THREAD_REPLY },
  { label: 'Reactions', value: ActivityType.REACTION },
  { label: 'DMs', value: ActivityType.DM }
]

defineProps<{
  modelValue: ActivityType | null
}>()

defineEmits<{
  'update:modelValue': [value: ActivityType | null]
}>()
</script>
