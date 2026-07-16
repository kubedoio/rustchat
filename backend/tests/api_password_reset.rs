#![allow(clippy::needless_borrows_for_generic_args)]
//! Integration tests for password reset functionality
//!
//! Tests cover:
//! - Token generation and validation
//! - Password reset flow
//! - Rate limiting
//! - Security/negative cases

use rustchat::services::password_reset::{
    request_password_reset, reset_password, validate_token, PasswordResetError,
};
use sha2::Digest;
use sqlx::PgPool;
use uuid::Uuid;

mod common;
use common::spawn_app;

// ============================================
// Helper Functions
// ============================================

async fn create_test_user(db: &PgPool, email: &str) -> (Uuid, String) {
    let user_id = Uuid::new_v4();
    let username = format!("user_{}", user_id.to_string().split('-').next().unwrap());
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$..."; // Dummy hash

    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, password_hash, display_name, is_active, role)
        VALUES ($1, $2, $3, $4, $5, true, 'member')
        ON CONFLICT (email) DO UPDATE SET 
            is_active = true, 
            deleted_at = NULL,
            password_hash = EXCLUDED.password_hash
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(&username)
    .bind(email)
    .bind(password_hash)
    .bind("Test User")
    .fetch_one(db)
    .await
    .expect("Failed to create test user");

    (user_id, username)
}

async fn setup_mail_provider(db: &PgPool) {
    // Ensure there's a default mail provider configured for tests
    sqlx::query(
        r#"
        INSERT INTO mail_provider_settings (
            id, provider_type, host, port, username, password_encrypted,
            tls_mode, from_address, from_name, enabled, is_default
        ) VALUES (
            '00000000-0000-0000-0000-000000000001'::uuid,
            'smtp',
            'localhost',
            1025,
            '',
            '',
            'none',
            'test@rustchat.local',
            'RustChat Test',
            true,
            true
        )
        ON CONFLICT (id) DO UPDATE SET enabled = true, is_default = true
        "#,
    )
    .execute(db)
    .await
    .expect("Failed to setup mail provider");
}

async fn setup_site_url(db: &PgPool) {
    sqlx::query(
        "UPDATE server_config SET site = jsonb_set(site, '{site_url}', '\"http://localhost:3000\"') WHERE id = 'default'"
    )
    .execute(db)
    .await
    .expect("Failed to set site_url");
}

async fn set_auth_policy_defaults(db: &PgPool) {
    sqlx::query(
        r#"
        UPDATE server_config
        SET authentication = authentication
            || '{"password_min_length":8,"password_require_lowercase":true,"password_require_uppercase":true,"password_require_number":true,"password_require_symbol":false}'::jsonb
        WHERE id = 'default'
        "#,
    )
    .execute(db)
    .await
    .expect("Failed to set auth policy defaults");
}

// ============================================
// Unit Tests for Token Functions
// ============================================

#[tokio::test]
async fn test_request_password_reset_creates_token() {
    let app = spawn_app().await;
    let (_user_id, _) = create_test_user(&app.db_pool, "test_reset@example.com").await;
    setup_mail_provider(&app.db_pool).await;
    setup_site_url(&app.db_pool).await;

    // Request password reset
    let result = request_password_reset(
        &app.db_pool,
        "test_reset@example.com",
        Some("127.0.0.1".parse().unwrap()),
        None,
    )
    .await;

    assert!(result.is_ok());

    // Verify token was created
    let token_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM password_reset_tokens WHERE email = $1)")
            .bind("test_reset@example.com")
            .fetch_one(&app.db_pool)
            .await
            .expect("Failed to check token");

    assert!(token_exists);
}

#[tokio::test]
async fn test_request_password_reset_anti_enumeration() {
    let app = spawn_app().await;
    setup_mail_provider(&app.db_pool).await;
    setup_site_url(&app.db_pool).await;

    // Request for non-existent email should still succeed
    let result = request_password_reset(
        &app.db_pool,
        "nonexistent@example.com",
        Some("127.0.0.1".parse().unwrap()),
        None,
    )
    .await;

    assert!(result.is_ok());

    // Token should be created even for non-existent user (anti-enumeration)
    let token_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM password_reset_tokens WHERE email = $1)")
            .bind("nonexistent@example.com")
            .fetch_one(&app.db_pool)
            .await
            .expect("Failed to check token");

    assert!(token_exists);
}

