//! Team model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Team entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Team {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub allow_open_invite: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Team member relationship
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TeamMember {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TeamMemberResponse {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// DTO for creating a team
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTeam {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

/// DTO for adding a member to a team
#[derive(Debug, Clone, Deserialize)]
pub struct AddTeamMember {
    pub user_id: Uuid,
    pub role: Option<String>,
}
