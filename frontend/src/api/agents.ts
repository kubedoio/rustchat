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

export interface AgentKnowledgeBase {
  agent_id: string
  knowledge_base_id: string
  top_k: number
  relevance_threshold: number | null
}

export interface AgentKnowledgeBaseDetail {
  agent_id: string
  knowledge_base_id: string
  top_k: number
  relevance_threshold: number | null
  knowledge_base_name: string
  knowledge_base_description: string | null
}

export interface AssignKnowledgeBasePayload {
  knowledge_base_id: string
  top_k?: number
  relevance_threshold?: number
}

export interface SubmitFeedbackPayload {
  feedback_type: 'positive' | 'negative'
  comment?: string
}

export interface FeedbackSummary {
  post_id: string
  positive_count: number
  negative_count: number
}

export interface AgentFeedbackStats {
  agent_id: string
  total_positive: number
  total_negative: number
  total_feedback: number
  feedback_ratio: number
}

export interface AgentUsageSummary {
  agent_id: string
  total_invocations: number
  total_tokens_input: number
  total_tokens_output: number
  avg_latency_ms: number
}

export interface AgentDailyUsage {
  date: string
  invocations: number
  tokens_input: number
  tokens_output: number
}

export interface AgentAnalyticsResponse {
  summary: AgentUsageSummary
  daily_usage: AgentDailyUsage[]
  feedback_stats: AgentFeedbackStats
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
  listKnowledgeBases: (id: string) =>
    api.get<AgentKnowledgeBaseDetail[]>(`/agents/${id}/knowledge-bases`),
  assignKnowledgeBase: (id: string, data: AssignKnowledgeBasePayload) =>
    api.post<AgentKnowledgeBase>(`/agents/${id}/knowledge-bases`, data),
  unassignKnowledgeBase: (id: string, kbId: string) =>
    api.delete(`/agents/${id}/knowledge-bases/${kbId}`),
  submitFeedback: (postId: string, data: SubmitFeedbackPayload) =>
    api.post<{
      id: string
      post_id: string
      user_id: string
      feedback_type: string
      comment?: string
      created_at: string
    }>(`/agents/posts/${postId}/feedback`, data),
  getFeedbackSummary: (postId: string) =>
    api.get<FeedbackSummary>(`/agents/posts/${postId}/feedback`),
  deleteFeedback: (postId: string) => api.delete(`/agents/posts/${postId}/feedback`),
  getAgentFeedbackStats: (agentId: string) =>
    api.get<AgentFeedbackStats>(`/agents/${agentId}/feedback-stats`),
  getAgentAnalytics: (agentId: string, days?: number) =>
    api.get<AgentAnalyticsResponse>(`/agents/${agentId}/analytics`, { params: { days } }),
}

export default agentsApi