#[tokio::test]
async fn test_password_reset_full_flow() {
    let app = spawn_app().await;
    let (user_id, _) = create_test_user(&app.db_pool, "test_flow@example.com").await;
    setup_mail_provider(&app.db_pool).await;
    setup_site_url(&app.db_pool).await;

    // Step 1: Request reset
    request_password_reset(
        &app.db_pool,
        "test_flow@example.com",
        Some("127.0.0.1".parse().unwrap()),
        None,
    )
    .await
    .expect("Failed to request reset");

    // Step 2: Get token from database (simulating email extraction)
    let token_hash: String = sqlx::query_scalar(
        "SELECT token_hash FROM password_reset_tokens WHERE email = $1 ORDER BY created_at DESC LIMIT 1"
    )
    .bind("test_flow@example.com")
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to get token hash");
    assert!(!token_hash.is_empty());

    // For testing, we need to find the raw token - in production this would be from email
    // Since we hash tokens, we'll create a test token directly
    let test_token = "test_token_for_flow_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    // Update the token hash to our known value
    sqlx::query("UPDATE password_reset_tokens SET token_hash = $1 WHERE email = $2")
        .bind(&test_token_hash)
        .bind("test_flow@example.com")
        .execute(&app.db_pool)
        .await
        .expect("Failed to update token");

    // Step 3: Validate token
    let (validated_user_id, _) = validate_token(&app.db_pool, test_token)
        .await
        .expect("Token should be valid");

    assert_eq!(validated_user_id, user_id);

    // Step 4: Reset password
    let new_password = "NewStr0ng!Passw0rd";
    let reset_result = reset_password(&app.db_pool, test_token, new_password).await;

    assert!(reset_result.is_ok());
    assert_eq!(reset_result.unwrap(), user_id);

    // Step 5: Verify token is now marked as used
    let used_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT used_at FROM password_reset_tokens WHERE token_hash = $1")
            .bind(&test_token_hash)
            .fetch_one(&app.db_pool)
            .await
            .expect("Failed to check used_at");

    assert!(used_at.is_some());
}

#[tokio::test]
async fn test_password_reset_token_replay_protection() {
    let app = spawn_app().await;
    let (user_id, _) = create_test_user(&app.db_pool, "test_replay@example.com").await;
    setup_mail_provider(&app.db_pool).await;
    setup_site_url(&app.db_pool).await;

    // Create a test token
    let test_token = "test_token_for_replay_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (token_hash, user_id, email, purpose, expires_at)
        VALUES ($1, $2, $3, 'password_reset', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(&test_token_hash)
    .bind(user_id)
    .bind("test_replay@example.com")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert token");

    // First reset should succeed
    let new_password = "NewStr0ng!Passw0rd";
    let result1 = reset_password(&app.db_pool, test_token, new_password).await;
    assert!(result1.is_ok());

    // Second attempt with same token should fail
    let result2 = reset_password(&app.db_pool, test_token, new_password).await;
    assert!(matches!(result2, Err(PasswordResetError::TokenAlreadyUsed)));
}

#[tokio::test]
async fn test_password_reset_expired_token() {
    let app = spawn_app().await;
    let (user_id, _) = create_test_user(&app.db_pool, "test_expired@example.com").await;

    // Create an expired token directly in database
    let test_token = "test_token_expired_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (token_hash, user_id, email, purpose, expires_at)
        VALUES ($1, $2, $3, 'password_reset', NOW() - INTERVAL '1 minute')
        "#,
    )
    .bind(&test_token_hash)
    .bind(user_id)
    .bind("test_expired@example.com")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert expired token");

    // Reset with expired token should fail
    let result = reset_password(&app.db_pool, test_token, "NewStr0ng!Passw0rd").await;
    assert!(matches!(result, Err(PasswordResetError::TokenExpired)));
}

#[tokio::test]
async fn test_password_reset_invalid_token() {
    let app = spawn_app().await;
    create_test_user(&app.db_pool, "test_invalid@example.com").await;

    // Try to reset with non-existent token
    let result = reset_password(&app.db_pool, "invalid_token_12345", "NewStr0ng!Passw0rd").await;
    assert!(matches!(result, Err(PasswordResetError::TokenNotFound)));
}

