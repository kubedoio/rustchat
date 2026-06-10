<script setup lang="ts">
import { ref, computed } from 'vue'
import { X, Database, Type, Settings, Hash } from 'lucide-vue-next'
import { useKnowledgeBaseStore } from '../../features/admin/stores/knowledgeBaseStore'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'created'): void
}>()

const kbStore = useKnowledgeBaseStore()

const submitting = ref(false)
const error = ref('')

const form = ref({
  name: '',
  description: '',
  embedding_model: 'text-embedding-3-small',
  embedding_dimensions: 1536,
  chunk_size: 512,
  chunk_overlap: 50,
})

const isValid = computed(() => {
  return form.value.name.length >= 1 && form.value.embedding_model.length > 0
})

async function submit() {
  if (!isValid.value) return

  submitting.value = true
  error.value = ''

  try {
    await kbStore.createKnowledgeBase({
      name: form.value.name,
      description: form.value.description || undefined,
      embedding_model: form.value.embedding_model,
      embedding_dimensions: form.value.embedding_dimensions,
      chunk_size: form.value.chunk_size,
      chunk_overlap: form.value.chunk_overlap,
    })

    // Reset form
    form.value = {
      name: '',
      description: '',
      embedding_model: 'text-embedding-3-small',
      embedding_dimensions: 1536,
      chunk_size: 512,
      chunk_overlap: 50,
    }

    emit('created')
    emit('close')
  } catch (e: unknown) {
    error.value = getApiErrorMessage(e) || 'Failed to create knowledge base'
  } finally {
    submitting.value = false
  }
}

function close() {
  error.value = ''
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="close"></div>

      <!-- Modal -->
      <div
        class="relative bg-bg-surface-1 rounded-xl shadow-2xl border border-border-1 w-full max-w-lg overflow-hidden flex flex-col max-h-[90vh]"
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-5 py-4 border-b border-border-1 shrink-0">
          <div class="flex items-center gap-2.5">
            <div class="rounded-lg bg-brand/10 p-1.5">
              <Database class="w-4 h-4 text-brand" />
            </div>
            <h2 class="text-sm font-semibold text-text-1">Create Knowledge Base</h2>
          </div>
          <button class="p-1.5 hover:bg-bg-surface-2 rounded-lg transition-colors" @click="close">
            <X class="w-4 h-4 text-text-4" />
          </button>
        </div>

        <!-- Form -->
        <form class="flex-1 overflow-y-auto p-5 space-y-4" @submit.prevent="submit">
          <!-- Error -->
          <div
            v-if="error"
            class="p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs"
          >
            {{ error }}
          </div>

          <div>
            <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
              <Type class="w-3.5 h-3.5 text-text-4" />
              Name *
            </label>
            <input
              v-model="form.name"
              type="text"
              required
              class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
              placeholder="My Knowledge Base"
            />
          </div>

          <div>
            <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
              <Settings class="w-3.5 h-3.5 text-text-4" />
              Description
            </label>
            <textarea
              v-model="form.description"
              rows="3"
              class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all resize-none"
              placeholder="Brief description of this knowledge base..."
            />
          </div>

          <div>
            <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
              <Settings class="w-3.5 h-3.5 text-text-4" />
              Embedding Model
            </label>
            <input
              v-model="form.embedding_model"
              type="text"
              required
              class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
              placeholder="text-embedding-3-small"
            />
          </div>

          <div class="grid grid-cols-3 gap-4">
            <div>
              <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                <Hash class="w-3.5 h-3.5 text-text-4" />
                Dimensions
              </label>
              <input
                v-model.number="form.embedding_dimensions"
                type="number"
                min="1"
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
              />
            </div>
            <div>
              <label class="block text-xs font-medium text-text-2 mb-1.5">Chunk Size</label>
              <input
                v-model.number="form.chunk_size"
                type="number"
                min="1"
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
              />
            </div>
            <div>
              <label class="block text-xs font-medium text-text-2 mb-1.5">Chunk Overlap</label>
              <input
                v-model.number="form.chunk_overlap"
                type="number"
                min="0"
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
              />
            </div>
          </div>

          <!-- Actions -->
          <div class="flex justify-end gap-2 pt-4 border-t border-border-1">
            <button
              type="button"
              class="px-3 py-2 text-text-2 hover:bg-bg-surface-2 rounded-lg text-xs font-medium transition-colors"
              @click="close"
            >
              Cancel
            </button>
            <button
              type="submit"
              :disabled="!isValid || submitting"
              class="px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-xs font-medium transition-colors"
            >
              {{ submitting ? 'Creating...' : 'Create Knowledge Base' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
