//! Password Reset Service
//!
//! Handles password setup and reset token generation, validation, and secure password changes.
//! Uses the email provider system for sending reset emails.
//!
//! Security features:
//! - Cryptographically random tokens (256 bits entropy)
//! - SHA-256 hashed token storage (raw tokens never stored)
//! - Single-use tokens with expiry
//! - Rate limiting per IP and email
//! - Constant-time token comparison
//! - Anti-enumeration measures

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::net::IpAddr;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::auth::hash_password;
use crate::error::AppError;
use crate::models::email::EmailPriority;
use crate::services::email_service::{EmailService, EnqueueOptions};

/// Token validity duration (60 minutes for password reset)
const TOKEN_VALIDITY_MINUTES: i64 = 60;
/// Token length in bytes (256 bits = 32 bytes = 43 base64 chars)
const TOKEN_LENGTH: usize = 32;
/// Max attempts per IP per hour
const RATE_LIMIT_IP_HOURLY: i32 = 10;
/// Max attempts per email per hour
const RATE_LIMIT_EMAIL_HOURLY: i32 = 3;

/// Error types specific to password reset
#[derive(Debug, Clone, PartialEq)]
pub enum PasswordResetError {
    TokenNotFound,
    TokenExpired,
    TokenAlreadyUsed,
    RateLimitExceeded,
    InvalidPassword(String),
    UserNotFound,
    EmailNotConfigured,
    Internal(String),
}

impl std::fmt::Display for PasswordResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordResetError::TokenNotFound => write!(f, "Invalid or expired token"),
            PasswordResetError::TokenExpired => write!(f, "Token has expired"),
            PasswordResetError::TokenAlreadyUsed => write!(f, "Token has already been used"),
            PasswordResetError::RateLimitExceeded => {
                write!(f, "Too many attempts. Please try again later.")
            }
            PasswordResetError::InvalidPassword(msg) => write!(f, "Invalid password: {}", msg),
            PasswordResetError::UserNotFound => write!(f, "User not found"),
            PasswordResetError::EmailNotConfigured => {
                write!(f, "Email system not configured. Contact administrator.")
            }
            PasswordResetError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for PasswordResetError {}

impl From<AppError> for PasswordResetError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::NotFound(_) => PasswordResetError::UserNotFound,
            _ => PasswordResetError::Internal(e.to_string()),
        }
    }
}

/// Generate a cryptographically secure random token
fn generate_secure_token() -> String {
    let mut rng = rand::thread_rng();
    let token: String = (0..TOKEN_LENGTH)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect();
    token
}

/// Hash a token using SHA-256 for storage
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Constant-time comparison of two hashes (mitigates timing attacks)
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

/// Check rate limits for password reset requests
async fn check_rate_limits(
    db: &PgPool,
    email: &str,
    ip_address: Option<IpAddr>,
) -> Result<(), PasswordResetError> {
    let since = Utc::now() - Duration::hours(1);

    // Check per-email rate limit
    let email_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM password_reset_tokens 
        WHERE email = $1 
          AND created_at > $2 
          AND created_at > NOW() - INTERVAL '1 hour'
        "#,
    )
    .bind(email)
    .bind(since)
    .fetch_one(db)
    .await
    .map_err(|e| {
        error!("Rate limit check failed: {}", e);
        PasswordResetError::Internal("Rate limit check failed".to_string())
    })?;

    if email_count >= RATE_LIMIT_EMAIL_HOURLY as i64 {
        warn!("Rate limit exceeded for email: {}", email);
        return Err(PasswordResetError::RateLimitExceeded);
    }

    // Check per-IP rate limit if IP is provided
    if let Some(ip) = ip_address {
        let ip_str = ip.to_string();
        let ip_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM password_reset_tokens 
            WHERE created_ip = $1::inet 
              AND created_at > $2
            "#,
        )
        .bind(&ip_str)
        .bind(since)
        .fetch_one(db)
        .await
        .map_err(|e| {
            error!("IP rate limit check failed: {}", e);
            PasswordResetError::Internal("Rate limit check failed".to_string())
        })?;

        if ip_count >= RATE_LIMIT_IP_HOURLY as i64 {
            warn!("Rate limit exceeded for IP: {}", ip);
            return Err(PasswordResetError::RateLimitExceeded);
        }
    }

    Ok(())
}

