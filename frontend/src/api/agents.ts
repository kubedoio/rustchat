import api from './client'

export interface AgentSummary {
  id: string
  user_id: string
  username: string
  display_name: string | null
  avatar_url: string | null
  title: string
  provider: string
  model: string
  is_active: boolean
  channel_count: number
  created_at: string
}

export interface AgentDetail {
  id: string
  user_id: string
  username: string
  display_name: string | null
  avatar_url: string | null
  title: string
  description: string | null
  system_prompt: string
  provider: string
  model: string
  temperature: number
  max_context_messages: number
  max_output_tokens: number
  capabilities: {
    respond_to_mentions: boolean
    respond_to_all: boolean
    use_memory: boolean
    use_rag: boolean
  }
  rag_enabled: boolean
  rag_top_k: number
  is_active: boolean
  created_at: string
  updated_at: string
  created_by: string
}

export interface CreateAgentPayload {
  username: string
  email: string
  display_name?: string
  title: string
  description?: string
  system_prompt: string
  provider: string
  model: string
  api_token?: string
  temperature?: number
  max_context_messages?: number
  max_output_tokens?: number
  capabilities?: {
    respond_to_mentions: boolean
    respond_to_all: boolean
    use_memory: boolean
    use_rag: boolean
  }
  rag_enabled?: boolean
  rag_top_k?: number
  channel_ids?: string[]
}

export interface UpdateAgentPayload {
  title?: string
  description?: string
  system_prompt?: string
  provider?: string
  model?: string
  api_token?: string
  temperature?: number
  max_context_messages?: number
  max_output_tokens?: number
  capabilities?: {
    respond_to_mentions: boolean
    respond_to_all: boolean
    use_memory: boolean
    use_rag: boolean
  }
  rag_enabled?: boolean
  rag_top_k?: number
  is_active?: boolean
}

export interface TestAgentPayload {
  message: string
  channel_id?: string
}

export interface TestAgentResponse {
  response: string
  provider: string
  model: string
  latency_ms: number
}

// API methods
export const agentsApi = {
  list: () => api.get<{ agents: AgentSummary[] }>('/agents'),
  create: (data: CreateAgentPayload) => api.post<AgentDetail>('/agents', data),
  get: (id: string) => api.get<AgentDetail>(`/agents/${id}`),
  update: (id: string, data: UpdateAgentPayload) => api.put<AgentDetail>(`/agents/${id}`, data),
  delete: (id: string) => api.delete(`/agents/${id}`),
  regenerateKey: (id: string) => api.post<{ api_key: string }>(`/agents/${id}/regenerate-key`),
  listChannels: (id: string) => api.get<{ channels: any[] }>(`/agents/${id}/channels`),
  addChannel: (id: string, channelId: string) => api.post(`/agents/${id}/channels/${channelId}`),
  removeChannel: (id: string, channelId: string) =>
    api.delete(`/agents/${id}/channels/${channelId}`),
  listMemories: (id: string) => api.get<{ memories: any[] }>(`/agents/${id}/memories`),
  deleteMemory: (id: string, memoryId: string) => api.delete(`/agents/${id}/memories/${memoryId}`),
  test: (id: string, data: TestAgentPayload) =>
    api.post<TestAgentResponse>(`/agents/${id}/test`, data),
}

export default agentsApi
