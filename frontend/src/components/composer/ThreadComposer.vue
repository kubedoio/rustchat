<template>
  <div class="thread-composer">
    <FormattingToolbar
      v-if="editor"
      :show-preview="false"
      @format="handleFormat"
      @toggle-preview="() => {}"
    />
    <div class="composer-input-wrapper">
      <EditorContent :editor="editor" class="composer-editor" />
      <button
        class="send-btn"
        :disabled="!canSend || isSending"
        @click="send"
      >
        {{ isSending ? 'Sending...' : 'Send' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useEditor, EditorContent } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import Placeholder from '@tiptap/extension-placeholder'
import FormattingToolbar from './FormattingToolbar.vue'

const props = defineProps<{
  modelValue: string
  isSending: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  send: []
}>()

const editor = useEditor({
  content: props.modelValue,
  extensions: [
    StarterKit,
    Placeholder.configure({ placeholder: 'Reply in thread...' }),
  ],
  onUpdate: ({ editor }) => {
    emit('update:modelValue', editor.getHTML())
  },
})

const canSend = computed(() => {
  const html = editor.value?.getHTML() || ''
  const textOnly = html.replace(/<[^>]*>/g, '').trim()
  return textOnly.length > 0 && !props.isSending
})

function send() {
  if (!canSend.value) return
  emit('send')
  editor.value?.commands.clearContent()
}

function handleFormat(type: string) {
  if (!editor.value) return

  switch (type) {
    case 'bold':
      editor.value.chain().focus().toggleBold().run()
      break
    case 'italic':
      editor.value.chain().focus().toggleItalic().run()
      break
    case 'strike':
      editor.value.chain().focus().toggleStrike().run()
      break
    case 'heading':
      editor.value.chain().focus().toggleHeading({ level: 2 }).run()
      break
    case 'code':
      editor.value.chain().focus().toggleCode().run()
      break
    case 'codeblock':
      editor.value.chain().focus().toggleCodeBlock().run()
      break
    case 'bullet':
      editor.value.chain().focus().toggleBulletList().run()
      break
    case 'numbered':
      editor.value.chain().focus().toggleOrderedList().run()
      break
    case 'quote':
      editor.value.chain().focus().toggleBlockquote().run()
      break
    case 'link':
      // Link handling would need a dialog for URL input
      break
  }
}
</script>

<style scoped>
.thread-composer {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-1);
  border-radius: var(--radius-2);
  background: var(--bg-surface-1);
}

.composer-input-wrapper {
  display: flex;
  align-items: flex-end;
  gap: 0.5rem;
  padding: 0.75rem;
}

.composer-editor {
  flex: 1;
  min-height: 60px;
  max-height: 200px;
  overflow-y: auto;
}

.composer-editor :deep(.ProseMirror) {
  outline: none;
  min-height: 60px;
  padding: 0.5rem;
}

.composer-editor :deep(.ProseMirror p.is-editor-empty:first-child::before) {
  content: attr(data-placeholder);
  float: left;
  color: var(--text-3);
  pointer-events: none;
  height: 0;
}

.send-btn {
  padding: 0.5rem 1rem;
  background: var(--primary);
  color: white;
  border: none;
  border-radius: var(--radius-1);
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.2s;
}

.send-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.send-btn:not(:disabled):hover {
  background: var(--primary-hover);
}
</style>
