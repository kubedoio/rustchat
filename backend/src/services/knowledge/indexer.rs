//! Document indexing service
//!
//! Orchestrates the pipeline: extracted text → chunks → embeddings → vector store.

use std::sync::Arc;
use uuid::Uuid;

use crate::models::knowledge::KnowledgeChunk;
use crate::repositories::KnowledgeRepository;
use crate::services::knowledge::chunker::{select_chunker, ChunkConfig};
use crate::services::knowledge::embedder::Embedder;
use crate::services::knowledge::vector_store::VectorStore;

pub struct IndexerService {
    pool: sqlx::PgPool,
    embedder: Arc<dyn Embedder>,
    vector_store: Arc<dyn VectorStore>,
}

impl IndexerService {
    pub fn new(
        pool: sqlx::PgPool,
        embedder: Arc<dyn Embedder>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Self {
        Self {
            pool,
            embedder,
            vector_store,
        }
    }

    /// Index a single document: chunk, embed, and store.
    pub async fn index_document(
        &self,
        document_id: Uuid,
        team_id: Uuid,
    ) -> Result<(), IndexerError> {
        let repository = KnowledgeRepository::new(&self.pool);

        // 1. Load document
        let doc = repository
            .get_document(document_id, team_id)
            .await
            .map_err(IndexerError::Database)?
            .ok_or(IndexerError::DocumentNotFound(document_id))?;

        let text = doc
            .extracted_text
            .ok_or(IndexerError::NoExtractedText(document_id))?;
        if text.trim().is_empty() {
            return Err(IndexerError::NoExtractedText(document_id));
        }

        // 2. Load knowledge base for chunking config
        let kb = repository
            .get_knowledge_base(doc.knowledge_base_id, doc.team_id)
            .await
            .map_err(IndexerError::Database)?
            .ok_or(IndexerError::KnowledgeBaseNotFound(doc.knowledge_base_id))?;

        // 3. Delete old chunks if re-indexing
        self.vector_store
            .delete_document_chunks(document_id)
            .await
            .map_err(IndexerError::Database)?;

        // 4. Chunk text
        let chunker = select_chunker(&doc.mime_type);
        let config = ChunkConfig {
            chunk_size: kb.chunk_size as usize,
            chunk_overlap: kb.chunk_overlap as usize,
        };
        let chunks = chunker.chunk(&text, &config);

        // 5. Embed chunks in batches
        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let embeddings = self.embedder.embed(&texts).await?;

        // 6. Build KnowledgeChunk structs
        let mut knowledge_chunks = Vec::new();
        for (i, (chunk, embedding)) in chunks.iter().zip(embeddings.iter()).enumerate() {
            knowledge_chunks.push(KnowledgeChunk {
                id: Uuid::new_v4(),
                document_id,
                knowledge_base_id: doc.knowledge_base_id,
                team_id: doc.team_id,
                chunk_index: i as i32,
                chunk_text: chunk.text.clone(),
                token_count: Some(chunk.token_count as i32),
                embedding: Some(pgvector::Vector::from(embedding.clone())),
                section_title: chunk.section_title.clone(),
                start_byte: Some(chunk.start_byte as i32),
                end_byte: Some(chunk.end_byte as i32),
                created_at: chrono::Utc::now(),
            });
        }

        // 7. Insert chunks into vector store
        self.vector_store
            .upsert_chunks(&knowledge_chunks)
            .await
            .map_err(IndexerError::Database)?;

        // 8. Mark document as indexed
        repository
            .update_document_indexed(document_id, doc.team_id, knowledge_chunks.len() as i32)
            .await
            .map_err(IndexerError::Database)?;

        tracing::info!(
            document_id = %document_id,
            chunk_count = knowledge_chunks.len(),
            "Document indexed successfully"
        );

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("Document not found: {0}")]
    DocumentNotFound(Uuid),
    #[error("Knowledge base not found: {0}")]
    KnowledgeBaseNotFound(Uuid),
    #[error("Document has no extracted text: {0}")]
    NoExtractedText(Uuid),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Embedding error: {0}")]
    Embed(#[from] crate::services::knowledge::embedder::EmbedError),
}
