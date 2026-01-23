<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  content: string
}>()

// Simple markdown to HTML converter
const renderedHtml = computed(() => {
  if (!props.content) return '<p class="text-gray-400">Nothing to preview</p>'
  
  let html = props.content
    // Escape HTML
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    // Bold
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    // Italic
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    // Inline code
    .replace(/`(.+?)`/g, '<code class="px-1 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-sm font-mono">$1</code>')
    // Links
    .replace(/\[(.+?)\]\((.+?)\)/g, '<a href="$2" class="text-primary underline" target="_blank">$1</a>')
    // Code blocks
    .replace(/```(\w+)?\n([\s\S]+?)```/g, '<pre class="p-3 bg-gray-900 text-gray-100 rounded-lg overflow-x-auto my-2"><code>$2</code></pre>')
    // Blockquotes
    .replace(/^> (.+)$/gm, '<blockquote class="border-l-4 border-gray-300 dark:border-gray-600 pl-3 text-gray-600 dark:text-gray-400 italic">$1</blockquote>')
    // Line breaks
    .replace(/\n/g, '<br>')
  
  return html
})
</script>

<template>
  <div 
    class="prose prose-sm dark:prose-invert max-w-none p-3 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700"
    v-html="renderedHtml"
  ></div>
</template>

<style scoped>
.prose :deep(strong) {
  font-weight: 700;
}
.prose :deep(em) {
  font-style: italic;
}
.prose :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}
</style>
