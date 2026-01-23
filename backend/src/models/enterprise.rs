//! Enterprise models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Audit log entry
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_ip: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// SSO configuration
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SsoConfig {
    pub id: Uuid,
    pub org_id: Uuid,
    pub provider: String,
    pub display_name: Option<String>,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    #[serde(skip_serializing)]
    pub client_secret_encrypted: Option<String>,
    pub scopes: Vec<String>,
    pub idp_metadata_url: Option<String>,
    pub idp_entity_id: Option<String>,
    pub is_active: bool,
    pub auto_provision: bool,
    pub default_role: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Retention policy
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RetentionPolicy {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub retention_days: i32,
    pub delete_files: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Permission definition
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Permission {
    pub id: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

/// DTOs
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAuditLog {
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSsoConfig {
    pub provider: String,
    pub display_name: Option<String>,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRetentionPolicy {
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub retention_days: i32,
    #[serde(default)]
    pub delete_files: bool,
}

/// Audit log query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogQuery {
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub actor_user_id: Option<Uuid>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
