//! User model and related types

use crate::models::entity::{EntityType, RateLimitTier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub fn validate_username_token(username: &str) -> Result<(), &'static str> {
    if username.is_empty() {
        return Err("Username cannot be empty");
    }
    if username.len() < 3 {
        return Err("Username must be at least 3 characters");
    }
    if username.len() > 64 {
        return Err("Username cannot exceed 64 characters");
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Username can only contain letters, numbers, hyphens, and underscores");
    }
    Ok(())
}

/// User roles for RBAC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    SystemAdmin,
    OrgAdmin,
    TeamAdmin,
    #[default]
    Member,
    Guest,
}

/// User entity from database
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    #[sqlx(default)]
    pub first_name: Option<String>,
    #[sqlx(default)]
    pub last_name: Option<String>,
    #[sqlx(default)]
    pub nickname: Option<String>,
    #[sqlx(default)]
    pub position: Option<String>,
    pub is_bot: bool,
    pub is_active: bool,
    pub role: String,
    #[sqlx(default)]
    pub presence: String, // 'online', 'away', 'dnd', 'offline'
    #[sqlx(default)]
    pub status_text: Option<String>,
    #[sqlx(default)]
    pub status_emoji: Option<String>,
    #[sqlx(default)]
    pub status_expires_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub custom_status: Option<serde_json::Value>,
    #[sqlx(default)]
    pub notify_props: serde_json::Value,
    #[sqlx(default)]
    pub timezone: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub email_verified: bool,
    #[sqlx(default)]
    pub email_verified_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub deleted_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub deleted_by: Option<Uuid>,
    #[sqlx(default)]
    pub delete_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Entity model fields (Phase 1)
    #[sqlx(default)]
    pub entity_type: EntityType,
    #[sqlx(default)]
    #[serde(skip_serializing)]
    pub api_key_hash: Option<String>,
    #[sqlx(default)]
    pub entity_metadata: serde_json::Value,
    #[sqlx(default)]
    pub rate_limit_tier: Option<RateLimitTier>,
}

/// Public user response (without sensitive fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub nickname: Option<String>,
    pub position: Option<String>,
    pub is_bot: bool,
    pub role: String,
    pub presence: String,
    pub status_text: Option<String>,
    pub status_emoji: Option<String>,
    pub status_expires_at: Option<DateTime<Utc>>,
    pub custom_status: Option<serde_json::Value>,
    pub notify_props: serde_json::Value,
    pub timezone: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn public_username(&self) -> String {
        if self.is_deleted() {
            "Deleted user".to_string()
        } else {
            self.username.clone()
        }
    }

    pub fn public_display_name(&self) -> Option<String> {
        if self.is_deleted() {
            Some("Deleted user".to_string())
        } else {
            self.display_name.clone()
        }
    }

    pub fn public_email(&self) -> String {
        if self.is_deleted() {
            "deleted-user@local".to_string()
        } else {
            self.email.clone()
        }
    }

    /// Check if this user is a non-human entity (agent, service, or CI)
    pub fn is_non_human(&self) -> bool {
        self.entity_type.is_non_human()
    }

    /// Get the effective rate limit tier (falls back to entity type default if not set)
    pub fn effective_rate_limit_tier(&self) -> RateLimitTier {
        self.rate_limit_tier
            .unwrap_or_else(|| self.entity_type.default_rate_limit())
    }

    /// Check if this user requires API key authentication
    pub fn requires_api_key(&self) -> bool {
        self.is_non_human()
    }

    /// Clear custom status in-memory if it has expired.
    /// This should be called before converting to UserResponse or mm::User.
    pub fn clear_custom_status_if_expired(&mut self) {
        if let Some(expires_at) = self.status_expires_at {
            if expires_at <= Utc::now() {
                self.status_text = None;
                self.status_emoji = None;
                self.status_expires_at = None;
                self.custom_status = None;
            }
        }
    }
}

pub fn stable_avatar_path(user_id: Uuid) -> String {
    format!("/api/v4/users/{user_id}/image")
}

pub fn legacy_avatar_key_from_url(user_id: Uuid, avatar_url: &str) -> Option<String> {
    let trimmed = avatar_url.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path_without_query = trimmed.split('?').next().unwrap_or(trimmed);
    let candidate = if let Some(idx) = path_without_query.find("/files/") {
        &path_without_query[(idx + 1)..]
    } else if let Some(stripped) = path_without_query.strip_prefix("files/") {
        return Some(format!("files/{stripped}"));
    } else if let Some(stripped) = path_without_query.strip_prefix("/files/") {
        return Some(format!("files/{stripped}"));
    } else {
        return None;
    };

    let expected_prefix = format!("files/{user_id}/");
    if candidate.starts_with(&expected_prefix) {
        Some(candidate.to_string())
    } else {
        None
    }
}

