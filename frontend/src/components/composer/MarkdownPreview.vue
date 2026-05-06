<script setup lang="ts">
import { computed } from 'vue'
import { renderMarkdown } from '../../utils/markdown'

const props = defineProps<{
  content: string
}>()

const renderedHtml = computed(() => {
  return renderMarkdown(props.content)
})

function copyCode(event: MouseEvent) {
  const target = event.target as HTMLElement
  const codeBlock = target.closest('.code-block-wrapper')?.querySelector('code')
  if (codeBlock) {
    navigator.clipboard.writeText(codeBlock.textContent || '')
    target.textContent = 'Copied!'
    setTimeout(() => {
      target.textContent = 'Copy'
    }, 2000)
  }
}
</script>

<template>
  <div 
    class="markdown-preview prose prose-xs max-w-none p-3 bg-bg-surface-2/50 rounded-lg border border-border-1 overflow-auto max-h-80"
    v-html="renderedHtml"
    @click="copyCode"
  ></div>
</template>

<style scoped>
.markdown-preview :deep(pre) {
  position: relative;
  background: #1e1e1e;
  border-radius: 0.375rem;
  padding: 0.75rem;
  overflow-x: auto;
  margin: 0.5rem 0;
}

.markdown-preview :deep(pre code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.75rem;
  line-height: 1.5;
}

.markdown-preview :deep(code:not(pre code)) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  background: rgba(0, 0, 0, 0.1);
  padding: 0.125rem 0.375rem;
  border-radius: 0.25rem;
  font-size: 0.75rem;
}

.markdown-preview :deep(.code-block-wrapper) {
  position: relative;
}

.markdown-preview :deep(.copy-button) {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  padding: 0.25rem 0.5rem;
  background: rgba(255, 255, 255, 0.1);
  border: none;
  border-radius: 0.25rem;
  color: #fff;
  font-size: 0.625rem;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.2s;
}

.markdown-preview :deep(.code-block-wrapper:hover .copy-button) {
  opacity: 1;
}

.markdown-preview :deep(p) {
  margin: 0.5rem 0;
  font-size: 0.75rem;
  line-height: 1.5;
}

.markdown-preview :deep(strong) {
  font-weight: 700;
}

.markdown-preview :deep(em) {
  font-style: italic;
}

.markdown-preview :deep(blockquote) {
  border-left: 3px solid var(--border-color, #ccc);
  padding-left: 0.75rem;
  margin: 0.5rem 0;
  color: var(--text-muted, #666);
  font-size: 0.75rem;
}

.markdown-preview :deep(ul), .markdown-preview :deep(ol) {
  margin: 0.5rem 0;
  padding-left: 1.25rem;
  font-size: 0.75rem;
}

.markdown-preview :deep(a) {
  color: var(--brand-color, #0066cc);
  text-decoration: underline;
}

.markdown-preview :deep(hr) {
  margin: 1rem 0;
  border: none;
  border-top: 1px solid var(--border-color, #ccc);
}
</style>
