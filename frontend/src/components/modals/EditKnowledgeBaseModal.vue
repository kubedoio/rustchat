<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  X,
  Database,
  Settings,
  FileText,
  RefreshCw,
  Trash2,
  Upload,
  CheckCircle,
  XCircle,
  Loader2,
  Plus,
} from 'lucide-vue-next'
import { useKnowledgeBaseStore } from '../../features/admin/stores/knowledgeBaseStore'
import type { KnowledgeBaseSummary } from '../../api/knowledgeBases'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

const props = defineProps<{
  open: boolean
  knowledgeBase: KnowledgeBaseSummary | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated'): void
}>()

const kbStore = useKnowledgeBaseStore()

const activeTab = ref('settings')
const submitting = ref(false)
const error = ref('')

const fileInput = ref<HTMLInputElement | null>(null)
const uploadLoading = ref(false)
const uploadError = ref('')

const documentsLoading = ref(false)
const syncSourcesLoading = ref(false)

const syncSourceForm = ref({
  source_type: 'web' as 'github' | 'confluence' | 'notion' | 'web' | 'api',
  config: '{}',
  sync_interval_minutes: 60,
})
const syncSourceSubmitting = ref(false)
const syncSourceError = ref('')

const tabs = [
  { id: 'settings', label: 'Settings', icon: Settings },
  { id: 'documents', label: 'Documents', icon: FileText },
  { id: 'sync-sources', label: 'Sync Sources', icon: RefreshCw },
]

const form = ref({
  name: '',
  description: '',
  embedding_model: 'text-embedding-3-small',
  embedding_dimensions: 1536,
  chunk_size: 512,
  chunk_overlap: 50,
  is_active: true,
})

watch(
  () => props.knowledgeBase,
  async newKb => {
    if (newKb) {
      activeTab.value = 'settings'
      error.value = ''
      uploadError.value = ''
      syncSourceError.value = ''

      // Populate form from summary
      form.value = {
        name: newKb.name,
        description: newKb.description || '',
        embedding_model: newKb.embedding_model,
        embedding_dimensions: newKb.embedding_dimensions,
        chunk_size: newKb.chunk_size,
        chunk_overlap: newKb.chunk_overlap,
        is_active: newKb.is_active,
      }

      // Fetch full detail and related data
      documentsLoading.value = true
      syncSourcesLoading.value = true
      try {
        await Promise.all([
          kbStore.fetchKnowledgeBase(newKb.id),
          kbStore.fetchDocuments(newKb.id),
          kbStore.fetchSyncSources(),
        ])
        if (kbStore.currentKnowledgeBase) {
          const detail = kbStore.currentKnowledgeBase
          form.value = {
            name: detail.name,
            description: detail.description || '',
            embedding_model: detail.embedding_model,
            embedding_dimensions: detail.embedding_dimensions,
            chunk_size: detail.chunk_size,
            chunk_overlap: detail.chunk_overlap,
            is_active: detail.is_active,
          }
        }
      } catch {
        // fallback to summary data already set
      } finally {
        documentsLoading.value = false
        syncSourcesLoading.value = false
      }
    }
  },
  { immediate: true }
)

async function submit() {
  if (!props.knowledgeBase) return

  submitting.value = true
  error.value = ''

  try {
    await kbStore.updateKnowledgeBase(props.knowledgeBase.id, {
      name: form.value.name,
      description: form.value.description || undefined,
      embedding_model: form.value.embedding_model,
      embedding_dimensions: form.value.embedding_dimensions,
      chunk_size: form.value.chunk_size,
      chunk_overlap: form.value.chunk_overlap,
      is_active: form.value.is_active,
    })

    emit('updated')
    emit('close')
  } catch (e: unknown) {
    error.value = getApiErrorMessage(e) || 'Failed to update knowledge base'
  } finally {
    submitting.value = false
  }
}

function close() {
  error.value = ''
  uploadError.value = ''
  syncSourceError.value = ''
  emit('close')
}

function triggerFileUpload() {
  fileInput.value?.click()
}

