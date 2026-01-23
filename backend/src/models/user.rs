//! User model and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User roles for RBAC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    SystemAdmin,
    OrgAdmin,
    TeamAdmin,
    Member,
    Guest,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::Member
    }
}

/// User entity from database
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_bot: bool,
    pub is_active: bool,
    pub role: String,
    pub presence: String, // 'online', 'away', 'dnd', 'offline'
    pub custom_status: Option<serde_json::Value>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public user response (without sensitive fields)
#[derive(Debug, Clone, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_bot: bool,
    pub role: String,
    pub presence: String,
    pub custom_status: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            org_id: user.org_id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            is_bot: user.is_bot,
            role: user.role,
            presence: user.presence,
            custom_status: user.custom_status,
            created_at: user.created_at,
        }
    }
}

/// DTO for creating a new user
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    pub org_id: Option<Uuid>,
}

/// DTO for updating a user
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub custom_status: Option<serde_json::Value>,
}

/// DTO for changing password
#[derive(Debug, Clone, Deserialize)]
pub struct ChangePassword {
    pub new_password: String,
}



#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Response after successful login
#[derive(Debug, Clone, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserResponse,
}
