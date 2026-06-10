# ADR-002: RAG Knowledge Base with External Sync

**Date:** 2026-06-09
**Status:** Proposed
**Risk tier:** architectural
**Issue:** [#177](https://github.com/kubedoio/rustchat/issues/177)
**Depends on:** ADR-001 (AI Agents as Channel Participants)

## Context

Phase 1 of the AI Agents feature gave agents identity, memory, and LLM connectivity. Agents can now respond to mentions with conversational context. However, agents have no access to structured domain knowledge — product documentation, runbooks, design specs, meeting notes, or shared files. Without this, every agent response is limited to what the LLM was trained on plus the current conversation thread.

The requirement is a **Retrieval-Augmented Generation (RAG)** knowledge base that:
1. Stores documents (uploaded directly or synced from external sources)
2. Chunks documents into semantically coherent fragments
3. Generates embeddings for each chunk
4. Retrieves the top-k most relevant chunks at agent query time
5. Injects those chunks into the agent's LLM prompt as grounded context

**Critical constraint:** External document sync — specifically from **RustShare** (our companion file-sharing product) — is not a nice-to-have; it is the primary ingestion path. Most valuable knowledge already lives in RustShare folders, not as fresh uploads into RustChat.

## Decision

We will build a **RAG pipeline** with three subsystems:

### 1. Storage: pgvector + S3 (Unified with Existing Infrastructure)

Documents live in **S3/RustFS** (same bucket as file uploads). Embeddings and metadata live in **PostgreSQL with the `pgvector` extension**.

**Why pgvector over a dedicated vector DB:**
- Zero new infrastructure — PostgreSQL is already required, operationalized, and replicated.
- ACID co-location — document metadata, chunk text, and embeddings are transactionally consistent with user/team data.
- Sufficient headroom — HNSW indexes give sub-10ms ANN search at millions of 1536-dim vectors.
- Clean migration path — if we outgrow pgvector, the `VectorStore` trait (see §3) allows swapping to Qdrant/Pinecone without touching chunking or embedding logic.

**Why S3 for blob storage:**
- Reuses existing `StorageService` (`rustfs` / S3 client) with encryption-at-rest and presigned URLs.
- No file size limits in PG (blobs stay out of the database).
- RustShare sync is naturally S3-to-S3 if both products share the same object store.

### 2. Ingestion: Chunker + Embedder Pipeline

```
Document (upload or sync)
    │
    ▼
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  Extractor  │───▶ │   Chunker    │───▶ │  Embedder   │
│ (pdf, md,   │     │ (semantic/   │     │ (OpenAI or  │
│  txt, html) │     │  sliding)    │     │  local)     │
└─────────────┘     └──────────────┘     └─────────────┘
    │                                           │
    ▼                                           ▼
S3: {team_id}/{kb_id}/{doc_id}/raw        PG: knowledge_chunks
                                              (chunk_text, embedding)
```

**Chunking strategy:**
- **Markdown-aware splitting** — split on headers (`#`, `##`) when possible; preserves document hierarchy.
- **Code-aware splitting** — split on function/class boundaries when `mime_type` indicates code.
- **Sliding window fallback** — 512-token chunks, 50-token overlap, for unstructured text.
- **Max chunk size:** 1024 tokens (leaves room for system prompt + conversation context within LLM context window).

**Embedding model:**
- Default: `text-embedding-3-small` (1536-dim, cheap, fast, excellent quality).
- Override per-knowledge-base via `embedding_model` column.
- Local fallback: `fastembed-rs` with `BAAI/bge-small-en-v1.5` for air-gapped deployments.

### 3. Retrieval: `VectorStore` Trait

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert_chunks(&self, chunks: &[KnowledgeChunk]) -> ApiResult<()>;
    async fn search(&self, query_embedding: &[f32], top_k: usize, filter: &SearchFilter) -> ApiResult<Vec<RetrievedChunk>>;
    async fn delete_document(&self, document_id: Uuid) -> ApiResult<()>;
}
```

**Implementations:**
- `PgVectorStore` — production default.
- `QdrantStore` — future scale-out option (behind feature flag).

**Search filter:**
- `team_id` — mandatory multi-tenant isolation.
- `knowledge_base_id` — optional scope to a specific KB.
- `document_id` — optional single-document retrieval.

### 4. External Sync: RustShare Integration (First-Class)

RustShare sync is **not** a generic webhook bolt-on. It is a dedicated sync subsystem.

```
RustShare Instance
       │
       │  OAuth 2.0 / Service Account
       ▼
