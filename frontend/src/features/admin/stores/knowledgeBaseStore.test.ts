import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useKnowledgeBaseStore } from './knowledgeBaseStore'
import knowledgeBasesApi, {
  type KnowledgeBaseDetail,
  type KnowledgeBaseDocument,
  type KnowledgeBaseSummary,
  type SyncSource,
} from '@/api/knowledgeBases'

vi.mock('@/api/knowledgeBases', () => ({
  default: {
    list: vi.fn(),
    get: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    listDocuments: vi.fn(),
    uploadDocument: vi.fn(),
    deleteDocument: vi.fn(),
    listSyncSources: vi.fn(),
    createSyncSource: vi.fn(),
    deleteSyncSource: vi.fn(),
  },
}))

const mockedKnowledgeBasesApi = vi.mocked(knowledgeBasesApi)

function buildKnowledgeBaseSummary(
  overrides: Partial<KnowledgeBaseSummary> = {}
): KnowledgeBaseSummary {
  return {
    id: 'kb-1',
    name: 'Product Docs',
    description: 'Internal docs',
    embedding_model: 'text-embedding-3-small',
    embedding_dimensions: 1536,
    chunk_size: 800,
    chunk_overlap: 120,
    document_count: 2,
    is_active: true,
    created_at: '2026-06-10T00:00:00Z',
    updated_at: '2026-06-10T00:00:00Z',
    ...overrides,
  }
}

function buildKnowledgeBaseDetail(
  overrides: Partial<KnowledgeBaseDetail> = {}
): KnowledgeBaseDetail {
  return {
    ...buildKnowledgeBaseSummary(),
    created_by: 'admin-1',
    ...overrides,
  }
}

function buildDocument(overrides: Partial<KnowledgeBaseDocument> = {}): KnowledgeBaseDocument {
  return {
    id: 'doc-1',
    knowledge_base_id: 'kb-1',
    filename: 'guide.md',
    file_type: 'text/markdown',
    file_size: 1024,
    chunk_count: 4,
    status: 'completed',
    created_at: '2026-06-10T00:00:00Z',
    updated_at: '2026-06-10T00:00:00Z',
    ...overrides,
  }
}

function buildSyncSource(overrides: Partial<SyncSource> = {}): SyncSource {
  return {
    id: 'source-1',
    knowledge_base_id: 'kb-1',
    source_type: 'github',
    config: { repository: 'kubedoio/rustchat' },
    sync_interval_minutes: 60,
    last_sync_at: null,
    next_sync_at: null,
    is_active: true,
    created_at: '2026-06-10T00:00:00Z',
    ...overrides,
  }
}

describe('knowledgeBaseStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('fetches knowledge bases and derives active knowledge bases', async () => {
    mockedKnowledgeBasesApi.list.mockResolvedValue({
      data: [
        buildKnowledgeBaseSummary({ id: 'kb-1', is_active: true }),
        buildKnowledgeBaseSummary({ id: 'kb-2', is_active: false }),
      ],
    } as any)

    const store = useKnowledgeBaseStore()
    await store.fetchKnowledgeBases()

    expect(mockedKnowledgeBasesApi.list).toHaveBeenCalledTimes(1)
    expect(store.knowledgeBases).toHaveLength(2)
    expect(store.activeKnowledgeBases).toEqual([expect.objectContaining({ id: 'kb-1' })])
  })

  it('creates, updates, and deletes knowledge bases in local state', async () => {
    const created = buildKnowledgeBaseDetail({ id: 'kb-3', name: 'Runbooks' })
    const updated = buildKnowledgeBaseDetail({ id: 'kb-3', name: 'Updated Runbooks' })
    mockedKnowledgeBasesApi.create.mockResolvedValue({ data: created } as any)
    mockedKnowledgeBasesApi.update.mockResolvedValue({ data: updated } as any)
    mockedKnowledgeBasesApi.delete.mockResolvedValue({ data: undefined } as any)

    const store = useKnowledgeBaseStore()
    await store.createKnowledgeBase({ name: 'Runbooks' })

    expect(store.knowledgeBases).toEqual([
      expect.objectContaining({ id: 'kb-3', document_count: 0 }),
    ])

    store.currentKnowledgeBase = created
    await store.updateKnowledgeBase('kb-3', { name: 'Updated Runbooks' })

    expect(store.knowledgeBases[0]).toEqual(expect.objectContaining({ name: 'Updated Runbooks' }))
    expect(store.currentKnowledgeBase).toEqual(updated)

    await store.deleteKnowledgeBase('kb-3')

    expect(store.knowledgeBases).toEqual([])
    expect(store.currentKnowledgeBase).toBeNull()
  })

  it('fetches documents and keeps document counts in sync after upload and delete', async () => {
    const file = new File(['hello'], 'guide.md', { type: 'text/markdown' })
    mockedKnowledgeBasesApi.listDocuments.mockResolvedValue({ data: [buildDocument()] } as any)
    mockedKnowledgeBasesApi.uploadDocument.mockResolvedValue({
      data: buildDocument({ id: 'doc-2', filename: 'faq.md' }),
    } as any)
    mockedKnowledgeBasesApi.deleteDocument.mockResolvedValue({ data: undefined } as any)

    const store = useKnowledgeBaseStore()
    store.knowledgeBases = [buildKnowledgeBaseSummary({ id: 'kb-1', document_count: 1 })]

    await store.fetchDocuments('kb-1')
    expect(store.documents.map(document => document.id)).toEqual(['doc-1'])

    await store.uploadDocument('kb-1', file)
    expect(mockedKnowledgeBasesApi.uploadDocument).toHaveBeenCalledWith('kb-1', file)
    expect(store.documents.map(document => document.id)).toEqual(['doc-1', 'doc-2'])
    expect(store.knowledgeBases[0]?.document_count).toBe(2)

    await store.deleteDocument('kb-1', 'doc-2')
    expect(store.documents.map(document => document.id)).toEqual(['doc-1'])
    expect(store.knowledgeBases[0]?.document_count).toBe(1)
  })

  it('fetches, creates, and deletes sync sources', async () => {
    mockedKnowledgeBasesApi.listSyncSources.mockResolvedValue({ data: [buildSyncSource()] } as any)
    mockedKnowledgeBasesApi.createSyncSource.mockResolvedValue({
      data: buildSyncSource({ id: 'source-2' }),
    } as any)
    mockedKnowledgeBasesApi.deleteSyncSource.mockResolvedValue({ data: undefined } as any)

    const store = useKnowledgeBaseStore()
    await store.fetchSyncSources()
    await store.createSyncSource({
      source_type: 'github',
      config: { repository: 'kubedoio/rustchat' },
    })
    await store.deleteSyncSource('source-1')

    expect(store.syncSources.map(source => source.id)).toEqual(['source-2'])
  })

  it('sets an API error message and rethrows when upload fails', async () => {
    const error = new Error('Unsupported file type')
    mockedKnowledgeBasesApi.uploadDocument.mockRejectedValue(error)

    const store = useKnowledgeBaseStore()
    await expect(store.uploadDocument('kb-1', new File(['bad'], 'bad.exe'))).rejects.toThrow(
      'Unsupported file type'
    )

    expect(store.error).toBe('Unsupported file type')
    expect(store.loading).toBe(false)
  })
})
