<script setup lang="ts">
import { useSlots } from 'vue'
import { ChevronRight } from 'lucide-vue-next'

defineProps<{
  label: string
  value: string
  description?: string
}>()

defineEmits<{
  click: []
}>()

const slots = useSlots()
</script>

<template>
  <button
    type="button"
    @click="$emit('click')"
    class="group flex w-full items-center justify-between rounded-r-1 px-4 py-3 text-left transition-standard hover:bg-bg-surface-2"
  >
    <div class="flex items-start flex-1 min-w-0">
      <div v-if="slots.icon" class="mr-3 mt-0.5">
        <slot name="icon" />
      </div>
      <div class="min-w-0">
        <div class="text-sm font-medium text-text-1">
          {{ label }}
        </div>
        <div v-if="description" class="mt-0.5 text-xs text-text-3">
          {{ description }}
        </div>
      </div>
    </div>
    <div class="flex items-center gap-2 ml-4">
      <span class="max-w-[200px] truncate text-sm text-text-3">
        {{ value }}
      </span>
      <slot name="extra" />
      <ChevronRight class="h-4 w-4 shrink-0 text-text-3 transition-standard group-hover:text-text-2" />
    </div>
  </button>
</template>