┌─────────────────────────┐
│  rustshare_sync module  │
│  ├─ list folders        │
│  ├─ poll / webhook      │
│  ├─ download delta      │
│  └─ dedup by hash       │
└─────────────────────────┘
       │
       ▼
┌─────────────────────────┐
│  Ingestion Pipeline     │
│  (same as manual upload)│
└─────────────────────────┘
```

**RustShare sync specifics:**
- **Authentication:** OAuth 2.0 service account or shared API key (configured per-sync-source).
- **Polling strategy:** Incremental sync via `If-None-Match` / `modified_since` on RustShare's folder listing API.
- **Webhook push (preferred):** RustShare sends `document.created`, `document.updated`, `document.deleted` events to RustChat's `/api/v1/knowledge/sync/rustshare` endpoint.
- **Deduplication:** SHA-256 of file content. If a file is unchanged, skip re-chunking and re-embedding.
- **Folder mapping:** Admin maps a RustShare folder (`/Product/Specs/`) to a RustChat knowledge base. Sub-folder recursion is configurable.
- **Access control mirror:** If RustShare enforces read access per-folder, RustChat stores the same ACL hash and refreshes it on sync.

**Generic sync framework:**
- The RustShare sync is built on a `SyncSource` trait so Confluence, Notion, GitHub, or generic S3 buckets can be added later without rewriting the pipeline.

## Schema

### `knowledge_bases`

```sql
CREATE TABLE knowledge_bases (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    description TEXT,
    -- Embedding config
    embedding_model VARCHAR(64) NOT NULL DEFAULT 'text-embedding-3-small',
    embedding_dimensions INT NOT NULL DEFAULT 1536,
    -- Chunking config
    chunk_size INT NOT NULL DEFAULT 512,
    chunk_overlap INT NOT NULL DEFAULT 50,
    -- State
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id)
);
```

### `knowledge_documents`

```sql
CREATE TABLE knowledge_documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    knowledge_base_id UUID NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    -- Identity
    title VARCHAR(512) NOT NULL,
    source_url TEXT,                          -- original URL (RustShare, web, etc.)
    source_type VARCHAR(32) NOT NULL DEFAULT 'upload',  -- upload, rustshare, confluence, github, s3
    -- Storage
    s3_key TEXT NOT NULL,                     -- path in S3/RustFS
    s3_bucket TEXT NOT NULL,                  -- bucket name
    content_hash TEXT NOT NULL,               -- SHA-256 for dedup
    mime_type VARCHAR(128) NOT NULL,
    size_bytes BIGINT NOT NULL,
    -- Content extraction (cached)
    extracted_text TEXT,                      -- plain text extracted from PDF/DOCX/HTML
    extracted_at TIMESTAMPTZ,
    -- Sync metadata (for external sources)
    external_id TEXT,                         -- RustShare file ID, Confluence page ID, etc.
    external_etag TEXT,                       -- etag for conditional sync
    external_modified_at TIMESTAMPTZ,         -- source modification time
    sync_source_id UUID REFERENCES knowledge_sync_sources(id),
    -- State
    is_indexed BOOLEAN NOT NULL DEFAULT FALSE,
    chunk_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### `knowledge_chunks`

