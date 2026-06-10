//! Knowledge base repository
//!
//! Provides CRUD operations for knowledge_bases, knowledge_documents,
//! knowledge_chunks, knowledge_sync_sources, and agent_knowledge_bases.
//! All operations use compile-time checked sqlx queries.

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::knowledge::*;

pub struct KnowledgeRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> KnowledgeRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Knowledge Bases
    // ------------------------------------------------------------------

    /// Create a new knowledge base.
    #[tracing::instrument(skip(self), fields(team_id = %team_id, name = %name))]
    pub async fn create_knowledge_base(
        &self,
        team_id: Uuid,
        name: &str,
        description: Option<&str>,
        embedding_model: &str,
        embedding_dimensions: i32,
        chunk_size: i32,
        chunk_overlap: i32,
        created_by: Uuid,
    ) -> Result<KnowledgeBase, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeBase>(
            r#"
            INSERT INTO knowledge_bases (
                team_id, name, description, embedding_model,
                embedding_dimensions, chunk_size, chunk_overlap, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(name)
        .bind(description)
        .bind(embedding_model)
        .bind(embedding_dimensions)
        .bind(chunk_size)
        .bind(chunk_overlap)
        .bind(created_by)
        .fetch_one(self.pool)
        .await
    }

    /// List all knowledge bases for a team with document counts.
    pub async fn list_knowledge_bases(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<KnowledgeBaseSummary>, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeBaseSummary>(
            r#"
            SELECT
                kb.id,
                kb.team_id,
                kb.name,
                kb.description,
                kb.embedding_model,
                kb.is_active,
                kb.created_at,
                kb.updated_at,
                COUNT(kd.id) as document_count
            FROM knowledge_bases kb
            LEFT JOIN knowledge_documents kd ON kd.knowledge_base_id = kb.id
            WHERE kb.team_id = $1
            GROUP BY kb.id
            ORDER BY kb.created_at DESC
            "#,
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await
    }

    /// Get a knowledge base by ID and team.
    pub async fn get_knowledge_base(
        &self,
        id: Uuid,
        team_id: Uuid,
    ) -> Result<Option<KnowledgeBase>, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeBase>(
            "SELECT * FROM knowledge_bases WHERE id = $1 AND team_id = $2",
        )
        .bind(id)
        .bind(team_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Update a knowledge base. Only updates fields that are Some.
    #[tracing::instrument(skip(self), fields(id = %id))]
    pub async fn update_knowledge_base(
        &self,
        id: Uuid,
        team_id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
        embedding_model: Option<&str>,
        embedding_dimensions: Option<i32>,
        chunk_size: Option<i32>,
        chunk_overlap: Option<i32>,
        is_active: Option<bool>,
    ) -> Result<KnowledgeBase, sqlx::Error> {
        let existing = self
            .get_knowledge_base(id, team_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        let name = name.unwrap_or(&existing.name);
        let description = match description {
            Some(d) => d,
            None => existing.description.as_deref(),
        };
        let embedding_model = embedding_model.unwrap_or(&existing.embedding_model);
        let embedding_dimensions = embedding_dimensions.unwrap_or(existing.embedding_dimensions);
        let chunk_size = chunk_size.unwrap_or(existing.chunk_size);
        let chunk_overlap = chunk_overlap.unwrap_or(existing.chunk_overlap);
        let is_active = is_active.unwrap_or(existing.is_active);

        sqlx::query_as::<_, KnowledgeBase>(
            r#"
            UPDATE knowledge_bases
            SET
                name = $1,
                description = $2,
                embedding_model = $3,
                embedding_dimensions = $4,
                chunk_size = $5,
                chunk_overlap = $6,
                is_active = $7
            WHERE id = $8 AND team_id = $9
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(embedding_model)
        .bind(embedding_dimensions)
        .bind(chunk_size)
        .bind(chunk_overlap)
        .bind(is_active)
        .bind(id)
        .bind(team_id)
        .fetch_one(self.pool)
        .await
    }

    /// Delete a knowledge base by ID and team.
    #[tracing::instrument(skip(self), fields(id = %id))]
    pub async fn delete_knowledge_base(&self, id: Uuid, team_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM knowledge_bases WHERE id = $1 AND team_id = $2",
        )
        .bind(id)
        .bind(team_id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    // ------------------------------------------------------------------
    // Documents
    // ------------------------------------------------------------------

    /// Create a new knowledge document.
    #[tracing::instrument(skip(self), fields(title = %doc.title))]
    pub async fn create_document(
        &self,
        doc: &CreateKnowledgeDocument,
    ) -> Result<KnowledgeDocument, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeDocument>(
            r#"
            INSERT INTO knowledge_documents (
                id, knowledge_base_id, team_id, title, source_url, source_type,
                s3_key, s3_bucket, content_hash, mime_type, size_bytes,
                extracted_text, external_id, external_etag, external_modified_at,
                sync_source_id, created_by
            )
            VALUES (COALESCE($1, uuid_generate_v4()), $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING *
            "#,
        )
        .bind(doc.id)
        .bind(doc.knowledge_base_id)
        .bind(doc.team_id)
        .bind(&doc.title)
        .bind(&doc.source_url)
        .bind(&doc.source_type)
        .bind(&doc.s3_key)
        .bind(&doc.s3_bucket)
        .bind(&doc.content_hash)
        .bind(&doc.mime_type)
        .bind(doc.size_bytes)
        .bind(&doc.extracted_text)
        .bind(&doc.external_id)
        .bind(&doc.external_etag)
        .bind(doc.external_modified_at)
        .bind(doc.sync_source_id)
        .bind(doc.created_by)
        .fetch_one(self.pool)
        .await
    }

    /// Get a document by its content hash and team.
    pub async fn get_document_by_hash(
        &self,
        content_hash: &str,
        team_id: Uuid,
    ) -> Result<Option<KnowledgeDocument>, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeDocument>(
            "SELECT * FROM knowledge_documents WHERE content_hash = $1 AND team_id = $2",
        )
        .bind(content_hash)
        .bind(team_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Get a document by ID and team.
    pub async fn get_document(
        &self,
        id: Uuid,
        team_id: Uuid,
    ) -> Result<Option<KnowledgeDocument>, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeDocument>(
            "SELECT * FROM knowledge_documents WHERE id = $1 AND team_id = $2",
        )
        .bind(id)
        .bind(team_id)
        .fetch_optional(self.pool)
        .await
    }

    /// List all documents in a knowledge base for a team.
    pub async fn list_documents(
        &self,
        knowledge_base_id: Uuid,
        team_id: Uuid,
    ) -> Result<Vec<KnowledgeDocument>, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeDocument>(
            r#"
            SELECT * FROM knowledge_documents
            WHERE knowledge_base_id = $1 AND team_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(knowledge_base_id)
        .bind(team_id)
        .fetch_all(self.pool)
        .await
    }

    /// Mark a document as indexed and set its chunk count.
    #[tracing::instrument(skip(self), fields(id = %id))]
    pub async fn update_document_indexed(
        &self,
        id: Uuid,
        team_id: Uuid,
        chunk_count: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE knowledge_documents
            SET is_indexed = TRUE, chunk_count = $1
            WHERE id = $2 AND team_id = $3
            "#,
        )
        .bind(chunk_count)
        .bind(id)
        .bind(team_id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    /// Update a document's extracted text.
    #[tracing::instrument(skip(self), fields(id = %id))]
    pub async fn update_document_extracted(
        &self,
        id: Uuid,
        team_id: Uuid,
        extracted_text: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE knowledge_documents
            SET extracted_text = $1, extracted_at = NOW()
            WHERE id = $2 AND team_id = $3
            "#,
        )
        .bind(extracted_text)
        .bind(id)
        .bind(team_id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    /// Delete a document by ID and team.
    #[tracing::instrument(skip(self), fields(id = %id))]
    pub async fn delete_document(&self, id: Uuid, team_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM knowledge_documents WHERE id = $1 AND team_id = $2",
        )
        .bind(id)
        .bind(team_id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    // ------------------------------------------------------------------
    // Chunks + Search
    // ------------------------------------------------------------------

    /// Insert multiple chunks within a transaction.
    pub async fn insert_chunks(
        &self,
        chunks: &[KnowledgeChunk],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        for chunk in chunks {
            sqlx::query(
                r#"
                INSERT INTO knowledge_chunks (
                    document_id, knowledge_base_id, team_id, chunk_index,
                    chunk_text, token_count, embedding, section_title,
                    start_byte, end_byte
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(chunk.document_id)
            .bind(chunk.knowledge_base_id)
            .bind(chunk.team_id)
            .bind(chunk.chunk_index)
            .bind(&chunk.chunk_text)
            .bind(chunk.token_count)
            .bind(&chunk.embedding)
            .bind(&chunk.section_title)
            .bind(chunk.start_byte)
            .bind(chunk.end_byte)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await
    }

    /// Search for similar chunks using pgvector cosine distance.
    /// Returns chunks ordered by similarity (highest first).
    pub async fn search_chunks(
        &self,
        embedding: &[f32],
        top_k: i32,
        filter: &SearchFilter,
    ) -> Result<Vec<RetrievedChunk>, sqlx::Error> {
        let vector = pgvector::Vector::from(embedding.to_vec());

        sqlx::query_as::<_, RetrievedChunk>(
            r#"
            SELECT
                kc.chunk_text,
                kd.title as document_title,
                kd.source_url as document_source_url,
                kc.section_title,
                1 - (kc.embedding <=> $1) as similarity
            FROM knowledge_chunks kc
            JOIN knowledge_documents kd ON kd.id = kc.document_id
            WHERE kc.team_id = $2
              AND ($3::uuid IS NULL OR kc.knowledge_base_id = $3)
              AND ($4::uuid IS NULL OR kc.document_id = $4)
              AND kc.embedding IS NOT NULL
            ORDER BY kc.embedding <=> $1
            LIMIT $5
            "#,
        )
        .bind(vector)
        .bind(filter.team_id)
        .bind(filter.knowledge_base_id)
        .bind(filter.document_id)
        .bind(top_k)
        .fetch_all(self.pool)
        .await
    }

    /// Hybrid search: semantic + full-text with RRF fusion.
    pub async fn search_chunks_hybrid(
        &self,
        query_text: &str,
        query_embedding: &[f32],
        top_k: i32,
        filter: &SearchFilter,
    ) -> Result<Vec<RetrievedChunk>, sqlx::Error> {
        use crate::services::knowledge::hybrid_search::{rrf_fuse, RRF_K};

        // Semantic search (top 10)
        let semantic_results = self.search_chunks(query_embedding, 10, filter).await?;

        // Full-text search via placeholder ILIKE query
        // (Meilisearch integration can be wired here later)
        let text_results = sqlx::query_as::<_, RetrievedChunk>(
            r#"
            SELECT
                kc.chunk_text,
                kd.title as document_title,
                kd.source_url as document_source_url,
                kc.section_title,
                0.0 as similarity
            FROM knowledge_chunks kc
            JOIN knowledge_documents kd ON kd.id = kc.document_id
            WHERE kc.team_id = $1
              AND ($2::uuid IS NULL OR kc.knowledge_base_id = $2)
              AND kc.chunk_text ILIKE $3
            LIMIT 10
            "#,
        )
        .bind(filter.team_id)
        .bind(filter.knowledge_base_id)
        .bind(format!("%{}%", query_text))
        .fetch_all(self.pool)
        .await?;

        // Fuse results
        let fused = rrf_fuse(semantic_results, text_results, RRF_K);

        // Return top_k
        Ok(fused.into_iter().take(top_k as usize).map(|h| h.chunk).collect())
    }

    /// Delete all chunks belonging to a document.
    #[tracing::instrument(skip(self), fields(document_id = %document_id))]
    pub async fn delete_chunks_by_document(
        &self,
        document_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM knowledge_chunks WHERE document_id = $1")
            .bind(document_id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    // ------------------------------------------------------------------
    // Agent ↔ KB Mapping
    // ------------------------------------------------------------------

    /// Assign a knowledge base to an agent.
    #[tracing::instrument(skip(self), fields(agent_id = %agent_id, kb_id = %kb_id))]
    pub async fn assign_kb_to_agent(
        &self,
        agent_id: Uuid,
        kb_id: Uuid,
        top_k: i32,
        relevance_threshold: Option<f32>,
    ) -> Result<AgentKnowledgeBase, sqlx::Error> {
        sqlx::query_as::<_, AgentKnowledgeBase>(
            r#"
            INSERT INTO agent_knowledge_bases (agent_id, knowledge_base_id, top_k, relevance_threshold)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (agent_id, knowledge_base_id)
            DO UPDATE SET
                top_k = EXCLUDED.top_k,
                relevance_threshold = EXCLUDED.relevance_threshold,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(agent_id)
        .bind(kb_id)
        .bind(top_k)
        .bind(relevance_threshold)
        .fetch_one(self.pool)
        .await
    }

    /// List all knowledge bases assigned to an agent.
    pub async fn list_agent_knowledge_bases(
        &self,
        agent_id: Uuid,
    ) -> Result<Vec<AgentKnowledgeBaseDetail>, sqlx::Error> {
        sqlx::query_as::<_, AgentKnowledgeBaseDetail>(
            r#"
            SELECT
                akb.agent_id,
                akb.knowledge_base_id,
                akb.top_k,
                akb.relevance_threshold,
                kb.name as knowledge_base_name,
                kb.description as knowledge_base_description,
                kb.team_id as team_id
            FROM agent_knowledge_bases akb
            JOIN knowledge_bases kb ON kb.id = akb.knowledge_base_id
            WHERE akb.agent_id = $1
            ORDER BY akb.created_at DESC
            "#,
        )
        .bind(agent_id)
        .fetch_all(self.pool)
        .await
    }

    /// Remove a knowledge base assignment from an agent.
    #[tracing::instrument(skip(self), fields(agent_id = %agent_id, kb_id = %kb_id))]
    pub async fn unassign_kb_from_agent(
        &self,
        agent_id: Uuid,
        kb_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM agent_knowledge_bases WHERE agent_id = $1 AND knowledge_base_id = $2",
        )
        .bind(agent_id)
        .bind(kb_id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    // ------------------------------------------------------------------
    // Sync Sources
    // ------------------------------------------------------------------

    /// Create a new sync source.
    #[tracing::instrument(skip(self, config_encrypted), fields(team_id = %team_id, name = %name))]
    pub async fn create_sync_source(
        &self,
        team_id: Uuid,
        name: &str,
        source_type: &str,
        config_encrypted: &str,
        sync_mode: &str,
        sync_interval_minutes: Option<i32>,
    ) -> Result<KnowledgeSyncSource, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeSyncSource>(
            r#"
            INSERT INTO knowledge_sync_sources (
                team_id, name, source_type, config_encrypted,
                sync_mode, sync_interval_minutes
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(name)
        .bind(source_type)
        .bind(config_encrypted)
        .bind(sync_mode)
        .bind(sync_interval_minutes)
        .fetch_one(self.pool)
        .await
    }

    /// List all sync sources for a team.
    pub async fn list_sync_sources(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<KnowledgeSyncSource>, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeSyncSource>(
            r#"
            SELECT * FROM knowledge_sync_sources
            WHERE team_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await
    }

    /// Get a sync source by ID and team.
    pub async fn get_sync_source(
        &self,
        id: Uuid,
        team_id: Uuid,
    ) -> Result<Option<KnowledgeSyncSource>, sqlx::Error> {
        sqlx::query_as::<_, KnowledgeSyncSource>(
            "SELECT * FROM knowledge_sync_sources WHERE id = $1 AND team_id = $2",
        )
        .bind(id)
        .bind(team_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Update a sync source. Only updates fields that are Some.
    #[tracing::instrument(skip(self), fields(id = %id))]
    pub async fn update_sync_source(
        &self,
        id: Uuid,
        team_id: Uuid,
        name: Option<&str>,
        config_encrypted: Option<&str>,
        sync_mode: Option<&str>,
        sync_interval_minutes: Option<i32>,
        is_active: Option<bool>,
    ) -> Result<KnowledgeSyncSource, sqlx::Error> {
        let existing = self
            .get_sync_source(id, team_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        let name = name.unwrap_or(&existing.name);
        let config_encrypted = config_encrypted.unwrap_or(&existing.config_encrypted);
        let sync_mode = sync_mode.unwrap_or(&existing.sync_mode);
        let sync_interval_minutes = match sync_interval_minutes {
            Some(v) => Some(v),
            None => existing.sync_interval_minutes,
        };
        let is_active = is_active.unwrap_or(existing.is_active);

        sqlx::query_as::<_, KnowledgeSyncSource>(
            r#"
            UPDATE knowledge_sync_sources
            SET
                name = $1,
                config_encrypted = $2,
                sync_mode = $3,
                sync_interval_minutes = $4,
                is_active = $5
            WHERE id = $6 AND team_id = $7
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(config_encrypted)
        .bind(sync_mode)
        .bind(sync_interval_minutes)
        .bind(is_active)
        .bind(id)
        .bind(team_id)
        .fetch_one(self.pool)
        .await
    }

    /// Update the sync state (last sync, errors, etc.) for a sync source.
    #[tracing::instrument(skip(self, state), fields(id = %id))]
    pub async fn update_sync_state(
        &self,
        id: Uuid,
        team_id: Uuid,
        state: &SyncStateUpdate,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE knowledge_sync_sources
            SET
                last_sync_at = COALESCE($1, last_sync_at),
                last_sync_status = COALESCE($2, last_sync_status),
                last_sync_error = COALESCE($3, last_sync_error),
                next_sync_at = COALESCE($4, next_sync_at),
                document_count = COALESCE($5, document_count)
            WHERE id = $6 AND team_id = $7
            "#,
        )
        .bind(state.last_sync_at)
        .bind(&state.last_sync_status)
        .bind(&state.last_sync_error)
        .bind(state.next_sync_at)
        .bind(state.document_count)
        .bind(id)
        .bind(team_id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    /// Delete a sync source by ID and team.
    #[tracing::instrument(skip(self), fields(id = %id))]
    pub async fn delete_sync_source(&self, id: Uuid, team_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM knowledge_sync_sources WHERE id = $1 AND team_id = $2",
        )
        .bind(id)
        .bind(team_id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }
}
