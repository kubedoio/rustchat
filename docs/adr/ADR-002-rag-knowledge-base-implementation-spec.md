# ADR-002 Implementation Spec: RAG Knowledge Base (Phase 2)

**Date:** 2026-06-09
**Status:** Draft
**Issue:** [#177](https://github.com/kubedoio/rustchat/issues/177)
**Depends on:** ADR-001 (AI Agents Phase 1 — complete)

---

## Goal

Implement the full RAG knowledge base subsystem:
1. Document upload, storage, and extraction
2. Chunking and embedding pipeline
3. pgvector-backed semantic search
4. RustShare sync (first-class external source)
5. Agent integration — agents retrieve relevant chunks at query time

---

## Pre-Flight Checklist

Before starting implementation:

1. ✅ ADR-001 (Phase 1) is merged to `main`
2. ✅ Branch from latest `main`: `git checkout -b feature/rag-knowledge-base`
3. ✅ PostgreSQL has `pgvector` extension installed:
   ```sql
   CREATE EXTENSION IF NOT EXISTS vector;
   ```
4. ✅ `RUSTCHAT_OPENAI_API_KEY` is configured (for embeddings)

---

## Slice Breakdown

### Slice 1: Database + Models + Repository (Foundation)

**Objective:** Create all tables, Rust models, and repository methods. No business logic yet.

**Files to create:**

| File | Purpose |
|------|---------|
| `backend/migrations/20260610000001_knowledge_bases.sql` | `knowledge_bases` table |
| `backend/migrations/20260610000002_knowledge_documents.sql` | `knowledge_documents` table |
| `backend/migrations/20260610000003_knowledge_chunks.sql` | `knowledge_chunks` table + HNSW index |
| `backend/migrations/20260610000004_knowledge_sync_sources.sql` | `knowledge_sync_sources` table |
| `backend/migrations/20260610000005_agent_knowledge_bases.sql` | Junction table for agent↔KB mapping |
| `backend/src/models/knowledge.rs` | All knowledge-related structs |
| `backend/src/repositories/knowledge_repository.rs` | CRUD + search operations |

**Key model types:**

```rust
// models/knowledge.rs
pub struct KnowledgeBase { ... }
pub struct KnowledgeDocument { ... }
pub struct KnowledgeChunk { ... }
pub struct KnowledgeSyncSource { ... }
pub struct AgentKnowledgeBase { ... }

// Search types
pub struct SearchFilter {
    pub team_id: Uuid,
    pub knowledge_base_id: Option<Uuid>,
    pub document_id: Option<Uuid>,
}

pub struct RetrievedChunk {
    pub chunk_text: String,
    pub document_title: String,
    pub document_source_url: Option<String>,
    pub section_title: Option<String>,
    pub similarity: f32,  // cosine similarity score
}
```

**Repository methods:**

```rust
impl KnowledgeRepository {
    // Knowledge bases
    pub async fn create_knowledge_base(&self, kb: &CreateKnowledgeBase) -> Result<KnowledgeBase>;
    pub async fn list_knowledge_bases(&self, team_id: Uuid) -> Result<Vec<KnowledgeBase>>;
    pub async fn get_knowledge_base(&self, id: Uuid, team_id: Uuid) -> Result<KnowledgeBase>;
    pub async fn update_knowledge_base(&self, id: Uuid, team_id: Uuid, update: &UpdateKnowledgeBase) -> Result<KnowledgeBase>;
    pub async fn delete_knowledge_base(&self, id: Uuid, team_id: Uuid) -> Result<()>;

    // Documents
    pub async fn create_document(&self, doc: &CreateDocument) -> Result<KnowledgeDocument>;
    pub async fn get_document_by_hash(&self, content_hash: &str, team_id: Uuid) -> Result<Option<KnowledgeDocument>>;
    pub async fn update_document_indexed(&self, id: Uuid, chunk_count: i32) -> Result<()>;
    pub async fn delete_document(&self, id: Uuid, team_id: Uuid) -> Result<()>;

    // Chunks + search
    pub async fn insert_chunks(&self, chunks: &[KnowledgeChunk]) -> Result<()>;
    pub async fn search_chunks(&self, embedding: &[f32], top_k: i32, filter: &SearchFilter) -> Result<Vec<RetrievedChunk>>;
    pub async fn delete_chunks_by_document(&self, document_id: Uuid) -> Result<()>;

    // Agent↔KB mapping
    pub async fn assign_kb_to_agent(&self, agent_id: Uuid, kb_id: Uuid, config: &AgentKbConfig) -> Result<()>;
    pub async fn list_agent_knowledge_bases(&self, agent_id: Uuid) -> Result<Vec<AgentKnowledgeBaseDetail>>;
    pub async fn unassign_kb_from_agent(&self, agent_id: Uuid, kb_id: Uuid) -> Result<()>;

    // Sync sources
    pub async fn create_sync_source(&self, source: &CreateSyncSource) -> Result<KnowledgeSyncSource>;
    pub async fn get_sync_source(&self, id: Uuid, team_id: Uuid) -> Result<KnowledgeSyncSource>;
    pub async fn update_sync_state(&self, id: Uuid, state: &SyncState) -> Result<()>;
}
```

**Verification:** `cargo check --lib` passes.

---

### Slice 2: Document Upload + Storage + Extraction

**Objective:** Users can upload documents via API; text is extracted and stored.

**Components:**

1. **Upload API** (`POST /api/v1/knowledge/{kb_id}/documents`)
   - Accepts multipart form data (file upload)
   - Streams file to S3/RustFS
   - Computes SHA-256 content hash
   - Deduplication: if hash exists, return existing document (idempotent)
   - Returns `KnowledgeDocument` with `is_indexed: false`

2. **Text Extractor** (`backend/src/services/knowledge/extractor.rs`)
   - Trait-based: `DocumentExtractor`
   - Implementations:
     - `PlainTextExtractor` — `.txt`, `.md`, `.rs`, `.py`, etc. (pass-through)
     - `PdfExtractor` — using `pdf-extract` or `lopdf` crate
     - `HtmlExtractor` — using `html2text` or `scraper`
     - `DocxExtractor` — using `docx-rs`
   - Extracted text stored in `knowledge_documents.extracted_text`

3. **S3 Integration**
   - S3 key pattern: `knowledge/{team_id}/{kb_id}/{doc_id}/{filename}`
   - Reuses existing `StorageService` / `S3Client`

**API Endpoints:**

```
POST   /api/v1/knowledge/bases                    → create KB
GET    /api/v1/knowledge/bases                    → list KBs
GET    /api/v1/knowledge/bases/:id                → get KB
PUT    /api/v1/knowledge/bases/:id                → update KB
DELETE /api/v1/knowledge/bases/:id                → delete KB

POST   /api/v1/knowledge/bases/:id/documents      → upload document
GET    /api/v1/knowledge/bases/:id/documents      → list documents
GET    /api/v1/knowledge/documents/:doc_id        → get document
DELETE /api/v1/knowledge/documents/:doc_id        → delete document
GET    /api/v1/knowledge/documents/:doc_id/download → presigned S3 URL
```

**Verification:**
- Upload a `.txt` file → document created, S3 key valid, text extracted
- Upload same file again → returns existing document (dedup)
- `cargo test` for repository + API integration

---

### Slice 3: Chunker + Embedder + Indexing Pipeline

**Objective:** Turn extracted text into searchable embedding vectors.

**Components:**

1. **Chunker** (`backend/src/services/knowledge/chunker.rs`)
   ```rust
   pub trait Chunker: Send + Sync {
       fn chunk(&self, text: &str, config: &ChunkConfig) -> Vec<Chunk>;
   }

   pub struct MarkdownChunker;   // splits on headers
   pub struct CodeChunker;       // splits on function/class boundaries
   pub struct SlidingWindowChunker;  // fallback: fixed size + overlap
   ```

   Chunk selection heuristic based on `mime_type`:
   - `text/markdown` → `MarkdownChunker`
   - `text/x-rust`, `text/x-python`, etc. → `CodeChunker`
   - everything else → `SlidingWindowChunker`

2. **Embedder** (`backend/src/services/knowledge/embedder.rs`)
   ```rust
   #[async_trait]
   pub trait Embedder: Send + Sync {
       async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
   }

   pub struct OpenAiEmbedder { client: reqwest::Client, api_key: String, model: String }
   pub struct FastembedEmbedder { model: FlagEmbedding }  // local, no API calls
   ```

   - OpenAiEmbedder uses `text-embedding-3-small` by default
   - Batches up to 96 texts per request (OpenAI limit)
   - Retry with exponential backoff on rate limit

3. **Indexer Service** (`backend/src/services/knowledge/indexer.rs`)
   ```rust
   pub struct IndexerService {
       repository: KnowledgeRepository,
       chunker: Arc<dyn Chunker>,
       embedder: Arc<dyn Embedder>,
       vector_store: Arc<dyn VectorStore>,
   }

   impl IndexerService {
       pub async fn index_document(&self, document_id: Uuid) -> Result<()> {
           // 1. Load document + extracted text
           // 2. Delete old chunks (re-index)
           // 3. Chunk text
           // 4. Embed chunks in batches
           // 5. Insert chunks into vector store
           // 6. Update document.is_indexed = true
       }
   }
   ```

4. **VectorStore trait + PgVectorStore impl**
   ```rust
   #[async_trait]
   pub trait VectorStore: Send + Sync {
       async fn upsert_chunks(&self, chunks: &[KnowledgeChunk]) -> Result<()>;
       async fn search(&self, embedding: &[f32], top_k: usize, filter: &SearchFilter) -> Result<Vec<RetrievedChunk>>;
       async fn delete_document_chunks(&self, document_id: Uuid) -> Result<()>;
   }
   ```

   `PgVectorStore` uses raw SQL with `sqlx` (not `sqlx::query_as!` macro, per project convention):
   ```sql
   SELECT chunk_text, document_id, section_title,
          1 - (embedding <=> $1) AS similarity
   FROM knowledge_chunks
   WHERE team_id = $2 AND knowledge_base_id = $3
   ORDER BY embedding <=> $1
   LIMIT $4;
   ```

**Background job trigger:**
- After document upload/extraction, spawn a tokio task to call `indexer_service.index_document(doc_id)`.
- If indexing fails, store error on document row for retry.

**Verification:**
- Upload a markdown file → chunks created, embeddings generated, HNSW search returns relevant chunks
- Unit test: chunk a 2000-token text → verify overlap and boundary logic
- Unit test: mock embedder → verify batching and DB insertion

---

### Slice 4: Agent RAG Integration

**Objective:** Agents automatically retrieve relevant knowledge chunks when responding to mentions.

**Changes:**

1. **Update `AgentMemoryService`** to include RAG context:
   ```rust
   pub struct AgentMemoryService {
       repository: KnowledgeRepository,
       vector_store: Arc<dyn VectorStore>,
       embedder: Arc<dyn Embedder>,
   }

   impl AgentMemoryService {
       pub async fn build_context(&self, agent_id: Uuid, channel_id: Uuid, query: &str) -> Result<String> {
           // 1. Build conversation context (existing Phase 1 logic)
           let conversation = self.build_conversation_context(channel_id).await?;

           // 2. Retrieve relevant knowledge chunks
           let kb_ids = self.repository.list_agent_knowledge_bases(agent_id).await?;
           let mut rag_context = String::new();

           if !kb_ids.is_empty() {
               let query_embedding = self.embedder.embed(&[query.to_string()]).await?.pop().unwrap();
               for kb in kb_ids {
                   let chunks = self.vector_store.search(
                       &query_embedding,
                       kb.top_k as usize,
                       &SearchFilter { team_id: kb.team_id, knowledge_base_id: Some(kb.id), document_id: None }
                   ).await?;

                   for chunk in chunks {
                       if chunk.similarity >= kb.relevance_threshold.unwrap_or(0.7) {
                           rag_context.push_str(&format!("\n[{}] {}\n", chunk.document_title, chunk.section_title.unwrap_or_default()));
                           rag_context.push_str(&chunk.chunk_text);
                       }
                   }
               }
           }

           // 3. Assemble final prompt
           Ok(format!("{}\n\n## Relevant Knowledge:\n{}\n\n## Conversation:\n{}",
               system_prompt, rag_context, conversation))
       }
   }
   ```

2. **Update `AgentRuntime`** to pass the user's mention message as the RAG query:
   - The mention text (e.g., "@dev-assistant how do I configure OAuth?") is the query.
   - `AgentMemoryService::build_context` embeds this query and retrieves chunks.

3. **Update `agent_knowledge_bases` API**:
   ```
   POST   /api/v1/agents/:id/knowledge-bases       → assign KB to agent
   GET    /api/v1/agents/:id/knowledge-bases       → list agent's KBs
   DELETE /api/v1/agents/:id/knowledge-bases/:kb_id → unassign
   ```

**Verification:**
- Create KB, upload document, assign KB to agent
- Mention agent with a question related to document content
- Verify agent response includes grounded information from the document

---

### Slice 5: RustShare Sync (First-Class External Source)

**Objective:** Bi-directional sync between RustShare folders and RustChat knowledge bases.

**RustShare API contract (assumed):**
```rust
// RustShare exposes a REST API:
// GET  /api/v1/folders/{folder_id}/files?modified_since={iso8601}&etag={etag}
// GET  /api/v1/files/{file_id}/download → redirect to presigned URL
// POST /api/v1/webhooks → register callback URL for events

// Webhook events:
// { "event": "file.created", "file_id": "...", "folder_id": "...", "etag": "...", "modified_at": "..." }
// { "event": "file.updated", "file_id": "...", ... }
// { "event": "file.deleted", "file_id": "...", ... }
```

**Components:**

1. **RustShare Client** (`backend/src/services/sync/rustshare/client.rs`)
   ```rust
   pub struct RustShareClient {
       base_url: String,
       auth_token: String,
       http: reqwest::Client,
   }

   impl RustShareClient {
       pub async fn list_files(&self, folder_id: &str, since: Option<DateTime<Utc>>) -> Result<Vec<RustShareFile>>;
       pub async fn download_file(&self, file_id: &str) -> Result<bytes::Bytes>;
       pub async fn register_webhook(&self, url: &str, events: &[&str]) -> Result<String>; // returns webhook_id
   }
   ```

2. **Sync Orchestrator** (`backend/src/services/sync/rustshare/orchestrator.rs`)
   ```rust
   pub struct RustShareSyncOrchestrator {
       rustshare_client: Arc<RustShareClient>,
       storage: Arc<StorageService>,
       indexer: Arc<IndexerService>,
       repository: KnowledgeRepository,
   }

   impl RustShareSyncOrchestrator {
       /// Full sync (run on initial setup or manual trigger)
       pub async fn full_sync(&self, sync_source_id: Uuid) -> Result<SyncReport>;

       /// Incremental sync (run periodically or on webhook)
       pub async fn incremental_sync(&self, sync_source_id: Uuid, events: Vec<SyncEvent>) -> Result<SyncReport>;
   }
   ```

   Full sync logic:
   ```
   1. Load sync_source config (folder_id, recursive, etc.)
   2. List all files in RustShare folder (paginated)
   3. For each file:
      a. Check if document with same external_id exists
      b. If exists and etag matches → skip
      c. If exists and etag differs → re-download, re-extract, re-index
      d. If new → download, create document, extract, index
   4. For each local document with this sync_source_id not seen in remote → mark deleted
   ```

3. **Webhook Handler** (`backend/src/api/v1/knowledge/sync.rs`)
   ```rust
   pub async fn handle_rustshare_webhook(
       State(state): State<Arc<AppState>>,
       headers: HeaderMap,
       Json(payload): Json<RustShareWebhookPayload>,
   ) -> ApiResult<impl IntoResponse> {
       // Verify webhook signature (HMAC-SHA256)
       // Lookup sync_source by webhook_id
       // Spawn background task for incremental_sync
   }
   ```

4. **Polling Fallback** (`backend/src/services/sync/poller.rs`)
   - For environments where RustShare cannot push webhooks (firewalled RustChat)
   - Cron-like background task every `sync_interval_minutes`
   - Calls `orchestrator.full_sync()` with `If-None-Match` / `modified_since` optimization

5. **Admin UI for Sync Setup**
   - Frontend: add "Sync Sources" tab in AgentManagement or new KnowledgeBase view
   - Form: RustShare URL, API token, folder selector, sync mode (push/pull/both)
   - Show sync status: last sync time, file count, errors

**API Endpoints:**
```
POST   /api/v1/knowledge/sync-sources              → create sync source
GET    /api/v1/knowledge/sync-sources              → list sync sources
GET    /api/v1/knowledge/sync-sources/:id          → get sync source
PUT    /api/v1/knowledge/sync-sources/:id          → update sync source
DELETE /api/v1/knowledge/sync-sources/:id          → delete sync source
POST   /api/v1/knowledge/sync-sources/:id/sync     → trigger manual sync
POST   /api/v1/knowledge/sync/rustshare            → RustShare webhook receiver
```

**Verification:**
- Create RustShare sync source → files appear as documents in KB
- Update file in RustShare → webhook triggers re-index within seconds
- Delete file in RustShare → document removed from KB
- Manual sync button works and shows progress

---

### Slice 6: Frontend Knowledge Base UI

**Objective:** Admin can create knowledge bases, upload documents, assign KBs to agents, and configure sync.

**New Components:**

| Component | File |
|-----------|------|
| `KnowledgeBaseManagement.vue` | `frontend/src/views/admin/KnowledgeBaseManagement.vue` |
| `CreateKnowledgeBaseModal.vue` | `frontend/src/components/modals/CreateKnowledgeBaseModal.vue` |
| `EditKnowledgeBaseModal.vue` | `frontend/src/components/modals/EditKnowledgeBaseModal.vue` |
| `DocumentUploader.vue` | `frontend/src/components/knowledge/DocumentUploader.vue` |
| `DocumentList.vue` | `frontend/src/components/knowledge/DocumentList.vue` |
| `SyncSourceConfig.vue` | `frontend/src/components/knowledge/SyncSourceConfig.vue` |
| `AgentKnowledgeAssignment.vue` | `frontend/src/components/knowledge/AgentKnowledgeAssignment.vue` |

**New Store:**
- `frontend/src/features/knowledge/stores/knowledgeStore.ts` — Pinia store for KB state

**Routes (admin sidebar):**
- `/admin/knowledge-bases` — list all KBs
- `/admin/knowledge-bases/:id` — KB detail (documents, sync, agent assignments)

**Agent edit modal addition:**
- Tab or section: "Knowledge Bases"
- Multi-select list of KBs with `top_k` and `relevance_threshold` per assignment

**Verification:**
- Create KB → appears in list
- Upload document → shows in document list with extraction status
- Assign KB to agent → agent can retrieve chunks
- Configure RustShare sync → shows sync status and last run

---

### Slice 7: Testing, Observability, Cleanup

**Objective:** Ensure the system is production-ready.

**Tests:**
- Unit: chunker logic (edge cases: empty text, single chunk, exact boundary)
- Unit: embedder batching (verify 97 texts → 2 batches of 96 + 1)
- Unit: vector store search (insert 100 chunks, query top-5, verify cosine ordering)
- Integration: full upload → extract → chunk → embed → search pipeline
- Integration: RustShare webhook handler (mock RustShare API)
- E2E: frontend upload flow → agent mention → grounded response

**Observability:**
- Metrics: `rag_documents_total`, `rag_chunks_total`, `rag_indexing_duration_seconds`, `rag_search_duration_seconds`, `rag_sync_duration_seconds`
- Tracing: span around `index_document` and `search` with document_id, kb_id
- Alerts: indexing failure rate > 5%, sync lag > 1 hour

**Cleanup:**
- Remove dead code, unwraps, and TODOs
- Ensure all SQL queries use parameterized inputs (no injection risk)
- Verify `team_id` filtering on every search query
- Update `.env.example` with new config vars:
  ```
  # Optional: local embedding model path (for air-gapped)
  # RUSTCHAT_LOCAL_EMBEDDING_MODEL=/models/bge-small-en-v1.5
  ```

---

## Dependency Graph

```
Slice 1 (DB + models + repo)
    │
    ▼
Slice 2 (Upload + extraction) ──▶ Slice 3 (Chunk + embed + index)
    │                                    │
    ▼                                    ▼
Slice 6 (Frontend UI) ◀────────── Slice 4 (Agent RAG integration)
    │
    ▼
Slice 5 (RustShare sync)
    │
    ▼
Slice 7 (Tests + observability)
```

**Parallelizable:**
- Slice 1 must complete before all others.
- Slice 2 and Slice 6 can start in parallel after Slice 1.
- Slice 3 depends on Slice 2.
- Slice 4 depends on Slice 3.
- Slice 5 depends on Slice 2 and Slice 3.
- Slice 7 is last.

---

## Open Questions / Decisions Needed

1. **RustShare API specifics** — Do we have the actual OpenAPI spec for RustShare? The client implementation depends on endpoint paths, auth scheme, and webhook payload shape.
2. **Embedding model for non-English** — If RustChat teams use German/French/Spanish docs, `text-embedding-3-small` handles multilingual well. Do we need a configurable per-language model?
3. **Max document size** — PDFs can be 100MB+. Do we stream download + extract, or reject >X MB? Suggested: 50MB limit for initial version.
4. **Real-time vs. batch indexing** — Should large RustShare syncs index documents serially or in parallel? Suggested: parallel with semaphore (max 4 concurrent) to avoid rate limits.
5. **Chunk citation in agent responses** — Should agents cite which document/section they used? Suggested: yes, include `[Source: Document Title, Section]` in RAG context so LLM can reference it.

---

## Appendix: RustShare Webhook Payload Schema

```json
{
  "event": "file.created | file.updated | file.deleted",
  "webhook_id": "uuid",
  "timestamp": "2026-06-09T12:00:00Z",
  "folder_id": "uuid",
  "file": {
    "id": "uuid",
    "name": "oauth-setup.md",
    "mime_type": "text/markdown",
    "size_bytes": 4096,
    "etag": "\"abc123\"",
    "modified_at": "2026-06-09T11:59:00Z",
    "download_url": "https://rustshare.example.com/api/v1/files/uuid/download"
  }
}
```
