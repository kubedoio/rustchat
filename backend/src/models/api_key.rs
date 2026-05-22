//! API Key model for non-human entities
//!
//! Manages API keys used by agents, services, and CI systems to authenticate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// API Key for non-human entity authentication
///
/// Stored in database with argon2id hash. The plain-text key is only shown
/// once during creation and never stored.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub key_hash: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApiKey {
    /// Create a new API key instance
    pub fn new(user_id: Uuid, key_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            key_hash,
            name: None,
            description: None,
            expires_at: None,
            last_used_at: None,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set a descriptive name for this API key (builder pattern)
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set a description for this API key (builder pattern)
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set an expiry date for this API key (builder pattern)
    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if this API key has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| exp < Utc::now()).unwrap_or(false)
    }

    /// Check if this API key is valid (active and not expired)
    pub fn is_valid(&self) -> bool {
        self.is_active && !self.is_expired()
    }
}

/// Response DTO for API key creation (includes plain-text key once)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCreationResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Plain-text API key - only shown once during creation
    pub key: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Request DTO for creating a new API key
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub expires_in_days: Option<i64>,
}
