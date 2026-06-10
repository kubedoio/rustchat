import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import agentsApi, {
  type AgentSummary,
  type AgentDetail,
  type CreateAgentPayload,
  type UpdateAgentPayload,
  type AgentAnalyticsResponse,
} from '@/api/agents'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

export const useAgentStore = defineStore('agentStore', () => {
  const agents = ref<AgentSummary[]>([])
  const currentAgent = ref<AgentDetail | null>(null)
  const agentAnalytics = ref<AgentAnalyticsResponse | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const activeAgents = computed(() => agents.value.filter(a => a.is_active))

  async function fetchAgents() {
    loading.value = true
    error.value = null
    try {
      const response = await agentsApi.list()
      agents.value = response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to load agents'
    } finally {
      loading.value = false
    }
  }

  async function fetchAgent(id: string) {
    loading.value = true
    error.value = null
    try {
      const response = await agentsApi.get(id)
      currentAgent.value = response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to load agent'
    } finally {
      loading.value = false
    }
  }

  async function createAgent(data: CreateAgentPayload) {
    loading.value = true
    error.value = null
    try {
      const response = await agentsApi.create(data)
      agents.value.push({ ...response.data, channel_count: 0 })
      return response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to create agent'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateAgent(id: string, data: UpdateAgentPayload) {
    loading.value = true
    error.value = null
    try {
      const response = await agentsApi.update(id, data)
      const idx = agents.value.findIndex(a => a.id === id)
      if (idx !== -1) {
        agents.value[idx] = { ...agents.value[idx], ...response.data }
      }
      if (currentAgent.value?.id === id) {
        currentAgent.value = response.data
      }
      return response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to update agent'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteAgent(id: string) {
    loading.value = true
    error.value = null
    try {
      await agentsApi.delete(id)
      agents.value = agents.value.filter(a => a.id !== id)
      if (currentAgent.value?.id === id) {
        currentAgent.value = null
      }
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to delete agent'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchAgentAnalytics(agentId: string, days = 7) {
    loading.value = true
    error.value = null
    try {
      const response = await agentsApi.getAgentAnalytics(agentId, days)
      agentAnalytics.value = response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to load agent analytics'
    } finally {
      loading.value = false
    }
  }

  return {
    agents,
    currentAgent,
    agentAnalytics,
    loading,
    error,
    activeAgents,
    fetchAgents,
    fetchAgent,
    createAgent,
    updateAgent,
    deleteAgent,
    fetchAgentAnalytics,
  }
})
