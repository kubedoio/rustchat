<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import {
  Database,
  Plus,
  Search,
  Edit2,
  Trash2,
  AlertTriangle,
  X,
  CheckCircle,
  XCircle,
  FileText,
} from 'lucide-vue-next'
import { useKnowledgeBaseStore } from '../../features/admin/stores/knowledgeBaseStore'
import { type KnowledgeBaseSummary } from '../../api/knowledgeBases'
import CreateKnowledgeBaseModal from '../../components/modals/CreateKnowledgeBaseModal.vue'
import EditKnowledgeBaseModal from '../../components/modals/EditKnowledgeBaseModal.vue'
import BaseModal from '../../components/ui/BaseModal.vue'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

const kbStore = useKnowledgeBaseStore()
const searchQuery = ref('')
const showCreateModal = ref(false)
const showEditModal = ref(false)
const editingKb = ref<KnowledgeBaseSummary | null>(null)
const showDeleteModal = ref(false)
const deletingKb = ref<KnowledgeBaseSummary | null>(null)
const deleteSubmitting = ref(false)
const deleteError = ref('')

let searchTimeout: ReturnType<typeof setTimeout>

onMounted(() => {
  kbStore.fetchKnowledgeBases()
})

function fetchKnowledgeBases() {
  kbStore.fetchKnowledgeBases()
}

watch(searchQuery, () => {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    kbStore.fetchKnowledgeBases()
  }, 300)
})

const filteredKnowledgeBases = computed(() => {
  if (!searchQuery.value) return kbStore.knowledgeBases
  const q = searchQuery.value.toLowerCase()
  return kbStore.knowledgeBases.filter(
    kb =>
      kb.name.toLowerCase().includes(q) ||
      (kb.description?.toLowerCase() ?? '').includes(q) ||
      kb.embedding_model.toLowerCase().includes(q)
  )
})

function handleEdit(kb: KnowledgeBaseSummary) {
  editingKb.value = kb
  showEditModal.value = true
}

function closeEditModal() {
  showEditModal.value = false
  editingKb.value = null
}

function openDeleteModal(kb: KnowledgeBaseSummary) {
  deletingKb.value = kb
  deleteError.value = ''
  deleteSubmitting.value = false
  showDeleteModal.value = true
}

function closeDeleteModal() {
  showDeleteModal.value = false
  deletingKb.value = null
  deleteError.value = ''
  deleteSubmitting.value = false
}

async function confirmDeleteKnowledgeBase() {
  if (!deletingKb.value) return
  deleteSubmitting.value = true
  deleteError.value = ''
  try {
    await kbStore.deleteKnowledgeBase(deletingKb.value.id)
    closeDeleteModal()
  } catch (e: unknown) {
    deleteError.value = getApiErrorMessage(e) || 'Failed to delete knowledge base'
  } finally {
    deleteSubmitting.value = false
  }
}
</script>

