<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { Bold, Italic, Strikethrough, Heading, Code, Link2, List, ListOrdered, Quote, Eye, EyeOff, HelpCircle } from 'lucide-vue-next'

const emit = defineEmits<{
  (e: 'format', type: string): void
  (e: 'togglePreview'): void
}>()

defineProps<{
  showPreview: boolean
}>()

const formatActions = [
  { icon: Bold, type: 'bold', title: 'Bold (Ctrl+B)', label: 'Bold' },
  { icon: Italic, type: 'italic', title: 'Italic (Ctrl+I)', label: 'Italic' },
  { icon: Strikethrough, type: 'strike', title: 'Strikethrough (Ctrl+Shift+X)', label: 'Strikethrough' },
  { icon: Heading, type: 'heading', title: 'Heading', label: 'Heading' },
  { icon: Code, type: 'code', title: 'Inline code', label: 'Inline code' },
  { icon: Code, type: 'codeblock', title: 'Code block', label: 'Code block' },
  { icon: Link2, type: 'link', title: 'Link', label: 'Link' },
  { icon: Quote, type: 'quote', title: 'Quote', label: 'Quote' },
  { icon: List, type: 'bullet', title: 'Bullet list', label: 'Bullet list' },
  { icon: ListOrdered, type: 'numbered', title: 'Numbered list', label: 'Numbered list' },
]

const showHelp = ref(false)
const rootRef = ref<HTMLElement | null>(null)

function handleDocumentClick(event: MouseEvent) {
  const target = event.target as Node | null
  if (!target) return
  if (rootRef.value?.contains(target)) return
  showHelp.value = false
}

onMounted(() => {
  document.addEventListener('mousedown', handleDocumentClick)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', handleDocumentClick)
})
</script>

<template>
  <div ref="rootRef" class="relative flex items-center gap-1 overflow-x-auto border-b border-border-1 bg-bg-surface-2/50 px-2 py-1 whitespace-nowrap">
    <!-- Formatting buttons -->
    <button
      v-for="action in formatActions"
      :key="action.type"
      @click="$emit('format', action.type)"
      :title="action.title"
      :aria-label="action.label"
      class="flex h-8 w-8 shrink-0 items-center justify-center rounded-r-1 text-text-3 transition-standard hover:bg-bg-surface-1 hover:text-text-1 focus-ring"
    >
      <component :is="action.icon" class="w-3.5 h-3.5" />
    </button>
    
    <!-- Divider -->
    <div class="mx-1 h-5 w-px shrink-0 bg-border-1"></div>
    
    <!-- Preview toggle -->
    <button
      @click="$emit('togglePreview')"
      :title="showPreview ? 'Hide preview' : 'Show preview'"
      aria-label="Toggle markdown preview"
      class="flex h-8 w-8 shrink-0 items-center justify-center rounded-r-1 transition-standard focus-ring"
      :class="showPreview 
        ? 'bg-brand/10 text-brand'
        : 'text-text-3 hover:bg-bg-surface-1 hover:text-text-1'"
    >
      <component :is="showPreview ? EyeOff : Eye" class="w-3.5 h-3.5" />
    </button>

    <button
      @click="showHelp = !showHelp"
      title="Formatting help"
      aria-label="Formatting help"
      class="flex h-8 w-8 shrink-0 items-center justify-center rounded-r-1 text-text-3 transition-standard hover:bg-bg-surface-1 hover:text-text-1 focus-ring"
      :class="showHelp ? 'bg-bg-surface-1 text-text-1' : ''"
    >
      <HelpCircle class="w-3.5 h-3.5" />
    </button>

    <div
      v-if="showHelp"
      class="absolute right-1 top-full mt-2 z-[130] w-[22rem] rounded-r-2 border border-border-1 bg-bg-surface-1 p-3 shadow-2xl"
    >
      <p class="text-[11px] font-semibold text-text-1">Formatting shortcuts</p>
      <div class="mt-2 space-y-1 text-[11px] text-text-2">
        <p><kbd class="rounded bg-bg-surface-2 px-1">Ctrl/Cmd+B</kbd> Bold</p>
        <p><kbd class="rounded bg-bg-surface-2 px-1">Ctrl/Cmd+I</kbd> Italic</p>
        <p><kbd class="rounded bg-bg-surface-2 px-1">Toolbar</kbd> Insert link</p>
        <p><kbd class="rounded bg-bg-surface-2 px-1">Ctrl/Cmd+Shift+X</kbd> Strikethrough</p>
        <p><kbd class="rounded bg-bg-surface-2 px-1">Ctrl/Cmd+Shift+7</kbd> Numbered list</p>
        <p><kbd class="rounded bg-bg-surface-2 px-1">Ctrl/Cmd+Shift+8</kbd> Bulleted list</p>
      </div>
      <p class="mt-2 text-[10px] text-text-3">Use <code>:emoji:</code>, <code>@mention</code>, <code>~channel</code>, and <code>^k</code> (or Ctrl/Cmd+K) for command menu.</p>
    </div>
  </div>
</template>
