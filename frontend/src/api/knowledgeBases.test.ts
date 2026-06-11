import { beforeEach, describe, expect, it, vi } from 'vitest'
import knowledgeBasesApi, {
  type CreateKnowledgeBasePayload,
  type CreateSyncSourcePayload,
  type UpdateKnowledgeBasePayload,
} from './knowledgeBases'
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

describe('knowledgeBasesApi', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('maps knowledge base CRUD methods to v1 knowledge endpoints', () => {
    const createPayload: CreateKnowledgeBasePayload = {
      name: 'Product Docs',
      description: 'Internal product knowledge',
    }
    const updatePayload: UpdateKnowledgeBasePayload = {
      name: 'Updated Product Docs',
      is_active: false,
    }

    knowledgeBasesApi.list()
    knowledgeBasesApi.create(createPayload)
    knowledgeBasesApi.get('kb-1')
    knowledgeBasesApi.update('kb-1', updatePayload)
    knowledgeBasesApi.delete('kb-1')

    expect(mockedApi.get).toHaveBeenCalledWith('/knowledge/bases')
    expect(mockedApi.post).toHaveBeenCalledWith('/knowledge/bases', createPayload)
    expect(mockedApi.get).toHaveBeenCalledWith('/knowledge/bases/kb-1')
    expect(mockedApi.put).toHaveBeenCalledWith('/knowledge/bases/kb-1', updatePayload)
    expect(mockedApi.delete).toHaveBeenCalledWith('/knowledge/bases/kb-1')
  })

  it('uploads documents as multipart form data', () => {
    const file = new File(['hello'], 'guide.md', { type: 'text/markdown' })

    knowledgeBasesApi.listDocuments('kb-1')
    knowledgeBasesApi.uploadDocument('kb-1', file)
    knowledgeBasesApi.deleteDocument('kb-1', 'doc-1')

    expect(mockedApi.get).toHaveBeenCalledWith('/knowledge/bases/kb-1/documents')
    expect(mockedApi.post).toHaveBeenCalledWith(
      '/knowledge/bases/kb-1/documents',
      expect.any(FormData),
      {
        headers: { 'Content-Type': 'multipart/form-data' },
      }
    )
    const formData = vi.mocked(mockedApi.post).mock.calls[0]?.[1] as FormData
    expect(formData.get('file')).toBe(file)
    expect(mockedApi.delete).toHaveBeenCalledWith('/knowledge/documents/doc-1')
  })

  it('maps sync source methods to sync-source endpoints', () => {
    const payload: CreateSyncSourcePayload = {
      source_type: 'github',
      config: { repository: 'kubedoio/rustchat' },
      sync_interval_minutes: 60,
    }

    knowledgeBasesApi.listSyncSources()
    knowledgeBasesApi.createSyncSource(payload)
    knowledgeBasesApi.deleteSyncSource('source-1')

    expect(mockedApi.get).toHaveBeenCalledWith('/knowledge/sync-sources')
    expect(mockedApi.post).toHaveBeenCalledWith('/knowledge/sync-sources', payload)
    expect(mockedApi.delete).toHaveBeenCalledWith('/knowledge/sync-sources/source-1')
  })
})
