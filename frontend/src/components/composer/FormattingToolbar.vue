<script setup lang="ts">
import { Bold, Italic, Code, Link2, List, ListOrdered, Quote, Eye, EyeOff } from 'lucide-vue-next'

const emit = defineEmits<{
  (e: 'format', type: string): void
  (e: 'togglePreview'): void
}>()

defineProps<{
  showPreview: boolean
}>()

const formatActions = [
  { icon: Bold, type: 'bold', title: 'Bold (Ctrl+B)', wrapper: ['**', '**'] },
  { icon: Italic, type: 'italic', title: 'Italic (Ctrl+I)', wrapper: ['*', '*'] },
  { icon: Code, type: 'code', title: 'Inline code', wrapper: ['`', '`'] },
  { icon: Link2, type: 'link', title: 'Link', wrapper: ['[', '](url)'] },
  { icon: Quote, type: 'quote', title: 'Quote', prefix: '> ' },
  { icon: List, type: 'bullet', title: 'Bullet list', prefix: '- ' },
  { icon: ListOrdered, type: 'numbered', title: 'Numbered list', prefix: '1. ' },
]
</script>

<template>
  <div class="flex items-center space-x-0.5 px-1 py-1 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
    <!-- Formatting buttons -->
    <button
      v-for="action in formatActions"
      :key="action.type"
      @click="$emit('format', action.type)"
      :title="action.title"
      class="p-1.5 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors"
    >
      <component :is="action.icon" class="w-4 h-4" />
    </button>
    
    <!-- Divider -->
    <div class="w-px h-5 bg-gray-300 dark:bg-gray-600 mx-1"></div>
    
    <!-- Preview toggle -->
    <button
      @click="$emit('togglePreview')"
      :title="showPreview ? 'Hide preview' : 'Show preview'"
      class="p-1.5 rounded transition-colors"
      :class="showPreview 
        ? 'text-primary bg-primary/10' 
        : 'text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'"
    >
      <component :is="showPreview ? EyeOff : Eye" class="w-4 h-4" />
    </button>
  </div>
</template>