<template>
  <div class="space-y-5">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-lg font-semibold text-text-1">Knowledge Bases</h1>
        <p class="text-text-3 text-xs mt-0.5">
          Manage knowledge bases for RAG and document retrieval
        </p>
      </div>
      <button
        class="flex items-center gap-1.5 px-3 py-2 bg-brand hover:bg-brand/90 text-white rounded-lg text-xs font-medium transition-colors"
        @click="showCreateModal = true"
      >
        <Plus class="w-3.5 h-3.5" />
        Create Knowledge Base
      </button>
    </div>

    <!-- Filters -->
    <div class="flex items-center gap-3">
      <div class="relative flex-1 max-w-sm">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-4" />
        <input
          v-model="searchQuery"
          type="text"
          placeholder="Search by name or model..."
          class="w-full pl-9 pr-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
        />
      </div>
    </div>

    <!-- Knowledge Bases Table -->
    <div class="bg-bg-surface-1 rounded-xl border border-border-1 overflow-hidden shadow-sm">
      <table class="min-w-full divide-y divide-border-1">
        <thead class="bg-bg-surface-2">
          <tr>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Name
            </th>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Description
            </th>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Embedding Model
            </th>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Documents
            </th>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Status
            </th>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Created
            </th>
            <th
              class="px-4 py-3 text-right text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Actions
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border-1">
          <tr
            v-for="kb in filteredKnowledgeBases"
            :key="kb.id"
            class="hover:bg-bg-surface-2/50 transition-colors"
          >
            <td class="px-4 py-3 whitespace-nowrap">
              <div class="flex items-center gap-3">
                <div
                  class="w-8 h-8 rounded-full bg-brand flex items-center justify-center text-white text-xs font-bold"
                >
                  <Database class="w-4 h-4" />
                </div>
                <div class="min-w-0">
                  <div class="text-xs font-medium text-text-1 truncate">
                    {{ kb.name }}
                  </div>
                </div>
              </div>
            </td>
            <td class="px-4 py-3 whitespace-nowrap text-xs text-text-2 max-w-xs truncate">
              {{ kb.description || '-' }}
            </td>
            <td class="px-4 py-3 whitespace-nowrap text-xs text-text-3">
              {{ kb.embedding_model }}
            </td>
            <td class="px-4 py-3 whitespace-nowrap text-xs text-text-3">
              <div class="flex items-center gap-1">
                <FileText class="w-3 h-3" />
                {{ kb.document_count }}
              </div>
            </td>
            <td class="px-4 py-3 whitespace-nowrap">
              <span
                v-if="kb.is_active"
                class="inline-flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded-full bg-success/10 text-success border border-success/20"
              >
                <CheckCircle class="w-3 h-3" /> Active
              </span>
              <span
                v-else
                class="inline-flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded-full bg-text-4/10 text-text-3 border border-text-4/20"
              >
                <XCircle class="w-3 h-3" /> Inactive
              </span>
            </td>
            <td class="px-4 py-3 whitespace-nowrap text-xs text-text-3">
              {{ new Date(kb.created_at).toLocaleDateString() }}
            </td>
            <td class="px-4 py-3 whitespace-nowrap text-right text-xs font-medium">
              <button
                class="text-brand hover:text-brand/80 mr-2 p-1 hover:bg-brand/10 rounded transition-colors"
                title="Edit Knowledge Base"
                @click="handleEdit(kb)"
              >
                <Edit2 class="w-3.5 h-3.5" />
              </button>
              <button
                class="text-danger hover:text-danger/80 p-1 hover:bg-danger/10 rounded transition-colors"
                title="Delete Knowledge Base"
                @click="openDeleteModal(kb)"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </td>
          </tr>
          <tr v-if="filteredKnowledgeBases.length === 0 && !kbStore.loading">
            <td colspan="7" class="px-4 py-12 text-center text-text-3">
              <Database class="w-10 h-10 mx-auto mb-3 text-text-4" />
              <p class="text-xs">No knowledge bases found</p>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <CreateKnowledgeBaseModal
      :open="showCreateModal"
      @close="showCreateModal = false"
      @created="fetchKnowledgeBases"
    />

    <EditKnowledgeBaseModal
      :open="showEditModal"
      :knowledge-base="editingKb"
      @close="closeEditModal"
      @updated="fetchKnowledgeBases"
    />

    <!-- Delete Modal -->
    <BaseModal v-if="deletingKb" v-model="showDeleteModal" size="md" @close="closeDeleteModal">
      <template #header>
        <div class="flex items-start justify-between p-5 border-b border-border-1">
          <div class="flex items-start gap-3">
            <div class="rounded-lg bg-danger/10 p-2">
              <AlertTriangle class="w-4 h-4 text-danger" />
            </div>
            <div>
              <h3 class="text-sm font-semibold text-text-1">Delete Knowledge Base</h3>
              <p class="text-[10px] text-text-3 mt-0.5">
                This will permanently delete the knowledge base and all its documents.
              </p>
            </div>
          </div>
          <button
            class="text-text-4 hover:text-text-2 p-1 hover:bg-bg-surface-2 rounded-lg transition-colors"
            @click="closeDeleteModal"
          >
            <X class="w-4 h-4" />
          </button>
        </div>
      </template>

      <div class="p-5 space-y-4">
        <div class="text-xs text-text-2">
          <p>
            You are about to delete knowledge base
            <span class="font-semibold text-text-1">{{ deletingKb?.name }}</span
            >.
          </p>
          <p class="mt-1">This action cannot be undone.</p>
        </div>

        <div
          v-if="deleteError"
          class="rounded-lg border border-danger/20 bg-danger/10 px-3 py-2 text-xs text-danger"
        >
          {{ deleteError }}
        </div>
      </div>

      <template #footer>
        <div class="flex items-center justify-end gap-2">
          <button
            class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
            @click="closeDeleteModal"
          >
            Cancel
          </button>
          <button
            :disabled="deleteSubmitting"
            class="px-3 py-2 rounded-lg bg-danger hover:bg-danger/90 disabled:opacity-50 text-white text-xs font-medium transition-colors"
            @click="confirmDeleteKnowledgeBase"
          >
            {{ deleteSubmitting ? 'Deleting...' : 'Delete Knowledge Base' }}
          </button>
        </div>
      </template>
    </BaseModal>
  </div>
</template>
