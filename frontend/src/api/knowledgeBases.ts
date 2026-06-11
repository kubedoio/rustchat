import api from './client'

export interface KnowledgeBaseSummary {
  id: string
  name: string
  description: string | null
  embedding_model: string
  embedding_dimensions: number
  chunk_size: number
  chunk_overlap: number
  document_count: number
  is_active: boolean
  created_at: string
  updated_at: string
}

export interface KnowledgeBaseDetail {
  id: string
  name: string
  description: string | null
  embedding_model: string
  embedding_dimensions: number
  chunk_size: number
  chunk_overlap: number
  is_active: boolean
  created_at: string
  updated_at: string
  created_by: string
}

export interface KnowledgeBaseDocument {
  id: string
  knowledge_base_id: string
  team_id: string
  title: string
  source_url: string | null
  source_type: string
  s3_key: string
  s3_bucket: string
  content_hash: string
  mime_type: string
  size_bytes: number
  extracted_text: string | null
  extracted_at: string | null
  external_id: string | null
  external_etag: string | null
  external_modified_at: string | null
  sync_source_id: string | null
  is_indexed: boolean
  chunk_count: number
  created_at: string
  updated_at: string
  created_by: string
}

export interface SyncSource {
  id: string
  team_id: string
  name: string
  source_type: string
  sync_mode: string
  sync_interval_minutes: number | null
  is_active: boolean
  last_sync_at: string | null
  last_sync_status: string | null
  last_sync_error: string | null
  next_sync_at: string | null
  document_count: number
  created_at: string
  updated_at: string
}

export interface CreateKnowledgeBasePayload {
  name: string
  description?: string
  embedding_model?: string
  embedding_dimensions?: number
  chunk_size?: number
  chunk_overlap?: number
}

export interface UpdateKnowledgeBasePayload {
  name?: string
  description?: string
  embedding_model?: string
  embedding_dimensions?: number
  chunk_size?: number
  chunk_overlap?: number
  is_active?: boolean
}

export interface CreateSyncSourcePayload {
  name: string
  source_type: string
  config: Record<string, any>
  sync_mode?: string
  sync_interval_minutes?: number
}

// API methods
export const knowledgeBasesApi = {
  list: () => api.get<KnowledgeBaseSummary[]>('/knowledge/bases'),
  create: (data: CreateKnowledgeBasePayload) =>
    api.post<KnowledgeBaseDetail>('/knowledge/bases', data),
  get: (id: string) => api.get<KnowledgeBaseDetail>(`/knowledge/bases/${id}`),
  update: (id: string, data: UpdateKnowledgeBasePayload) =>
    api.put<KnowledgeBaseDetail>(`/knowledge/bases/${id}`, data),
  delete: (id: string) => api.delete(`/knowledge/bases/${id}`),

  // Documents
  listDocuments: (id: string) =>
    api.get<KnowledgeBaseDocument[]>(`/knowledge/bases/${id}/documents`),
  uploadDocument: (id: string, file: File) => {
    const formData = new FormData()
    formData.append('file', file)
    return api.post<KnowledgeBaseDocument>(`/knowledge/bases/${id}/documents`, formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    })
  },
  deleteDocument: (_kbId: string, docId: string) => api.delete(`/knowledge/documents/${docId}`),

  // Sync Sources
  listSyncSources: () => api.get<SyncSource[]>('/knowledge/sync-sources'),
  createSyncSource: (data: CreateSyncSourcePayload) =>
    api.post<SyncSource>('/knowledge/sync-sources', data),
  deleteSyncSource: (sourceId: string) => api.delete(`/knowledge/sync-sources/${sourceId}`),
}

export default knowledgeBasesApi
