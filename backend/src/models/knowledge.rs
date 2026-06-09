//! Knowledge base models
//!
//! Defines structs for RAG knowledge bases, documents, chunks, sync sources,
//! and agent↔KB mappings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A knowledge base configuration.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KnowledgeBase {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub embedding_model: String,
    pub embedding_dimensions: i32,
    pub chunk_size: i32,
    pub chunk_overlap: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
}

/// Summary of a knowledge base for listing (includes document count).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct KnowledgeBaseSummary {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub embedding_model: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub document_count: Option<i64>,
}

/// A document uploaded to a knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KnowledgeDocument {
    pub id: Uuid,
    pub knowledge_base_id: Uuid,
    pub team_id: Uuid,
    pub title: String,
    pub source_url: Option<String>,
    pub source_type: String,
    pub s3_key: String,
    pub s3_bucket: String,
    pub content_hash: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub extracted_text: Option<String>,
    pub extracted_at: Option<DateTime<Utc>>,
    pub external_id: Option<String>,
    pub external_etag: Option<String>,
    pub external_modified_at: Option<DateTime<Utc>>,
    pub sync_source_id: Option<Uuid>,
    pub is_indexed: bool,
    pub chunk_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
}

/// A single text chunk with its vector embedding.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KnowledgeChunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub knowledge_base_id: Uuid,
    pub team_id: Uuid,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub token_count: Option<i32>,
    #[serde(skip)]
    pub embedding: Option<pgvector::Vector>,
    pub section_title: Option<String>,
    pub start_byte: Option<i32>,
    pub end_byte: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// An external sync source for a knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KnowledgeSyncSource {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub source_type: String,
    #[serde(skip_serializing)]
    pub config_encrypted: String,
    pub sync_mode: String,
    pub sync_interval_minutes: Option<i32>,
    pub is_active: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_status: Option<String>,
    pub last_sync_error: Option<String>,
    pub next_sync_at: Option<DateTime<Utc>>,
    pub document_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Junction table mapping agents to knowledge bases.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentKnowledgeBase {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub knowledge_base_id: Uuid,
    pub top_k: i32,
    pub relevance_threshold: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Detailed view of an agent's knowledge base assignment.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AgentKnowledgeBaseDetail {
    pub agent_id: Uuid,
    pub knowledge_base_id: Uuid,
    pub top_k: i32,
    pub relevance_threshold: Option<f32>,
    pub knowledge_base_name: String,
    pub knowledge_base_description: Option<String>,
}

// ------------------------------------------------------------------
// Request / DTO types
// ------------------------------------------------------------------

/// Request to create a new knowledge base.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateKnowledgeBaseRequest {
    pub name: String,
    pub description: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub chunk_size: Option<i32>,
    pub chunk_overlap: Option<i32>,
}

/// Request to update an existing knowledge base.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateKnowledgeBaseRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub chunk_size: Option<i32>,
    pub chunk_overlap: Option<i32>,
    pub is_active: Option<bool>,
}

/// Request to create a new sync source.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSyncSourceRequest {
    pub name: String,
    pub source_type: String,
    pub config_encrypted: String,
    pub sync_mode: String,
    pub sync_interval_minutes: Option<i32>,
}

/// Request to update an existing sync source.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateSyncSourceRequest {
    pub name: Option<String>,
    pub config_encrypted: Option<String>,
    pub sync_mode: Option<String>,
    pub sync_interval_minutes: Option<i32>,
    pub is_active: Option<bool>,
}

/// Request to assign a knowledge base to an agent.
#[derive(Debug, Clone, Deserialize)]
pub struct AssignKnowledgeBaseRequest {
    pub knowledge_base_id: Uuid,
    pub top_k: Option<i32>,
    pub relevance_threshold: Option<f32>,
}

/// Input for creating a knowledge document.
#[derive(Debug, Clone)]
pub struct CreateKnowledgeDocument {
    pub knowledge_base_id: Uuid,
    pub team_id: Uuid,
    pub title: String,
    pub source_url: Option<String>,
    pub source_type: String,
    pub s3_key: String,
    pub s3_bucket: String,
    pub content_hash: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub extracted_text: Option<String>,
    pub external_id: Option<String>,
    pub external_etag: Option<String>,
    pub external_modified_at: Option<DateTime<Utc>>,
    pub sync_source_id: Option<Uuid>,
    pub created_by: Uuid,
}

/// Filter for chunk search queries.
#[derive(Debug, Clone)]
pub struct SearchFilter {
    pub team_id: Uuid,
    pub knowledge_base_id: Option<Uuid>,
    pub document_id: Option<Uuid>,
}

/// A chunk retrieved from semantic search.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RetrievedChunk {
    pub chunk_text: String,
    pub document_title: String,
    pub document_source_url: Option<String>,
    pub section_title: Option<String>,
    pub similarity: f32,
}

/// Update payload for sync source state.
#[derive(Debug, Clone)]
pub struct SyncStateUpdate {
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_status: Option<String>,
    pub last_sync_error: Option<String>,
    pub next_sync_at: Option<DateTime<Utc>>,
    pub document_count: Option<i32>,
}