```sql
CREATE TABLE knowledge_chunks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES knowledge_documents(id) ON DELETE CASCADE,
    knowledge_base_id UUID NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    -- Content
    chunk_index INT NOT NULL,
    chunk_text TEXT NOT NULL,
    token_count INT,
    -- Embedding (pgvector)
    embedding VECTOR(1536),                   -- dimension matches knowledge_bases.embedding_dimensions
    -- Context for citation
    section_title TEXT,                       -- e.g., "## Authentication"
    start_byte INT,
    end_byte INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_knowledge_chunks_document ON knowledge_chunks(document_id);
CREATE INDEX idx_knowledge_chunks_kb ON knowledge_chunks(knowledge_base_id);
CREATE INDEX idx_knowledge_chunks_team ON knowledge_chunks(team_id);

-- HNSW index for fast similarity search
CREATE INDEX idx_knowledge_chunks_embedding_hnsw
ON knowledge_chunks
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);
```

### `knowledge_sync_sources`

```sql
CREATE TABLE knowledge_sync_sources (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    source_type VARCHAR(32) NOT NULL,         -- rustshare, confluence, github, s3, webhook
    -- Connection config (encrypted)
    config_encrypted TEXT NOT NULL,           -- JSON blob encrypted with RUSTCHAT_ENCRYPTION_KEY
    -- Sync behavior
    sync_mode VARCHAR(16) NOT NULL DEFAULT 'push',  -- push (webhook), pull (polling), bidirectional
    sync_interval_minutes INT,                -- for pull mode
    last_sync_at TIMESTAMPTZ,
    last_sync_status VARCHAR(16),             -- success, partial, failed
    last_sync_error TEXT,
    -- State
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### `agent_knowledge_bases` (Many-to-Many)

```sql
CREATE TABLE agent_knowledge_bases (
    agent_id UUID NOT NULL REFERENCES agent_configs(user_id) ON DELETE CASCADE,
    knowledge_base_id UUID NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,
    -- Retrieval config per assignment
    top_k INT NOT NULL DEFAULT 5,
    relevance_threshold REAL,                 -- minimum cosine similarity (0.0–1.0)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (agent_id, knowledge_base_id)
);
```

## Consequences

### Positive

- **Grounded responses** — Agents cite actual documents, reducing hallucination.
- **No new infrastructure** — pgvector extends existing PostgreSQL; S3 is already operational.
- **RustShare sync is native** — Users don't manually re-upload files; knowledge stays in sync.
- **Clean abstraction** — `VectorStore` trait allows future migration to Qdrant without rewriting the pipeline.
- **Multi-tenant by design** — Every query is filtered by `team_id`; no cross-team leakage.

### Negative / Risks

- **pgvector load** — High-volume embedding insertions (batch RustShare syncs) can spike PG I/O. Mitigation: batch inserts, async background workers, and the `VectorStore` escape hatch.
- **Embedding cost** — Large RustShare folders with thousands of files generate significant OpenAI API usage. Mitigation: dedup by content hash, incremental sync, and local embedding fallback.
- **Eventual consistency** — Webhook → ingestion → embedding → index availability is asynchronous (seconds to minutes). Agents may respond without the very latest document version.
- **pgvector extension availability** — Not all managed PostgreSQL providers offer pgvector. Mitigation: document requirement; provide `pgvector` in docker-compose; Qdrant fallback for non-pgvector environments.

## Alternatives Considered

| Alternative | Why Rejected |
|-------------|-------------|
| **Qdrant as primary** | Adds a new service to operate, monitor, and backup. Overkill until we hit >10M vectors. |
| **Pinecone** | Vendor lock-in; per-query cost scales unpredictably; data leaves premises. |
| **Elasticsearch vector search** | Heavy JVM dependency; overkill when we already have Meilisearch for full-text. |
| **Store blobs in PostgreSQL (BYTEA)** | Bloats the database; S3 is purpose-built for object storage. |
| **In-process embedding (always local)** | Quality gap vs. OpenAI embeddings for non-English and technical content; CPU-intensive. |

## Related

- ADR-001: AI Agents as Channel Participants
- `docs/adr/ADR-002-rag-knowledge-base-implementation-spec.md` — Phase 2 slice breakdown
