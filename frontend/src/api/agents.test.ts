import { beforeEach, describe, expect, it, vi } from 'vitest'
import agentsApi, {
  type AssignKnowledgeBasePayload,
  type CreateAgentPayload,
  type SubmitFeedbackPayload,
  type TestAgentPayload,
  type UpdateAgentPayload,
} from './agents'
import api from './client'

vi.mock('./client', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
  },
}))

const mockedApi = vi.mocked(api)

describe('agentsApi', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('maps agent CRUD methods to v1 agent endpoints', () => {
    const createPayload: CreateAgentPayload = {
      username: 'triage-bot',
      email: 'triage-bot@example.com',
      display_name: 'Triage Bot',
      title: 'Triage Assistant',
      system_prompt: 'Help triage issues',
      provider: 'openai',
      model: 'gpt-4.1-mini',
    }
    const updatePayload: UpdateAgentPayload = {
      title: 'Updated Triage Assistant',
      is_active: false,
    }

    agentsApi.list()
    agentsApi.create(createPayload)
    agentsApi.get('agent-1')
    agentsApi.update('agent-1', updatePayload)
    agentsApi.delete('agent-1')
    agentsApi.regenerateKey('agent-1')

    expect(mockedApi.get).toHaveBeenCalledWith('/agents')
    expect(mockedApi.post).toHaveBeenCalledWith('/agents', createPayload)
    expect(mockedApi.get).toHaveBeenCalledWith('/agents/agent-1')
    expect(mockedApi.put).toHaveBeenCalledWith('/agents/agent-1', updatePayload)
    expect(mockedApi.delete).toHaveBeenCalledWith('/agents/agent-1')
    expect(mockedApi.post).toHaveBeenCalledWith('/agents/agent-1/regenerate-key')
  })

  it('maps channel, memory, and test methods to agent subresources', () => {
    const testPayload: TestAgentPayload = {
      message: 'Summarize this channel',
      channel_id: 'channel-1',
    }

    agentsApi.listChannels('agent-1')
    agentsApi.addChannel('agent-1', 'channel-1')
    agentsApi.removeChannel('agent-1', 'channel-1')
    agentsApi.listMemories('agent-1')
    agentsApi.deleteMemory('agent-1', 'memory-1')
    agentsApi.test('agent-1', testPayload)

    expect(mockedApi.get).toHaveBeenCalledWith('/agents/agent-1/channels')
    expect(mockedApi.post).toHaveBeenCalledWith('/agents/agent-1/channels/channel-1')
    expect(mockedApi.delete).toHaveBeenCalledWith('/agents/agent-1/channels/channel-1')
    expect(mockedApi.get).toHaveBeenCalledWith('/agents/agent-1/memories')
    expect(mockedApi.delete).toHaveBeenCalledWith('/agents/agent-1/memories/memory-1')
    expect(mockedApi.post).toHaveBeenCalledWith('/agents/agent-1/test', testPayload)
  })

  it('maps knowledge-base assignment methods to agent endpoints', () => {
    const payload: AssignKnowledgeBasePayload = {
      knowledge_base_id: 'kb-1',
      top_k: 5,
      relevance_threshold: 0.7,
    }

    agentsApi.listKnowledgeBases('agent-1')
    agentsApi.assignKnowledgeBase('agent-1', payload)
    agentsApi.unassignKnowledgeBase('agent-1', 'kb-1')

    expect(mockedApi.get).toHaveBeenCalledWith('/agents/agent-1/knowledge-bases')
    expect(mockedApi.post).toHaveBeenCalledWith('/agents/agent-1/knowledge-bases', payload)
    expect(mockedApi.delete).toHaveBeenCalledWith('/agents/agent-1/knowledge-bases/kb-1')
  })

  it('uses post-scoped feedback endpoints and agent analytics endpoints', () => {
    const feedbackPayload: SubmitFeedbackPayload = {
      feedback_type: 'positive',
      comment: 'Useful response',
    }

    agentsApi.submitFeedback('post-1', feedbackPayload)
    agentsApi.getFeedbackSummary('post-1')
    agentsApi.deleteFeedback('post-1')
    agentsApi.getAgentFeedbackStats('agent-1')
    agentsApi.getAgentAnalytics('agent-1', 30)

    expect(mockedApi.post).toHaveBeenCalledWith('/agents/posts/post-1/feedback', feedbackPayload)
    expect(mockedApi.get).toHaveBeenCalledWith('/agents/posts/post-1/feedback')
    expect(mockedApi.delete).toHaveBeenCalledWith('/agents/posts/post-1/feedback')
    expect(mockedApi.get).toHaveBeenCalledWith('/agents/agent-1/feedback-stats')
    expect(mockedApi.get).toHaveBeenCalledWith('/agents/agent-1/analytics', {
      params: { days: 30 },
    })
  })
})