/// Request a password reset (sends email if user exists)
pub async fn request_password_reset(
    db: &PgPool,
    email: &str,
    ip_address: Option<IpAddr>,
    user_agent: Option<&str>,
) -> Result<(), PasswordResetError> {
    // Normalize email
    let email = email.trim().to_lowercase();

    // Check rate limits before proceeding
    check_rate_limits(db, &email, ip_address).await?;

    // Find user by email
    let user: Option<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, username, email FROM users WHERE email = $1 AND is_active = true AND deleted_at IS NULL"
    )
    .bind(&email)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        error!("Failed to fetch user: {}", e);
        PasswordResetError::Internal("Database error".to_string())
    })?;

    // Create token regardless of whether user exists (anti-enumeration)
    let token = generate_secure_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::minutes(TOKEN_VALIDITY_MINUTES);

    // Store token with metadata
    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens 
            (token_hash, user_id, email, purpose, expires_at, created_ip, user_agent)
        VALUES ($1, $2, $3, $4, $5, $6::inet, $7)
        "#,
    )
    .bind(&token_hash)
    .bind(user.as_ref().map(|(id, _, _)| *id))
    .bind(&email)
    .bind("password_reset")
    .bind(expires_at)
    .bind(ip_address.map(|ip| ip.to_string()))
    .bind(user_agent)
    .execute(db)
    .await
    .map_err(|e| {
        error!("Failed to store reset token: {}", e);
        PasswordResetError::Internal("Failed to create reset token".to_string())
    })?;

    // Send email only if user exists
    if let Some((user_id, username, user_email)) = user {
        // Fetch site_url from server_config
        let site_url: Option<String> =
            sqlx::query_scalar("SELECT site->>'site_url' FROM server_config WHERE id = 'default'")
                .fetch_optional(db)
                .await
                .ok()
                .flatten()
                .filter(|url: &String| !url.is_empty());

        if let Some(site_url) = site_url {
            let reset_link = format!("{}/reset-password?token={}", site_url, token);

            let email_service = EmailService::new(db.clone());
            let payload = serde_json::json!({
                "user_name": username,
                "email": user_email,
                "reset_link": reset_link,
                "expiry_hours": TOKEN_VALIDITY_MINUTES / 60,
                "site_name": "RustChat",
                "ip_address": ip_address.map(|ip| ip.to_string()).unwrap_or_default(),
            });

            match email_service
                .enqueue_email(
                    "password_reset",
                    &user_email,
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
                        "Password reset email enqueued: outbox_id={}, user_id={}",
                        outbox_id, user_id
                    );
                }
                Err(e) => {
                    error!("Failed to enqueue password reset email: {}", e);
                    // Don't fail the request - user can retry
                }
            }
        } else {
            warn!("site_url not configured, cannot send password reset email");
        }
    }

    // Always return Ok to prevent email enumeration
    Ok(())
}

/// Validate a reset token without consuming it (for UI preview)
pub async fn validate_token(
    db: &PgPool,
    token: &str,
) -> Result<(Uuid, String), PasswordResetError> {
    let token_hash = hash_token(token);

    #[allow(clippy::type_complexity)]
    let result: Option<(Uuid, String, String, Option<DateTime<Utc>>, DateTime<Utc>)> =
        sqlx::query_as(
            r#"
        SELECT user_id, email, token_hash, used_at, expires_at
        FROM password_reset_tokens 
        WHERE token_hash = $1
        "#,
        )
        .bind(&token_hash)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            error!("Token validation query failed: {}", e);
            PasswordResetError::Internal("Token validation failed".to_string())
        })?;

    match result {
        Some((user_id, email, stored_hash, used_at, expires_at)) => {
            // Constant-time comparison
            if !constant_time_compare(&stored_hash, &token_hash) {
                return Err(PasswordResetError::TokenNotFound);
            }

            if used_at.is_some() {
                return Err(PasswordResetError::TokenAlreadyUsed);
            }

            if Utc::now() > expires_at {
                return Err(PasswordResetError::TokenExpired);
            }

            Ok((user_id, email))
        }
        None => Err(PasswordResetError::TokenNotFound),
    }
}

