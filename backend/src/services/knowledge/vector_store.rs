//! Vector store implementations
//!
//! Persists and searches knowledge chunk embeddings.

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::knowledge::{KnowledgeChunk, RetrievedChunk, SearchFilter};
use crate::repositories::KnowledgeRepository;
use crate::telemetry::metrics;

/// PostgreSQL + pgvector implementation.
pub struct PgVectorStore {
    pool: PgPool,
}

impl PgVectorStore {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn upsert_chunks(&self, chunks: &[KnowledgeChunk]) -> Result<(), sqlx::Error> {
        let repo = KnowledgeRepository::new(&self.pool);
        repo.insert_chunks(chunks).await
    }

    pub async fn delete_document_chunks(&self, document_id: Uuid) -> Result<(), sqlx::Error> {
        let repo = KnowledgeRepository::new(&self.pool);
        repo.delete_chunks_by_document(document_id).await
    }

    #[tracing::instrument(skip(self), fields(top_k = %top_k, team_id = %filter.team_id))]
    pub async fn search(
        &self,
        embedding: &[f32],
        top_k: usize,
        filter: &SearchFilter,
    ) -> Result<Vec<RetrievedChunk>, sqlx::Error> {
        let start = std::time::Instant::now();
        let repo = KnowledgeRepository::new(&self.pool);
        let result = repo.search_chunks(embedding, top_k as i32, filter).await;
        metrics::RAG_SEARCH_DURATION.observe(start.elapsed().as_secs_f64());
        result
    }
}