async function handleFileUpload(event: Event) {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file || !props.knowledgeBase) return

  uploadLoading.value = true
  uploadError.value = ''
  try {
    await kbStore.uploadDocument(props.knowledgeBase.id, file)
    target.value = ''
  } catch (e: unknown) {
    uploadError.value = getApiErrorMessage(e) || 'Failed to upload document'
  } finally {
    uploadLoading.value = false
  }
}

async function handleDeleteDocument(docId: string) {
  if (!props.knowledgeBase) return
  if (!confirm('Are you sure you want to delete this document?')) return
  try {
    await kbStore.deleteDocument(props.knowledgeBase.id, docId)
  } catch (e: unknown) {
    uploadError.value = getApiErrorMessage(e) || 'Failed to delete document'
  }
}

async function handleCreateSyncSource() {
  if (!props.knowledgeBase) return
  syncSourceSubmitting.value = true
  syncSourceError.value = ''
  try {
    let config: Record<string, any>
    try {
      config = JSON.parse(syncSourceForm.value.config)
    } catch {
      syncSourceError.value = 'Invalid JSON in config'
      syncSourceSubmitting.value = false
      return
    }
    await kbStore.createSyncSource({
      source_type: syncSourceForm.value.source_type,
      config,
      sync_interval_minutes: syncSourceForm.value.sync_interval_minutes,
    })
    syncSourceForm.value = {
      source_type: 'web',
      config: '{}',
      sync_interval_minutes: 60,
    }
  } catch (e: unknown) {
    syncSourceError.value = getApiErrorMessage(e) || 'Failed to create sync source'
  } finally {
    syncSourceSubmitting.value = false
  }
}