#[tokio::test]
async fn test_password_policy_validation() {
    let app = spawn_app().await;
    let (user_id, _) = create_test_user(&app.db_pool, "test_policy@example.com").await;
    sqlx::query(
        r#"
        UPDATE server_config
        SET authentication = authentication
            || '{"password_min_length":8,"password_require_lowercase":true,"password_require_uppercase":true,"password_require_number":true,"password_require_symbol":true}'::jsonb
        WHERE id = 'default'
        "#,
    )
    .execute(&app.db_pool)
    .await
    .expect("Failed to set strict auth policy");

    // Create a test token
    let test_token = "test_token_policy_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (token_hash, user_id, email, purpose, expires_at)
        VALUES ($1, $2, $3, 'password_reset', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(&test_token_hash)
    .bind(user_id)
    .bind("test_policy@example.com")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert token");

    // Test too short password
    let result = reset_password(&app.db_pool, test_token, "Short1!").await;
    assert!(matches!(
        result,
        Err(PasswordResetError::InvalidPassword(_))
    ));

    // Test missing uppercase
    let result = reset_password(&app.db_pool, test_token, "lowercase123!").await;
    assert!(matches!(
        result,
        Err(PasswordResetError::InvalidPassword(_))
    ));

    // Test missing lowercase
    let result = reset_password(&app.db_pool, test_token, "UPPERCASE123!").await;
    assert!(matches!(
        result,
        Err(PasswordResetError::InvalidPassword(_))
    ));

    // Test missing digit
    let result = reset_password(&app.db_pool, test_token, "PasswordNoDigit!").await;
    assert!(matches!(
        result,
        Err(PasswordResetError::InvalidPassword(_))
    ));

    // Test missing special char
    let result = reset_password(&app.db_pool, test_token, "PasswordNoSpecial123").await;
    assert!(matches!(
        result,
        Err(PasswordResetError::InvalidPassword(_))
    ));
}

#[tokio::test]
async fn test_password_reset_respects_server_auth_policy() {
    let app = spawn_app().await;
    let (user_id, _) = create_test_user(&app.db_pool, "test_policy_config@example.com").await;
    set_auth_policy_defaults(&app.db_pool).await;

    let test_token = "test_policy_config_token_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (token_hash, user_id, email, purpose, expires_at)
        VALUES ($1, $2, $3, 'password_reset', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(&test_token_hash)
    .bind(user_id)
    .bind("test_policy_config@example.com")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert token");

    // Valid according to default auth policy (8 chars, upper/lower/number, symbol optional)
    let result = reset_password(&app.db_pool, test_token, "Abcdef12").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_passwordless_registration_stays_inactive_until_password_setup() {
    let app = spawn_app().await;
    set_auth_policy_defaults(&app.db_pool).await;
    let email = "pending_setup@example.com";

    let register_response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&serde_json::json!({
            "username": "pending_setup_user",
            "email": email
        }))
        .send()
        .await
        .expect("Failed to execute register request");

    assert_eq!(200, register_response.status().as_u16());

    let (user_id, is_active, has_password, email_verified): (Uuid, bool, bool, bool) =
        sqlx::query_as(
            r#"
            SELECT id, is_active, password_hash IS NOT NULL AS has_password, email_verified
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to query registered user");

    assert!(!is_active);
    assert!(!has_password);
    assert!(!email_verified);

    let test_token = "test_password_setup_token_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (token_hash, user_id, email, purpose, expires_at)
        VALUES ($1, $2, $3, 'password_setup', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(&test_token_hash)
    .bind(user_id)
    .bind(email)
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert password setup token");

    let result = reset_password(&app.db_pool, test_token, "Abcdef12").await;
    assert!(result.is_ok());

    let (is_active_after, has_password_after, email_verified_after): (bool, bool, bool) =
        sqlx::query_as(
            r#"
            SELECT is_active, password_hash IS NOT NULL AS has_password, email_verified
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to query updated user");

    assert!(is_active_after);
    assert!(has_password_after);
    assert!(email_verified_after);
}

#[tokio::test]
async fn test_rate_limit_per_email() {
    let app = spawn_app().await;
    create_test_user(&app.db_pool, "test_rate_email@example.com").await;
    setup_mail_provider(&app.db_pool).await;
    setup_site_url(&app.db_pool).await;

    // Make 3 requests (limit is 3 per hour per email)
    for i in 0..3 {
        let result = request_password_reset(
            &app.db_pool,
            "test_rate_email@example.com",
            Some(format!("127.0.0.{}", i).parse().unwrap()),
            None,
        )
        .await;
        assert!(result.is_ok(), "Request {} should succeed", i);
    }

    // 4th request should fail with rate limit
    let result = request_password_reset(
        &app.db_pool,
        "test_rate_email@example.com",
        Some("127.0.0.10".parse().unwrap()),
        None,
    )
    .await;

    assert!(matches!(result, Err(PasswordResetError::RateLimitExceeded)));
}

#[tokio::test]
async fn test_rate_limit_per_ip() {
    let app = spawn_app().await;
    setup_mail_provider(&app.db_pool).await;
    setup_site_url(&app.db_pool).await;

    // Create multiple users
    for i in 0..10 {
        create_test_user(&app.db_pool, &format!("user{}@example.com", i)).await;
    }

    // Make 10 requests from same IP (limit is 10 per hour per IP)
    for i in 0..10 {
        let result = request_password_reset(
            &app.db_pool,
            &format!("user{}@example.com", i),
            Some("192.168.1.1".parse().unwrap()),
            None,
        )
        .await;
        assert!(result.is_ok(), "Request {} should succeed", i);
    }

    // 11th request from same IP should fail
    let result = request_password_reset(
        &app.db_pool,
        "user11@example.com",
        Some("192.168.1.1".parse().unwrap()),
        None,
    )
    .await;

    assert!(matches!(result, Err(PasswordResetError::RateLimitExceeded)));
}

// ============================================
// API Integration Tests
// ============================================

#[tokio::test]
async fn test_api_forgot_password_endpoint() {
    let app = spawn_app().await;
    create_test_user(&app.db_pool, "test_api_forgot@example.com").await;
    setup_mail_provider(&app.db_pool).await;
    setup_site_url(&app.db_pool).await;

    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/password/forgot", app.address))
        .json(&serde_json::json!({
            "email": "test_api_forgot@example.com"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
}

#[tokio::test]
async fn test_api_forgot_password_anti_enumeration() {
    let app = spawn_app().await;
    setup_mail_provider(&app.db_pool).await;
    setup_site_url(&app.db_pool).await;

    // Request for non-existent email should return same success response
    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/password/forgot", app.address))
        .json(&serde_json::json!({
            "email": "nonexistent_api@example.com"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
}

#[tokio::test]
async fn test_api_reset_password_endpoint() {
    let app = spawn_app().await;
    let (user_id, _) = create_test_user(&app.db_pool, "test_api_reset@example.com").await;

    // Create a test token directly
    let test_token = "test_api_token_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (token_hash, user_id, email, purpose, expires_at)
        VALUES ($1, $2, $3, 'password_reset', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(&test_token_hash)
    .bind(user_id)
    .bind("test_api_reset@example.com")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert token");

    // Reset password via API
    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/password/reset", app.address))
        .json(&serde_json::json!({
            "token": test_token,
            "new_password": "NewStr0ng!Passw0rd123"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
}

#[tokio::test]
async fn test_api_validate_token_endpoint() {
    let app = spawn_app().await;
    let (user_id, _) = create_test_user(&app.db_pool, "test_api_validate@example.com").await;

    // Create a test token
    let test_token = "test_api_validate_token_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (token_hash, user_id, email, purpose, expires_at)
        VALUES ($1, $2, $3, 'password_reset', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(&test_token_hash)
    .bind(user_id)
    .bind("test_api_validate@example.com")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert token");

    // Validate token via API
    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/password/validate", app.address))
        .json(&serde_json::json!({
            "token": test_token
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body.get("valid").and_then(|v| v.as_bool()).unwrap_or(false));
    assert_eq!(
        body.get("user_id").and_then(|v| v.as_str()),
        Some(user_id.to_string().as_str())
    );
}

#[tokio::test]
async fn test_api_validate_invalid_token() {
    let app = spawn_app().await;

    // Validate invalid token
    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/password/validate", app.address))
        .json(&serde_json::json!({
            "token": "invalid_token_xyz"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(!body.get("valid").and_then(|v| v.as_bool()).unwrap_or(true));
}

#[tokio::test]
async fn test_api_reset_with_weak_password() {
    let app = spawn_app().await;
    let (user_id, _) = create_test_user(&app.db_pool, "test_api_weak@example.com").await;

    // Create a test token
    let test_token = "test_api_weak_token_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (token_hash, user_id, email, purpose, expires_at)
        VALUES ($1, $2, $3, 'password_reset', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(&test_token_hash)
    .bind(user_id)
    .bind("test_api_weak@example.com")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert token");

    // Try to reset with weak password
    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/password/reset", app.address))
        .json(&serde_json::json!({
            "token": test_token,
            "new_password": "weak"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    // Should return 422 Unprocessable Entity for validation errors
    assert_eq!(422, response.status().as_u16());
}

#[tokio::test]
async fn test_password_reset_changes_password_hash() {
    let app = spawn_app().await;
    let (user_id, _) = create_test_user(&app.db_pool, "test_hash_change@example.com").await;

    // Get original password hash
    let original_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to get original hash");

    // Create a test token
    let test_token = "test_hash_token_12345";
    let test_token_hash = format!("{:x}", sha2::Sha256::digest(test_token.as_bytes()));

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (token_hash, user_id, email, purpose, expires_at)
        VALUES ($1, $2, $3, 'password_reset', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(&test_token_hash)
    .bind(user_id)
    .bind("test_hash_change@example.com")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert token");

    // Reset password
    reset_password(&app.db_pool, test_token, "NewStr0ng!Passw0rd123")
        .await
        .expect("Failed to reset password");

    // Get new password hash
    let new_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to get new hash");

    // Hash should have changed
    assert_ne!(original_hash, new_hash);

    // New hash should be valid argon2
    assert!(new_hash.starts_with("$argon2id$"));
}
