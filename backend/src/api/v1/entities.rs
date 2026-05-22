//! Entity registration endpoint
//!
//! Provides API for admins to register non-human entities (agents, services, CI systems)
//! and generate API keys for authentication.

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::api_key::{extract_prefix, generate_api_key, hash_api_key};
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::entity::{EntityType, RateLimitTier};

/// Build entity routes
pub fn router() -> Router<AppState> {
    Router::new().route("/register", post(register_entity))
}

/// Request to register a new non-human entity
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterEntityRequest {
    /// Entity type (agent, service, or ci)
    pub entity_type: EntityType,
    /// Unique username for the entity
    pub username: String,
    /// Email address for the entity
    pub email: String,
    /// Optional display name
    pub display_name: Option<String>,
    /// Optional entity metadata (JSON object)
    pub entity_metadata: Option<serde_json::Value>,
}

/// Response after successful entity registration
#[derive(Debug, Clone, Serialize)]
pub struct RegisterEntityResponse {
    /// Entity UUID
    pub id: Uuid,
    /// Entity type
    pub entity_type: EntityType,
    /// Username
    pub username: String,
    /// Email
    pub email: String,
    /// Generated API key (shown only once)
    pub api_key: String,
    /// Assigned rate limit tier
    pub rate_limit_tier: RateLimitTier,
}

/// Register a new non-human entity (agent, service, or CI)
///
/// This endpoint is admin-only and creates a new entity with an API key for authentication.
/// The API key is returned in the response and cannot be retrieved later.
///
/// # Security
///
/// - Requires JWT authentication with admin role
/// - Validates entity_type is non-human (agent, service, or ci)
/// - Generates cryptographically secure API key
/// - Hashes API key with argon2id before storage
///
/// # Rate Limiting
///
/// Automatically assigns rate limit tier based on entity type:
/// - agent -> agent_high (300 req/min)
/// - service -> service_unlimited
/// - ci -> ci_standard (100 req/min)
///
/// # Example Request
///
/// ```json
/// POST /api/v1/entities/register
/// Authorization: Bearer <admin-jwt-token>
///
/// {
///   "entity_type": "agent",
///   "username": "code-assistant",
///   "email": "code-assistant@example.com",
///   "display_name": "Code Assistant Bot",
///   "entity_metadata": {
///     "model": "claude-4-sonnet",
///     "purpose": "code review automation"
///   }
/// }
/// ```
///
/// # Example Response
///
/// ```json
/// {
///   "id": "123e4567-e89b-12d3-a456-426614174000",
///   "entity_type": "agent",
///   "username": "code-assistant",
///   "email": "code-assistant@example.com",
///   "api_key": "a1b2c3d4e5f6...64-char-hex-string",
///   "rate_limit_tier": "agent_high"
/// }
/// ```
async fn register_entity(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<RegisterEntityRequest>,
) -> ApiResult<Json<RegisterEntityResponse>> {
    // Authorization: Only admins can register entities
    if !auth.has_role("system_admin") && !auth.has_role("org_admin") {
        return Err(AppError::Forbidden(
            "Only administrators can register entities".to_string(),
        ));
    }

    // Validation: Entity type must be non-human
    if !input.entity_type.is_non_human() {
        return Err(AppError::BadRequest(
            "Cannot register human entities via this endpoint. Entity type must be 'agent', 'service', or 'ci'.".to_string(),
        ));
    }

    // Validation: Username and email format
    validate_username(&input.username)?;
    validate_email(&input.email)?;

    // Check for duplicate username or email
    check_duplicates(&state.db, &input.username, &input.email).await?;

    // Determine rate limit tier based on entity type
    let rate_limit_tier = input.entity_type.default_rate_limit();

    // Prepare metadata (default to empty object if not provided)
    let entity_metadata = input.entity_metadata.unwrap_or(serde_json::json!({}));

    // Prepare database insertion with retry logic for API key prefix collisions
    const MAX_COLLISION_RETRIES: usize = 3;
    let entity_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let mut attempt = 0;
    let api_key: String;

    loop {
        attempt += 1;

        // Generate and hash API key
        let generated_key = generate_api_key();
        let api_key_prefix = extract_prefix(&generated_key).map_err(|e| {
            tracing::error!("Failed to extract API key prefix: {}", e);
            AppError::Internal("Failed to generate API key".to_string())
        })?;
        let api_key_hash = hash_api_key(&generated_key).await.map_err(|e| {
            tracing::error!("Failed to hash API key: {}", e);
            AppError::Internal("Failed to generate API key".to_string())
        })?;

        // Attempt to insert entity into database
        let insert_result = sqlx::query(
            r#"
            INSERT INTO users (
                id, username, email, display_name,
                entity_type, api_key_hash, api_key_prefix, entity_metadata, rate_limit_tier,
                password_hash, is_bot, is_active, role, presence,
                notify_props, email_verified, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4,
                $5, $6, $7, $8, $9,
                NULL, TRUE, TRUE, 'member', 'offline',
                '{}', TRUE, $10, $11
            )
            "#,
        )
        .bind(entity_id)
        .bind(&input.username)
        .bind(&input.email)
        .bind(&input.display_name)
        .bind(input.entity_type)
        .bind(&api_key_hash)
        .bind(&api_key_prefix)
        .bind(&entity_metadata)
        .bind(rate_limit_tier)
        .bind(now)
        .bind(now)
        .execute(&state.db)
        .await;

        match insert_result {
            Ok(_) => {
                // Success - assign api_key and break out of retry loop
                api_key = generated_key;
                if attempt > 1 {
                    tracing::warn!(
                        entity_id = %entity_id,
                        attempt = attempt,
                        "API key prefix collision resolved after {} attempts",
                        attempt
                    );
                }
                break;
            }
            Err(e) => {
                // Check if this is an API key prefix collision
                if let Some(db_err) = e.as_database_error() {
                    if let Some(constraint) = db_err.constraint() {
                        // Handle username/email collisions (shouldn't happen due to pre-check)
                        if constraint.contains("username") {
                            return Err(AppError::Conflict("Username already exists".to_string()));
                        }
                        if constraint.contains("email") {
                            return Err(AppError::Conflict("Email already exists".to_string()));
                        }

                        // Handle API key prefix collision
                        if constraint.contains("idx_users_api_key_prefix") {
                            if attempt >= MAX_COLLISION_RETRIES {
                                tracing::error!(
                                    entity_id = %entity_id,
                                    attempts = attempt,
                                    "API key prefix collision persisted after {} attempts",
                                    attempt
                                );
                                return Err(AppError::Internal(
                                    "Failed to generate unique API key after multiple attempts"
                                        .to_string(),
                                ));
                            }

                            // Log collision and retry
                            tracing::warn!(
                                entity_id = %entity_id,
                                attempt = attempt,
                                prefix = %api_key_prefix,
                                "API key prefix collision detected, retrying"
                            );
                            continue;
                        }
                    }
                }

                // Not a collision we can retry - fail
                tracing::error!("Failed to insert entity: {}", e);
                return Err(AppError::Database(e));
            }
        }
    }

    tracing::info!(
        entity_id = %entity_id,
        entity_type = ?input.entity_type,
        username = %input.username,
        admin_id = %auth.user_id,
        "Entity registered successfully"
    );

    // Return entity details with API key (shown only once)
    Ok(Json(RegisterEntityResponse {
        id: entity_id,
        entity_type: input.entity_type,
        username: input.username,
        email: input.email,
        api_key,
        rate_limit_tier,
    }))
}

