import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useAgentStore } from './agentStore'
import agentsApi, {
  type AgentAnalyticsResponse,
  type AgentDetail,
  type AgentSummary,
  type CreateAgentPayload,
} from '@/api/agents'

vi.mock('@/api/agents', () => ({
  default: {
    list: vi.fn(),
    get: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    getAgentAnalytics: vi.fn(),
  },
}))

const mockedAgentsApi = vi.mocked(agentsApi)

function buildAgentSummary(overrides: Partial<AgentSummary> = {}): AgentSummary {
  return {
    id: 'agent-1',
    user_id: 'user-1',
    username: 'triage-bot',
    display_name: 'Triage Bot',
    avatar_url: null,
    title: 'Triage Assistant',
    provider: 'openai',
    model: 'gpt-4.1-mini',
    is_active: true,
    channel_count: 2,
    created_at: '2026-06-10T00:00:00Z',
    ...overrides,
  }
}

function buildAgentDetail(overrides: Partial<AgentDetail> = {}): AgentDetail {
  return {
    ...buildAgentSummary(),
    description: 'Helps triage channels',
    system_prompt: 'Answer as a support assistant',
    temperature: 0.2,
    max_context_messages: 20,
    max_output_tokens: 1024,
    capabilities: {
      respond_to_mentions: true,
      respond_to_all: false,
      use_memory: true,
      use_rag: true,
    },
    rag_enabled: true,
    rag_top_k: 5,
    updated_at: '2026-06-10T00:00:00Z',
    created_by: 'admin-1',
    ...overrides,
  }
}

describe('agentStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('fetches agents and derives active agents', async () => {
    mockedAgentsApi.list.mockResolvedValue({
      data: [buildAgentSummary(), buildAgentSummary({ id: 'agent-2', is_active: false })],
    } as any)

    const store = useAgentStore()
    await store.fetchAgents()

    expect(mockedAgentsApi.list).toHaveBeenCalledTimes(1)
    expect(store.agents).toHaveLength(2)
    expect(store.activeAgents).toEqual([expect.objectContaining({ id: 'agent-1' })])
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('fetches a single agent into currentAgent', async () => {
    const agent = buildAgentDetail({ id: 'agent-2' })
    mockedAgentsApi.get.mockResolvedValue({ data: agent } as any)

    const store = useAgentStore()
    await store.fetchAgent('agent-2')

    expect(mockedAgentsApi.get).toHaveBeenCalledWith('agent-2')
    expect(store.currentAgent).toEqual(agent)
  })

  it('creates an agent and appends it to the summary list', async () => {
    const payload: CreateAgentPayload = {
      username: 'kb-bot',
      email: 'kb-bot@example.com',
      title: 'Knowledge Assistant',
      system_prompt: 'Use the knowledge base',
      provider: 'openai',
      model: 'gpt-4.1-mini',
    }
    const agent = buildAgentDetail({ id: 'agent-3', username: 'kb-bot' })
    mockedAgentsApi.create.mockResolvedValue({ data: agent } as any)

    const store = useAgentStore()
    const result = await store.createAgent(payload)

    expect(mockedAgentsApi.create).toHaveBeenCalledWith(payload)
    expect(result).toEqual(agent)
    expect(store.agents).toEqual([expect.objectContaining({ id: 'agent-3', channel_count: 0 })])
  })

  it('updates both list and current agent state', async () => {
    const updated = buildAgentDetail({
      id: 'agent-1',
      title: 'Updated Assistant',
      is_active: false,
    })
    mockedAgentsApi.update.mockResolvedValue({ data: updated } as any)

    const store = useAgentStore()
    store.agents = [buildAgentSummary({ id: 'agent-1', title: 'Old Assistant' })]
    store.currentAgent = buildAgentDetail({ id: 'agent-1', title: 'Old Assistant' })

    const result = await store.updateAgent('agent-1', { title: 'Updated Assistant' })

    expect(result).toEqual(updated)
    expect(store.agents[0]).toEqual(expect.objectContaining({ title: 'Updated Assistant' }))
    expect(store.currentAgent).toEqual(updated)
  })

  it('deletes an agent from state and clears currentAgent when selected', async () => {
    mockedAgentsApi.delete.mockResolvedValue({ data: undefined } as any)

    const store = useAgentStore()
    store.agents = [buildAgentSummary({ id: 'agent-1' }), buildAgentSummary({ id: 'agent-2' })]
    store.currentAgent = buildAgentDetail({ id: 'agent-1' })

    await store.deleteAgent('agent-1')

    expect(mockedAgentsApi.delete).toHaveBeenCalledWith('agent-1')
    expect(store.agents.map(agent => agent.id)).toEqual(['agent-2'])
    expect(store.currentAgent).toBeNull()
  })

  it('stores agent analytics', async () => {
    const analytics: AgentAnalyticsResponse = {
      summary: {
        agent_id: 'agent-1',
        total_invocations: 4,
        total_tokens_input: 100,
        total_tokens_output: 60,
        avg_latency_ms: 320,
      },
      daily_usage: [],
      feedback_stats: {
        agent_id: 'agent-1',
        total_positive: 3,
        total_negative: 1,
        total_feedback: 4,
        feedback_ratio: 0.75,
      },
    }
    mockedAgentsApi.getAgentAnalytics.mockResolvedValue({ data: analytics } as any)

    const store = useAgentStore()
    await store.fetchAgentAnalytics('agent-1', 14)

    expect(mockedAgentsApi.getAgentAnalytics).toHaveBeenCalledWith('agent-1', 14)
    expect(store.agentAnalytics).toEqual(analytics)
  })

  it('sets an API error message and rethrows when creation fails', async () => {
    const error = new Error('Provider token is required')
    mockedAgentsApi.create.mockRejectedValue(error)

    const store = useAgentStore()
    await expect(
      store.createAgent({
        username: 'broken-bot',
        email: 'broken-bot@example.com',
        title: 'Broken',
        system_prompt: 'fail',
        provider: 'openai',
        model: 'gpt-4.1-mini',
      })
    ).rejects.toThrow('Provider token is required')

    expect(store.error).toBe('Provider token is required')
    expect(store.loading).toBe(false)
  })
})
