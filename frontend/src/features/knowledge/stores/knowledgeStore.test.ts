import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useKnowledgeStore } from './knowledgeStore'
import agentsApi from '@/api/agents'
import { knowledgeBasesApi, type KnowledgeBaseSummary } from '@/api/knowledgeBases'

vi.mock('@/api/agents', () => ({
  default: {
    listKnowledgeBases: vi.fn(),
    assignKnowledgeBase: vi.fn(),
    unassignKnowledgeBase: vi.fn(),
  },
}))

vi.mock('@/api/knowledgeBases', () => ({
  knowledgeBasesApi: {
    list: vi.fn(),
  },
}))

const mockedAgentsApi = vi.mocked(agentsApi)
const mockedKnowledgeBasesApi = vi.mocked(knowledgeBasesApi)

function buildKnowledgeBase(overrides: Partial<KnowledgeBaseSummary> = {}): KnowledgeBaseSummary {
  return {
    id: 'kb-1',
    name: 'Product Docs',
    description: null,
    embedding_model: 'text-embedding-3-small',
    embedding_dimensions: 1536,
    chunk_size: 800,
    chunk_overlap: 120,
    document_count: 1,
    is_active: true,
    created_at: '2026-06-10T00:00:00Z',
    updated_at: '2026-06-10T00:00:00Z',
    ...overrides,
  }
}

describe('knowledgeStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('fetches knowledge bases from array responses', async () => {
    mockedKnowledgeBasesApi.list.mockResolvedValue({
      data: [buildKnowledgeBase({ id: 'kb-1' }), buildKnowledgeBase({ id: 'kb-2' })],
    } as any)

    const store = useKnowledgeStore()
    await store.fetchKnowledgeBases()

    expect(store.knowledgeBases.map(kb => kb.id)).toEqual(['kb-1', 'kb-2'])
    expect(store.error).toBeNull()
  })

  it('normalizes wrapped knowledge base list responses', async () => {
    mockedKnowledgeBasesApi.list.mockResolvedValue({
      data: { knowledge_bases: [buildKnowledgeBase({ id: 'kb-3' })] },
    } as any)

    const store = useKnowledgeStore()
    await store.fetchKnowledgeBases()

    expect(store.knowledgeBases.map(kb => kb.id)).toEqual(['kb-3'])
  })

  it('fetches agent knowledge bases from array and wrapped responses', async () => {
    mockedAgentsApi.listKnowledgeBases.mockResolvedValueOnce({
      data: [
        {
          agent_id: 'agent-1',
          knowledge_base_id: 'kb-1',
          top_k: 5,
          relevance_threshold: null,
          knowledge_base_name: 'Product Docs',
          knowledge_base_description: null,
        },
      ],
    } as any)
    mockedAgentsApi.listKnowledgeBases.mockResolvedValueOnce({
      data: {
        knowledge_bases: [
          {
            agent_id: 'agent-1',
            knowledge_base_id: 'kb-2',
            top_k: 3,
            relevance_threshold: 0.8,
            knowledge_base_name: 'Runbooks',
            knowledge_base_description: 'Ops docs',
          },
        ],
      },
    } as any)

    const store = useKnowledgeStore()
    await store.fetchAgentKbs('agent-1')
    expect(store.agentKbs.map(kb => kb.knowledge_base_id)).toEqual(['kb-1'])

    await store.fetchAgentKbs('agent-1')
    expect(store.agentKbs.map(kb => kb.knowledge_base_id)).toEqual(['kb-2'])
  })

  it('assigns and unassigns knowledge bases for agents', async () => {
    mockedAgentsApi.assignKnowledgeBase.mockResolvedValue({ data: undefined } as any)
    mockedAgentsApi.unassignKnowledgeBase.mockResolvedValue({ data: undefined } as any)

    const store = useKnowledgeStore()
    await store.assignKbToAgent('agent-1', {
      knowledge_base_id: 'kb-1',
      top_k: 5,
      relevance_threshold: 0.7,
    })
    await store.unassignKbFromAgent('agent-1', 'kb-1')

    expect(mockedAgentsApi.assignKnowledgeBase).toHaveBeenCalledWith('agent-1', {
      knowledge_base_id: 'kb-1',
      top_k: 5,
      relevance_threshold: 0.7,
    })
    expect(mockedAgentsApi.unassignKnowledgeBase).toHaveBeenCalledWith('agent-1', 'kb-1')
    expect(store.loading).toBe(false)
  })

  it('sets an API error message and rethrows when assignment fails', async () => {
    const error = new Error('Knowledge base is inactive')
    mockedAgentsApi.assignKnowledgeBase.mockRejectedValue(error)

    const store = useKnowledgeStore()
    await expect(
      store.assignKbToAgent('agent-1', { knowledge_base_id: 'kb-1', top_k: 5 })
    ).rejects.toThrow('Knowledge base is inactive')

    expect(store.error).toBe('Knowledge base is inactive')
    expect(store.loading).toBe(false)
  })
})
