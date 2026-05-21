//! Email Verification Service
//!
//! Handles email verification token generation, sending, and verification.
//! Uses the email provider system for sending emails.

use chrono::{Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::email::EmailPriority;
use crate::services::email_service::{EmailService, EnqueueOptions};

/// Token validity duration (24 hours)
const TOKEN_VALIDITY_HOURS: i64 = 24;
/// Token length in bytes (before hashing)
const TOKEN_LENGTH: usize = 32;

/// Generate a secure random token
fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let token: String = (0..TOKEN_LENGTH)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect();
    token
}

/// Hash a token for storage
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Create or update a verification token for a user
pub async fn create_verification_token(
    db: &sqlx::PgPool,
    user_id: Uuid,
    email: &str,
    purpose: &str,
) -> Result<String, AppError> {
    let token = generate_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::hours(TOKEN_VALIDITY_HOURS);

    // Upsert the token
    sqlx::query(
        r#"
        INSERT INTO email_verification_tokens (user_id, token_hash, email, purpose, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_id, purpose) 
        DO UPDATE SET 
            token_hash = EXCLUDED.token_hash,
            email = EXCLUDED.email,
            expires_at = EXCLUDED.expires_at,
            used_at = NULL,
            created_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(email)
    .bind(purpose)
    .bind(expires_at)
    .execute(db)
    .await
    .map_err(|e| {
        error!("Failed to create verification token: {}", e);
        AppError::Internal("Failed to create verification token".to_string())
    })?;

    info!("Created verification token for user {}", user_id);
    Ok(token)
}

/// Verify a token and mark the email as verified
pub async fn verify_token(db: &sqlx::PgPool, token: &str, purpose: &str) -> Result<Uuid, AppError> {
    let token_hash = hash_token(token);

    // Find and validate the token
    let result: Option<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT user_id, email 
        FROM email_verification_tokens 
        WHERE token_hash = $1 
          AND purpose = $2
          AND used_at IS NULL 
          AND expires_at > NOW()
        "#,
    )
    .bind(&token_hash)
    .bind(purpose)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        error!("Failed to verify token: {}", e);
        AppError::Internal("Failed to verify token".to_string())
    })?;

    let (user_id, email) = match result {
        Some(r) => r,
        None => {
            warn!("Invalid or expired verification token");
            return Err(AppError::BadRequest(
                "Invalid or expired verification token".to_string(),
            ));
        }
    };

    // Mark token as used
    sqlx::query(
        r#"
        UPDATE email_verification_tokens 
        SET used_at = NOW() 
        WHERE token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .execute(db)
    .await
    .map_err(|e| {
        error!("Failed to mark token as used: {}", e);
        AppError::Internal("Failed to complete verification".to_string())
    })?;

    // Mark user's email as verified
    sqlx::query(
        r#"
        UPDATE users 
        SET email_verified = true, 
            email_verified_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(db)
    .await
    .map_err(|e| {
        error!("Failed to update user email_verified: {}", e);
        AppError::Internal("Failed to complete verification".to_string())
    })?;

    info!("Email verified for user {} ({})", user_id, email);
    Ok(user_id)
}

/// Check if a user's email is verified
pub async fn is_email_verified(db: &sqlx::PgPool, user_id: Uuid) -> Result<bool, AppError> {
    let result: Option<(bool,)> = sqlx::query_as("SELECT email_verified FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            error!("Failed to check email verification status: {}", e);
            AppError::Internal("Failed to check verification status".to_string())
        })?;

    Ok(result.map(|r| r.0).unwrap_or(false))
}

/// Send email verification email using the email workflow system
pub async fn send_verification_email(
    db: &sqlx::PgPool,
    user_id: Uuid,
    username: &str,
    email: &str,
    verification_url_base: &str,
) -> Result<(), AppError> {
    // Create verification token
    let token = create_verification_token(db, user_id, email, "registration").await?;

    // Build verification link
    let verification_link = format!("{}?token={}", verification_url_base, token);

    // Use the email service to enqueue the verification email
    let email_service = EmailService::new(db.clone());

    let payload = serde_json::json!({
        "username": username,
        "email": email,
        "verification_link": verification_link,
        "expiry_hours": TOKEN_VALIDITY_HOURS,
        "site_name": "RustChat" // Could be configurable
    });

    match email_service
        .enqueue_email(
            "email_verification",
            email,
            Some(user_id),
            payload,
            EnqueueOptions {
                priority: EmailPriority::High,
                ..Default::default()
            },
        )
        .await
    {
        Ok(outbox_id) => {
            info!(
                "Verification email enqueued: outbox_id={}, user_id={}",
                outbox_id, user_id
            );
            Ok(())
        }
        Err(e) => {
            // If the email service fails, we still return Ok but log the error
            // The user can request a resend later
            error!("Failed to enqueue verification email: {}", e);
            Err(AppError::Internal(
                "Failed to send verification email. Please try again later.".to_string(),
            ))
        }
    }
}

/// Resend verification email
pub async fn resend_verification_email(
    db: &sqlx::PgPool,
    user_id: Uuid,
    verification_url_base: &str,
) -> Result<(), AppError> {
    // Get user details
    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            error!("Failed to fetch user: {}", e);
            AppError::Internal("Failed to fetch user".to_string())
        })?
        .ok_or_else(|| AppError::UserNotFound)?;

    if user.email_verified {
        return Err(AppError::BadRequest(
            "Email is already verified".to_string(),
        ));
    }

    send_verification_email(
        db,
        user_id,
        &user.username,
        &user.email,
        verification_url_base,
    )
    .await
}

/// Check if email verification is required for registration
/// This could be configured per-server in the future
pub fn is_verification_required() -> bool {
    // For now, always require verification if email is configured
    // Could be made configurable via server_config
    true
}