/// Reset password with token
pub async fn reset_password(
    db: &PgPool,
    token: &str,
    new_password: &str,
) -> Result<Uuid, PasswordResetError> {
    // Use the same policy source and validation logic as registration.
    let auth_config = crate::services::auth_config::get_password_rules(db)
        .await
        .map_err(|e| {
            error!("Failed to load auth policy: {}", e);
            PasswordResetError::Internal("Failed to load password policy".to_string())
        })?;

    crate::services::auth_config::validate_password(new_password, &auth_config).map_err(|e| {
        if let AppError::Validation(msg) = e {
            PasswordResetError::InvalidPassword(msg)
        } else {
            error!("Password validation failed: {}", e);
            PasswordResetError::Internal("Failed to validate password".to_string())
        }
    })?;

    let token_hash = hash_token(token);

    // Find and validate token (with row lock to prevent race conditions)
    #[allow(clippy::type_complexity)]
    let result: Option<(
        Uuid,
        String,
        String,
        Option<DateTime<Utc>>,
        DateTime<Utc>,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT user_id, email, token_hash, used_at, expires_at, purpose
        FROM password_reset_tokens 
        WHERE token_hash = $1
        FOR UPDATE
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        error!("Token fetch failed: {}", e);
        PasswordResetError::Internal("Token validation failed".to_string())
    })?;

    let (user_id, email, stored_hash, used_at, expires_at, purpose) = match result {
        Some(r) => r,
        None => return Err(PasswordResetError::TokenNotFound),
    };

    // Constant-time comparison
    if !constant_time_compare(&stored_hash, &token_hash) {
        return Err(PasswordResetError::TokenNotFound);
    }

    if used_at.is_some() {
        return Err(PasswordResetError::TokenAlreadyUsed);
    }

    if Utc::now() > expires_at {
        return Err(PasswordResetError::TokenExpired);
    }

    let activate_user = purpose == "password_setup";

    // Hash new password
    let password_hash = hash_password(new_password).map_err(|e| {
        error!("Password hashing failed: {}", e);
        PasswordResetError::Internal("Failed to hash password".to_string())
    })?;

    // Update user password and mark token as used in a transaction
    let mut tx = db.begin().await.map_err(|e| {
        error!("Transaction start failed: {}", e);
        PasswordResetError::Internal("Database error".to_string())
    })?;

    // Mark token as used
    sqlx::query(
        r#"
        UPDATE password_reset_tokens 
        SET used_at = NOW() 
        WHERE token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error!("Token update failed: {}", e);
        PasswordResetError::Internal("Failed to consume token".to_string())
    })?;

    // Update user password
    sqlx::query(
        r#"
        UPDATE users 
        SET password_hash = $1, 
            updated_at = NOW(),
            email_verified = true,
            email_verified_at = COALESCE(email_verified_at, NOW()),
            is_active = CASE WHEN $3 THEN true ELSE is_active END
        WHERE id = $2
        "#,
    )
    .bind(&password_hash)
    .bind(user_id)
    .bind(activate_user)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error!("Password update failed: {}", e);
        PasswordResetError::Internal("Failed to update password".to_string())
    })?;

    tx.commit().await.map_err(|e| {
        error!("Transaction commit failed: {}", e);
        PasswordResetError::Internal("Database error".to_string())
    })?;

    info!("Password reset successful for user {} ({})", user_id, email);
    Ok(user_id)
}

/// Check if user needs password setup (invited users without password)
pub async fn needs_password_setup(db: &PgPool, user_id: Uuid) -> Result<bool, PasswordResetError> {
    let result: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT password_hash FROM users WHERE id = $1 AND is_active = true AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        error!("Failed to check password status: {}", e);
        PasswordResetError::Internal("Database error".to_string())
    })?;

    Ok(result.map(|(hash,)| hash.is_none()).unwrap_or(false))
}