async function handleDeleteSyncSource(sourceId: string) {
  if (!props.knowledgeBase) return
  if (!confirm('Are you sure you want to delete this sync source?')) return
  try {
    await kbStore.deleteSyncSource(sourceId)
  } catch (e: unknown) {
    syncSourceError.value = getApiErrorMessage(e) || 'Failed to delete sync source'
  }
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

function formatDate(date: string | null): string {
  if (!date) return 'Never'
  return new Date(date).toLocaleDateString()
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="close"></div>

      <!-- Modal -->
      <div
        class="relative bg-bg-surface-1 rounded-xl shadow-2xl border border-border-1 w-full max-w-2xl overflow-hidden flex flex-col max-h-[90vh]"
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-5 py-4 border-b border-border-1 shrink-0">
          <div class="flex items-center gap-2.5">
            <div class="rounded-lg bg-brand/10 p-1.5">
              <Database class="w-4 h-4 text-brand" />
            </div>
            <div>
              <h2 class="text-sm font-semibold text-text-1">Edit Knowledge Base</h2>
              <p v-if="knowledgeBase" class="text-[10px] text-text-3">{{ knowledgeBase.name }}</p>
            </div>
          </div>
          <button class="p-1.5 hover:bg-bg-surface-2 rounded-lg transition-colors" @click="close">
            <X class="w-4 h-4 text-text-4" />
          </button>
        </div>

        <!-- Tabs -->
        <div class="flex border-b border-border-1 bg-bg-surface-2 shrink-0">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            class="flex items-center gap-1.5 px-4 py-2.5 text-xs font-medium transition-colors border-b-2"
            :class="
              activeTab === tab.id
                ? 'border-brand text-brand bg-brand/5'
                : 'border-transparent text-text-3 hover:text-text-1 hover:bg-bg-surface-1'
            "
            @click="activeTab = tab.id"
          >
            <component :is="tab.icon" class="w-3.5 h-3.5" />
            {{ tab.label }}
          </button>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-5 space-y-4">
          <!-- Error -->
          <div
            v-if="error"
            class="p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs"
          >
            {{ error }}
          </div>

          <!-- Settings Tab -->
          <div v-if="activeTab === 'settings'" class="space-y-4">
            <div>
              <label class="block text-xs font-medium text-text-2 mb-1.5">Name *</label>
              <input
                v-model="form.name"
                type="text"
                required
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
              />
            </div>

            <div>
              <label class="block text-xs font-medium text-text-2 mb-1.5">Description</label>
              <textarea
                v-model="form.description"
                rows="3"
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all resize-none"
              />
            </div>

            <div>
              <label class="block text-xs font-medium text-text-2 mb-1.5">Embedding Model</label>
              <input
                v-model="form.embedding_model"
                type="text"
                required
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
              />
            </div>

            <div class="grid grid-cols-3 gap-4">
              <div>
                <label class="block text-xs font-medium text-text-2 mb-1.5">Dimensions</label>
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

            <label
              class="flex items-center gap-2.5 p-2.5 rounded-lg border border-border-1 hover:bg-bg-surface-2 cursor-pointer transition-colors"
            >
              <input v-model="form.is_active" type="checkbox" class="w-4 h-4 text-brand rounded" />
              <div>
                <div class="text-xs font-medium text-text-1">Active</div>
                <div class="text-[10px] text-text-3">
                  Knowledge base is available for RAG queries when active
                </div>
              </div>
            </label>
          </div>

          <!-- Documents Tab -->
          <div v-if="activeTab === 'documents'" class="space-y-4">
            <div class="flex items-center justify-between">
              <h3 class="text-xs font-semibold text-text-1">Documents</h3>
              <button
                type="button"
                class="flex items-center gap-1.5 px-3 py-1.5 bg-brand hover:bg-brand/90 text-white rounded-lg text-xs font-medium transition-colors"
                :disabled="uploadLoading"
                @click="triggerFileUpload"
              >
                <Upload v-if="!uploadLoading" class="w-3.5 h-3.5" />
                <Loader2 v-else class="w-3.5 h-3.5 animate-spin" />
                {{ uploadLoading ? 'Uploading...' : 'Upload Document' }}
              </button>
              <input
                ref="fileInput"
                type="file"
                class="hidden"
                accept=".pdf,.txt,.md,.doc,.docx"
                @change="handleFileUpload"
              />
            </div>

            <div
              v-if="uploadError"
              class="p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs"
            >
              {{ uploadError }}
            </div>

            <div
              v-if="documentsLoading"
              class="text-xs text-text-3 py-4 text-center flex items-center justify-center gap-2"
            >
              <Loader2 class="w-3.5 h-3.5 animate-spin" />
              Loading documents...
            </div>
            <div
              v-else-if="kbStore.documents.length === 0"
              class="text-xs text-text-3 py-8 text-center"
            >
              <FileText class="w-8 h-8 mx-auto mb-2 text-text-4" />
              <p>No documents yet.</p>
              <p class="mt-1">Upload documents to add them to this knowledge base.</p>
            </div>
            <div v-else class="space-y-2">
              <div
                v-for="doc in kbStore.documents"
                :key="doc.id"
                class="flex items-center justify-between p-3 rounded-lg border border-border-1 hover:bg-bg-surface-2 transition-colors"
              >
                <div class="flex items-center gap-3 min-w-0">
                  <FileText class="w-4 h-4 text-text-3 shrink-0" />
                  <div class="min-w-0">
                    <div class="text-xs font-medium text-text-1 truncate">{{ doc.filename }}</div>
                    <div class="text-[10px] text-text-3">
                      {{ formatFileSize(doc.file_size) }} · {{ doc.chunk_count }} chunks ·
                      <span
                        :class="
                          doc.status === 'completed'
                            ? 'text-success'
                            : doc.status === 'failed'
                              ? 'text-danger'
                              : 'text-text-3'
                        "
                      >
                        {{ doc.status }}
                      </span>
                    </div>
                  </div>
                </div>
                <button
                  class="text-danger hover:text-danger/80 p-1 hover:bg-danger/10 rounded transition-colors shrink-0"
                  title="Delete Document"
                  @click="handleDeleteDocument(doc.id)"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>

          <!-- Sync Sources Tab -->
          <div v-if="activeTab === 'sync-sources'" class="space-y-4">
            <!-- Add Sync Source Form -->
            <div class="p-4 rounded-lg border border-border-1 bg-bg-surface-2 space-y-3">
              <h4 class="text-xs font-semibold text-text-1 flex items-center gap-1.5">
                <Plus class="w-3.5 h-3.5" />
                Add Sync Source
              </h4>

              <div
                v-if="syncSourceError"
                class="p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs"
              >
                {{ syncSourceError }}
              </div>

              <div class="grid grid-cols-2 gap-3">
                <div>
                  <label class="block text-xs font-medium text-text-2 mb-1.5">Source Type</label>
                  <select
                    v-model="syncSourceForm.source_type"
                    class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                  >
                    <option value="github">GitHub</option>
                    <option value="confluence">Confluence</option>
                    <option value="notion">Notion</option>
                    <option value="web">Web</option>
                    <option value="api">API</option>
                  </select>
                </div>
                <div>
                  <label class="block text-xs font-medium text-text-2 mb-1.5">
                    Sync Interval (min)
                  </label>
                  <input
                    v-model.number="syncSourceForm.sync_interval_minutes"
                    type="number"
                    min="5"
                    class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                  />
                </div>
              </div>

              <div>
                <label class="block text-xs font-medium text-text-2 mb-1.5">Config (JSON)</label>
                <textarea
                  v-model="syncSourceForm.config"
                  rows="3"
                  class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all resize-none font-mono"
                  placeholder='{ "url": "https://example.com/docs" }'
                />
              </div>

              <div class="flex justify-end">
                <button
                  type="button"
                  :disabled="syncSourceSubmitting"
                  class="px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 text-white rounded-lg text-xs font-medium transition-colors"
                  @click="handleCreateSyncSource"
                >
                  {{ syncSourceSubmitting ? 'Adding...' : 'Add Sync Source' }}
                </button>
              </div>
            </div>

            <!-- Sync Sources List -->
            <div
              v-if="syncSourcesLoading"
              class="text-xs text-text-3 py-4 text-center flex items-center justify-center gap-2"
            >
              <Loader2 class="w-3.5 h-3.5 animate-spin" />
              Loading sync sources...
            </div>
            <div
              v-else-if="kbStore.syncSources.length === 0"
              class="text-xs text-text-3 py-8 text-center"
            >
              <RefreshCw class="w-8 h-8 mx-auto mb-2 text-text-4" />
              <p>No sync sources configured.</p>
            </div>
            <div v-else class="space-y-2">
              <div
                v-for="source in kbStore.syncSources"
                :key="source.id"
                class="flex items-center justify-between p-3 rounded-lg border border-border-1 hover:bg-bg-surface-2 transition-colors"
              >
                <div class="flex items-center gap-3 min-w-0">
                  <RefreshCw class="w-4 h-4 text-text-3 shrink-0" />
                  <div class="min-w-0">
                    <div class="text-xs font-medium text-text-1 capitalize">
                      {{ source.source_type }}
                    </div>
                    <div class="text-[10px] text-text-3">
                      Every {{ source.sync_interval_minutes }} min · Last sync:
                      {{ formatDate(source.last_sync_at) }}
                      <span
                        v-if="source.is_active"
                        class="inline-flex items-center gap-0.5 ml-1 text-success"
                      >
                        <CheckCircle class="w-3 h-3" /> Active
                      </span>
                      <span v-else class="inline-flex items-center gap-0.5 ml-1 text-text-3">
                        <XCircle class="w-3 h-3" /> Inactive
                      </span>
                    </div>
                  </div>
                </div>
                <button
                  class="text-danger hover:text-danger/80 p-1 hover:bg-danger/10 rounded transition-colors shrink-0"
                  title="Delete Sync Source"
                  @click="handleDeleteSyncSource(source.id)"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer Actions -->
        <div
          v-if="activeTab === 'settings'"
          class="flex justify-end gap-2 px-5 py-4 border-t border-border-1 shrink-0"
        >
          <button
            type="button"
            class="px-3 py-2 text-text-2 hover:bg-bg-surface-2 rounded-lg text-xs font-medium transition-colors"
            @click="close"
          >
            Cancel
          </button>
          <button
            type="button"
            :disabled="submitting"
            class="px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-xs font-medium transition-colors"
            @click="submit"
          >
            {{ submitting ? 'Saving...' : 'Save Changes' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
