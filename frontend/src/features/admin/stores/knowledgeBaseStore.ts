import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import knowledgeBasesApi, {
  type KnowledgeBaseSummary,
  type KnowledgeBaseDetail,
  type KnowledgeBaseDocument,
  type SyncSource,
  type CreateKnowledgeBasePayload,
  type UpdateKnowledgeBasePayload,
  type CreateSyncSourcePayload,
} from '@/api/knowledgeBases'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

export const useKnowledgeBaseStore = defineStore('knowledgeBaseStore', () => {
  const knowledgeBases = ref<KnowledgeBaseSummary[]>([])
  const currentKnowledgeBase = ref<KnowledgeBaseDetail | null>(null)
  const documents = ref<KnowledgeBaseDocument[]>([])
  const syncSources = ref<SyncSource[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const activeKnowledgeBases = computed(() => knowledgeBases.value.filter(kb => kb.is_active))

  async function fetchKnowledgeBases() {
    loading.value = true
    error.value = null
    try {
      const response = await knowledgeBasesApi.list()
      knowledgeBases.value = response.data.knowledge_bases
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to load knowledge bases'
    } finally {
      loading.value = false
    }
  }

  async function fetchKnowledgeBase(id: string) {
    loading.value = true
    error.value = null
    try {
      const response = await knowledgeBasesApi.get(id)
      currentKnowledgeBase.value = response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to load knowledge base'
    } finally {
      loading.value = false
    }
  }

  async function createKnowledgeBase(data: CreateKnowledgeBasePayload) {
    loading.value = true
    error.value = null
    try {
      const response = await knowledgeBasesApi.create(data)
      knowledgeBases.value.push({
        ...response.data,
        document_count: 0,
      })
      return response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to create knowledge base'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateKnowledgeBase(id: string, data: UpdateKnowledgeBasePayload) {
    loading.value = true
    error.value = null
    try {
      const response = await knowledgeBasesApi.update(id, data)
      const idx = knowledgeBases.value.findIndex(kb => kb.id === id)
      if (idx !== -1) {
        knowledgeBases.value[idx] = { ...knowledgeBases.value[idx], ...response.data }
      }
      if (currentKnowledgeBase.value?.id === id) {
        currentKnowledgeBase.value = response.data
      }
      return response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to update knowledge base'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteKnowledgeBase(id: string) {
    loading.value = true
    error.value = null
    try {
      await knowledgeBasesApi.delete(id)
      knowledgeBases.value = knowledgeBases.value.filter(kb => kb.id !== id)
      if (currentKnowledgeBase.value?.id === id) {
        currentKnowledgeBase.value = null
      }
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to delete knowledge base'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchDocuments(id: string) {
    loading.value = true
    error.value = null
    try {
      const response = await knowledgeBasesApi.listDocuments(id)
      documents.value = response.data.documents
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to load documents'
    } finally {
      loading.value = false
    }
  }

  async function uploadDocument(id: string, file: File) {
    loading.value = true
    error.value = null
    try {
      const response = await knowledgeBasesApi.uploadDocument(id, file)
      documents.value.push(response.data)
      const idx = knowledgeBases.value.findIndex(kb => kb.id === id)
      if (idx !== -1) {
        knowledgeBases.value[idx].document_count += 1
      }
      return response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to upload document'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteDocument(kbId: string, docId: string) {
    loading.value = true
    error.value = null
    try {
      await knowledgeBasesApi.deleteDocument(kbId, docId)
      documents.value = documents.value.filter(d => d.id !== docId)
      const idx = knowledgeBases.value.findIndex(kb => kb.id === kbId)
      if (idx !== -1) {
        knowledgeBases.value[idx].document_count = Math.max(
          0,
          knowledgeBases.value[idx].document_count - 1
        )
      }
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to delete document'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchSyncSources(id: string) {
    loading.value = true
    error.value = null
    try {
      const response = await knowledgeBasesApi.listSyncSources(id)
      syncSources.value = response.data.sync_sources
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to load sync sources'
    } finally {
      loading.value = false
    }
  }

  async function createSyncSource(id: string, data: CreateSyncSourcePayload) {
    loading.value = true
    error.value = null
    try {
      const response = await knowledgeBasesApi.createSyncSource(id, data)
      syncSources.value.push(response.data)
      return response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to create sync source'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteSyncSource(kbId: string, sourceId: string) {
    loading.value = true
    error.value = null
    try {
      await knowledgeBasesApi.deleteSyncSource(kbId, sourceId)
      syncSources.value = syncSources.value.filter(s => s.id !== sourceId)
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to delete sync source'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    knowledgeBases,
    currentKnowledgeBase,
    documents,
    syncSources,
    loading,
    error,
    activeKnowledgeBases,
    fetchKnowledgeBases,
    fetchKnowledgeBase,
    createKnowledgeBase,
    updateKnowledgeBase,
    deleteKnowledgeBase,
    fetchDocuments,
    uploadDocument,
    deleteDocument,
    fetchSyncSources,
    createSyncSource,
    deleteSyncSource,
  }
})
