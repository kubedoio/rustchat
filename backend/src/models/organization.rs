//! Organization model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Organization entity
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating an organization
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrganization {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}
