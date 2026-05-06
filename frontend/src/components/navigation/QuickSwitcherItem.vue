<template>
  <div
    class="flex items-center gap-3 px-4 py-2.5 cursor-pointer transition-colors"
    :class="selected ? 'bg-blue-50' : 'hover:bg-gray-50'"
    @click="$emit('click')"
    @mouseenter="$emit('mouseenter')"
  >
    <div class="flex-shrink-0 w-8 h-8 flex items-center justify-center rounded-md bg-gray-100">
      <component :is="iconComponent" class="w-4 h-4 text-gray-600" />
    </div>
    <div class="flex-1 min-w-0">
      <p class="text-sm font-medium truncate">{{ item.name }}</p>
      <p v-if="item.subtitle" class="text-xs text-gray-400 truncate">{{ item.subtitle }}</p>
    </div>
    <kbd v-if="selected" class="flex-shrink-0 text-xs text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded font-mono">↵</kbd>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Hash, Lock, User, MessageSquare, Users } from 'lucide-vue-next'
import type { QuickSwitcherItem } from '../../composables/useQuickSwitcher'

const props = defineProps<{
  item: QuickSwitcherItem
  selected: boolean
}>()

defineEmits<{
  click: []
  mouseenter: []
}>()

const iconMap: Record<string, unknown> = { Hash, Lock, User, MessageSquare, Users }
const iconComponent = computed(() => iconMap[props.item.icon] ?? Hash)
</script>