/// Validate username format
fn validate_username(username: &str) -> ApiResult<()> {
    if username.is_empty() {
        return Err(AppError::BadRequest("Username cannot be empty".to_string()));
    }
    if username.len() < 3 {
        return Err(AppError::BadRequest(
            "Username must be at least 3 characters".to_string(),
        ));
    }
    if username.len() > 64 {
        return Err(AppError::BadRequest(
            "Username cannot exceed 64 characters".to_string(),
        ));
    }

    // Username should only contain alphanumeric characters, hyphens, and underscores
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::BadRequest(
            "Username can only contain letters, numbers, hyphens, and underscores".to_string(),
        ));
    }

    Ok(())
}

/// Validate email format (basic check)
fn validate_email(email: &str) -> ApiResult<()> {
    if email.is_empty() {
        return Err(AppError::BadRequest("Email cannot be empty".to_string()));
    }
    if !email.contains('@') {
        return Err(AppError::BadRequest("Invalid email format".to_string()));
    }
    if email.len() > 255 {
        return Err(AppError::BadRequest(
            "Email cannot exceed 255 characters".to_string(),
        ));
    }

    Ok(())
}

/// Check for duplicate username or email
async fn check_duplicates(db: &PgPool, username: &str, email: &str) -> ApiResult<()> {
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT username FROM users WHERE username = $1 OR email = $2 LIMIT 1")
            .bind(username)
            .bind(email)
            .fetch_optional(db)
            .await?;

    if let Some((existing_username,)) = existing {
        if existing_username == username {
            return Err(AppError::Conflict("Username already exists".to_string()));
        } else {
            return Err(AppError::Conflict("Email already exists".to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username() {
        // Valid usernames
        assert!(validate_username("agent-bot").is_ok());
        assert!(validate_username("service_account").is_ok());
        assert!(validate_username("ci-system-123").is_ok());

        // Invalid usernames
        assert!(validate_username("").is_err()); // Empty
        assert!(validate_username("ab").is_err()); // Too short
        assert!(validate_username(&"a".repeat(65)).is_err()); // Too long
        assert!(validate_username("user@domain").is_err()); // Invalid characters
        assert!(validate_username("user name").is_err()); // Spaces
    }

    #[test]
    fn test_validate_email() {
        // Valid emails
        assert!(validate_email("agent@example.com").is_ok());
        assert!(validate_email("service+tag@domain.co.uk").is_ok());

        // Invalid emails
        assert!(validate_email("").is_err()); // Empty
        assert!(validate_email("notemail").is_err()); // No @
        assert!(validate_email(&format!("{}@domain.com", "a".repeat(250))).is_err());
        // Too long
    }
}