pub fn normalize_avatar_url(user_id: Uuid, avatar_url: Option<&str>) -> Option<String> {
    let trimmed = avatar_url?.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("/api/v4/users/") || trimmed.starts_with("/api/v1/files/") {
        return Some(trimmed.to_string());
    }

    if legacy_avatar_key_from_url(user_id, trimmed).is_some() {
        return Some(stable_avatar_path(user_id));
    }

    // Structured logging to catch any remaining presigned URL leaks
    let looks_presigned =
        trimmed.contains("X-Amz-") || trimmed.contains("x-id=") || trimmed.contains("presigned");
    if looks_presigned {
        tracing::warn!(
            user_id = %user_id,
            avatar_url = %trimmed,
            "Avatar URL contains presigned indicators but was not normalized"
        );
    }

    Some(trimmed.to_string())
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            org_id: user.org_id,
            username: user.public_username(),
            email: user.public_email(),
            display_name: user.public_display_name(),
            avatar_url: normalize_avatar_url(user.id, user.avatar_url.as_deref()),
            first_name: user.first_name,
            last_name: user.last_name,
            nickname: user.nickname,
            position: user.position,
            is_bot: user.is_bot,
            role: user.role,
            presence: user.presence,
            status_text: user.status_text,
            status_emoji: user.status_emoji,
            status_expires_at: user.status_expires_at,
            custom_status: user.custom_status,
            notify_props: user.notify_props,
            timezone: user.timezone,
            email_verified: user.email_verified,
            created_at: user.created_at,
        }
    }
}

/// DTO for creating a new user
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: Option<String>,
    pub display_name: Option<String>,
    pub org_id: Option<Uuid>,
    /// Cloudflare Turnstile token (bot protection)
    #[serde(rename = "cf-turnstile-response")]
    pub turnstile_token: Option<String>,
    /// Honeypot field - should be empty (bots usually fill this)
    #[serde(rename = "website")]
    pub honeypot: Option<String>,
}

/// DTO for updating a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub custom_status: Option<serde_json::Value>,
}

/// DTO for changing password
#[derive(Debug, Clone, Deserialize)]
pub struct ChangePassword {
    pub current_password: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::{
        legacy_avatar_key_from_url, normalize_avatar_url, stable_avatar_path,
        validate_username_token,
    };
    use uuid::Uuid;

    #[test]
    fn normalizes_internal_presigned_avatar_urls_to_stable_path() {
        let user_id = Uuid::parse_str("545a19ad-35e4-4cb6-85d6-8b6fc9b09b50").unwrap();
        let presigned = "https://s3.example.com/rustchat-uploads/files/545a19ad-35e4-4cb6-85d6-8b6fc9b09b50/9e0472c0-de97-4b1a-a2cc-b3b929f53f16.jpg?x-id=GetObject&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Date=20260401T232501Z";

        assert_eq!(
            normalize_avatar_url(user_id, Some(presigned)),
            Some(stable_avatar_path(user_id))
        );
    }

    #[test]
    fn preserves_external_avatar_urls() {
        let user_id = Uuid::new_v4();
        let external = "https://cdn.example.com/avatar.jpg";

        assert_eq!(
            normalize_avatar_url(user_id, Some(external)),
            Some(external.to_string())
        );
    }

    #[test]
    fn extracts_legacy_avatar_key_from_presigned_url() {
        let user_id = Uuid::parse_str("545a19ad-35e4-4cb6-85d6-8b6fc9b09b50").unwrap();
        let presigned = "https://s3.example.com/rustchat-uploads/files/545a19ad-35e4-4cb6-85d6-8b6fc9b09b50/9e0472c0-de97-4b1a-a2cc-b3b929f53f16.jpg?x-id=GetObject&X-Amz-Algorithm=AWS4-HMAC-SHA256";

        assert_eq!(
            legacy_avatar_key_from_url(user_id, presigned),
            Some("files/545a19ad-35e4-4cb6-85d6-8b6fc9b09b50/9e0472c0-de97-4b1a-a2cc-b3b929f53f16.jpg".to_string())
        );
    }

    #[test]
    fn validate_username_token_accepts_safe_tokens() {
        assert!(validate_username_token("agent-bot").is_ok());
        assert!(validate_username_token("service_account").is_ok());
        assert!(validate_username_token("ci123").is_ok());
    }

    #[test]
    fn validate_username_token_rejects_unsafe_tokens() {
        assert!(validate_username_token("").is_err());
        assert!(validate_username_token("ab").is_err());
        assert!(validate_username_token(&"a".repeat(65)).is_err());
        assert!(validate_username_token("user.name").is_err());
        assert!(validate_username_token("user@domain").is_err());
        assert!(validate_username_token("user name").is_err());
    }
}
