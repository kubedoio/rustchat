import { defineStore } from 'pinia'
import { ref } from 'vue'
import { knowledgeBasesApi, type KnowledgeBaseSummary } from '@/api/knowledgeBases'
import agentsApi, { type AgentKnowledgeBaseDetail } from '@/api/agents'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

export interface AssignKbPayload {
  knowledge_base_id: string
  top_k: number
  relevance_threshold?: number
}

export const useKnowledgeStore = defineStore('knowledge', () => {
  const knowledgeBases = ref<KnowledgeBaseSummary[]>([])
  const agentKbs = ref<AgentKnowledgeBaseDetail[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchKnowledgeBases() {
    loading.value = true
    error.value = null
    try {
      const response = await knowledgeBasesApi.list()
      const data = response.data as any
      if (Array.isArray(data)) {
        knowledgeBases.value = data
      } else if (data && Array.isArray(data.knowledge_bases)) {
        knowledgeBases.value = data.knowledge_bases
      } else {
        knowledgeBases.value = []
      }
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to load knowledge bases'
    } finally {
      loading.value = false
    }
  }

  async function fetchAgentKbs(agentId: string) {
    loading.value = true
    error.value = null
    try {
      const response = await agentsApi.listKnowledgeBases(agentId)
      const data = response.data as any
      if (Array.isArray(data)) {
        agentKbs.value = data
      } else if (data && Array.isArray(data.knowledge_bases)) {
        agentKbs.value = data.knowledge_bases
      } else {
        agentKbs.value = []
      }
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to load agent knowledge bases'
    } finally {
      loading.value = false
    }
  }

  async function assignKbToAgent(agentId: string, payload: AssignKbPayload) {
    loading.value = true
    error.value = null
    try {
      await agentsApi.assignKnowledgeBase(agentId, payload)
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to assign knowledge base'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function unassignKbFromAgent(agentId: string, kbId: string) {
    loading.value = true
    error.value = null
    try {
      await agentsApi.unassignKnowledgeBase(agentId, kbId)
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to unassign knowledge base'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    knowledgeBases,
    agentKbs,
    loading,
    error,
    fetchKnowledgeBases,
    fetchAgentKbs,
    assignKbToAgent,
    unassignKbFromAgent,
  }
})