/// Request password setup for invited users
pub async fn request_password_setup(
    db: &PgPool,
    user_id: Uuid,
    ip_address: Option<IpAddr>,
    user_agent: Option<&str>,
) -> Result<(), PasswordResetError> {
    // Get user details
    let user: Option<(String, String)> = sqlx::query_as(
        "SELECT username, email FROM users WHERE id = $1 AND is_active = true AND deleted_at IS NULL"
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        error!("Failed to fetch user: {}", e);
        PasswordResetError::Internal("Database error".to_string())
    })?;

    let (username, email) = match user {
        Some(u) => u,
        None => return Err(PasswordResetError::UserNotFound),
    };

    // Check rate limits
    check_rate_limits(db, &email, ip_address).await?;

    // Create token
    let token = generate_secure_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::minutes(TOKEN_VALIDITY_MINUTES);

    // Store token
    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens 
            (token_hash, user_id, email, purpose, expires_at, created_ip, user_agent)
        VALUES ($1, $2, $3, $4, $5, $6::inet, $7)
        "#,
    )
    .bind(&token_hash)
    .bind(user_id)
    .bind(&email)
    .bind("password_setup")
    .bind(expires_at)
    .bind(ip_address.map(|ip| ip.to_string()))
    .bind(user_agent)
    .execute(db)
    .await
    .map_err(|e| {
        error!("Failed to store setup token: {}", e);
        PasswordResetError::Internal("Failed to create setup token".to_string())
    })?;

    // Fetch site_url
    let site_url: Option<String> =
        sqlx::query_scalar("SELECT site->>'site_url' FROM server_config WHERE id = 'default'")
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .filter(|url: &String| !url.is_empty());

    let site_url = site_url.ok_or_else(|| {
        warn!("site_url not configured");
        PasswordResetError::EmailNotConfigured
    })?;

    let reset_link = format!("{}/setup-password?token={}", site_url, token);

    let email_service = EmailService::new(db.clone());
    let payload = serde_json::json!({
        "user_name": username,
        "email": email,
        "reset_link": reset_link,
        "expiry_hours": TOKEN_VALIDITY_MINUTES / 60,
        "site_name": "RustChat",
    });

    match email_service
        .enqueue_email(
            "password_reset", // Uses same workflow
            &email,
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
                "Password setup email enqueued: outbox_id={}, user_id={}",
                outbox_id, user_id
            );
            Ok(())
        }
        Err(e) => {
            error!("Failed to enqueue password setup email: {}", e);
            Err(PasswordResetError::Internal(
                "Failed to send setup email".to_string(),
            ))
        }
    }
}

/// Send password setup email for new registrations (simplified interface)
pub async fn send_password_setup_email(
    db: &PgPool,
    user_id: Uuid,
    username: &str,
    email: &str,
    site_url: &str,
) -> Result<(), PasswordResetError> {
    // Check rate limits
    check_rate_limits(db, email, None).await?;

    // Create token
    let token = generate_secure_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::minutes(TOKEN_VALIDITY_MINUTES);

    // Store token
    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens 
            (token_hash, user_id, email, purpose, expires_at, created_ip, user_agent)
        VALUES ($1, $2, $3, $4, $5, $6::inet, $7)
        "#,
    )
    .bind(&token_hash)
    .bind(user_id)
    .bind(email)
    .bind("password_setup")
    .bind(expires_at)
    .bind(None::<String>)
    .bind(None::<String>)
    .execute(db)
    .await
    .map_err(|e| {
        error!("Failed to store setup token: {}", e);
        PasswordResetError::Internal("Failed to create setup token".to_string())
    })?;

    let reset_link = format!("{}/set-password?token={}", site_url, token);

    let email_service = EmailService::new(db.clone());
    let payload = serde_json::json!({
        "user_name": username,
        "email": email,
        "reset_link": reset_link,
        "expiry_hours": TOKEN_VALIDITY_MINUTES / 60,
        "site_name": "RustChat",
    });

    match email_service
        .enqueue_email(
            "password_reset", // Uses same workflow
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
                "Password setup email enqueued: outbox_id={}, user_id={}",
                outbox_id, user_id
            );
            Ok(())
        }
        Err(e) => {
            error!("Failed to enqueue password setup email: {}", e);
            Err(PasswordResetError::Internal(
                "Failed to send setup email".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token_length() {
        let token = generate_secure_token();
        assert_eq!(token.len(), TOKEN_LENGTH);
    }

    #[test]
    fn test_generate_token_entropy() {
        // Generate multiple tokens and ensure they're different
        let token1 = generate_secure_token();
        let token2 = generate_secure_token();
        let token3 = generate_secure_token();

        assert_ne!(token1, token2);
        assert_ne!(token2, token3);
        assert_ne!(token1, token3);
    }

    #[test]
    fn test_hash_token_consistency() {
        let token = "test_token_123";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);

        // Same token should produce same hash
        assert_eq!(hash1, hash2);

        // Hash should be 64 chars (SHA-256 hex)
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_hash_token_different_tokens() {
        let hash1 = hash_token("token1");
        let hash2 = hash_token("token2");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_constant_time_compare() {
        // Equal strings
        assert!(constant_time_compare("abc", "abc"));

        // Different strings
        assert!(!constant_time_compare("abc", "def"));

        // Different lengths
        assert!(!constant_time_compare("abc", "abcd"));

        // Empty strings
        assert!(constant_time_compare("", ""));
        assert!(!constant_time_compare("a", ""));
    }
}
