#![allow(clippy::needless_borrows_for_generic_args)]
//! Security integration tests
//!
//! Tests for security-critical functionality including:
//! - Rate limiting
//! - OAuth secure token exchange
//! - WebSocket token handling
//! - Secret validation

mod common;

use axum::http::StatusCode;
use deadpool_redis::redis::AsyncCommands;
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use uuid::Uuid;

use common::{spawn_app, spawn_app_with_config, test_config};
use rustchat::services::oauth_token_exchange::{create_exchange_code, ExchangeCodePayload};

fn unique_test_ip() -> String {
    let bytes = Uuid::new_v4().as_bytes().to_owned();
    format!("203.0.113.{}", 1 + (bytes[0] % 200))
}

async fn create_test_user_token(app: &common::TestApp) -> String {
    let client_ip = unique_test_ip();
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Security Test Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    let username = format!("sec_user_{}", Uuid::new_v4().simple());
    let email = format!("{}@example.com", username);

    let register_response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .header("X-Forwarded-For", &client_ip)
        .json(&json!({
            "username": username,
            "email": email,
            "password": "Str0ng!Passw0rd",
            "display_name": "Security Test User",
            "org_id": org_id
        }))
        .send()
        .await
        .expect("Failed to register user");
    assert_eq!(register_response.status(), StatusCode::OK);

    let login_response = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .header("X-Forwarded-For", &client_ip)
        .json(&json!({
            "email": email,
            "password": "Str0ng!Passw0rd"
        }))
        .send()
        .await
        .expect("Failed to login user");
    assert_eq!(login_response.status(), StatusCode::OK);

    let body: serde_json::Value = login_response
        .json()
        .await
        .expect("Failed to parse login response");
    body["token"]
        .as_str()
        .expect("Missing login token")
        .to_string()
}

/// Test rate limiting on login endpoint
#[tokio::test]
async fn test_login_rate_limiting() {
    let mut config = test_config();
    config.security.rate_limit_enabled = true;
    let app = spawn_app_with_config(config).await;

    // Register a real user so the login handler can find them before the rate limit check.
    let username = format!("rl_user_{}", Uuid::new_v4().simple());
    let email = format!("{}@example.com", username);
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("RL Test Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");
    let reg = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&json!({
            "username": username,
            "email": email,
            "password": "Str0ng!Passw0rd",
            "org_id": org_id
        }))
        .send()
        .await
        .expect("Failed to register");
    assert_eq!(reg.status(), StatusCode::OK);

    // Look up the user id so we can pre-fill the correct Redis key.
    let user_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch user id");

    // Pre-fill the per-account sliding window (key = ratelimit:user:{user_id}).
    // Default max is 10 requests per minute; inserting 10 entries fills the window.
    let redis_key = format!("ratelimit:user:{user_id}");
    let now = chrono::Utc::now().timestamp();
    let mut redis_conn = app
        .redis_pool
        .get()
        .await
        .expect("Failed to get Redis connection");
    let _: usize = redis_conn
        .del(&redis_key)
        .await
        .expect("Failed to cleanup key");
    for i in 0..10_i64 {
        let ts = now - i;
        let _: () = redis_conn
            .zadd(&redis_key, ts, ts)
            .await
            .expect("Failed to seed rate limit key");
    }
    let _: () = redis_conn
        .expire(&redis_key, 60)
        .await
        .expect("Failed to expire rate limit key");

    // The login handler finds the user, then hits the rate limit check → 429.
    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&json!({
            "email": email,
            "password": "wrong-password"
        }))
        .send()
        .await
        .expect("Failed to call login");

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

/// Test OAuth exchange code flow
#[tokio::test]
async fn test_oauth_exchange_code() {
    let app = spawn_app().await;
    let exchange_url = format!("{}/api/v1/oauth2/exchange", app.address);
    let exchange_ip = unique_test_ip();

    let code = create_exchange_code(
        &app.redis_pool,
        Uuid::new_v4(),
        "oauth-security@example.com".to_string(),
        "member".to_string(),
        Some(Uuid::new_v4()),
    )
    .await
    .expect("Failed to create exchange code");

    // First exchange succeeds.
    let first = app
        .api_client
        .post(&exchange_url)
        .header("X-Forwarded-For", &exchange_ip)
        .json(&json!({ "code": code.clone() }))
        .send()
        .await
        .expect("Failed to exchange code");
    assert_eq!(first.status(), StatusCode::OK);
    let first_body: serde_json::Value = first.json().await.expect("Invalid exchange response");
    assert!(first_body["token"].as_str().is_some());
    assert_eq!(first_body["token_type"], "Bearer");

    // Same code is one-time use and must fail.
    let replay = app
        .api_client
        .post(&exchange_url)
        .header("X-Forwarded-For", &exchange_ip)
        .json(&json!({ "code": code }))
        .send()
        .await
        .expect("Failed replay request");
    assert_eq!(replay.status(), StatusCode::BAD_REQUEST);

    // Invalid code fails.
    let invalid = app
        .api_client
        .post(&exchange_url)
        .header("X-Forwarded-For", &exchange_ip)
        .json(&json!({ "code": "invalid-code" }))
        .send()
        .await
        .expect("Failed invalid-code request");
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    // Expired code fails.
    let expired_code = format!("expired-{}", Uuid::new_v4().simple());
    let expired_payload = ExchangeCodePayload {
        user_id: Uuid::new_v4(),
        email: "expired@example.com".to_string(),
        role: "member".to_string(),
        org_id: Some(Uuid::new_v4()),
        created_at: chrono::Utc::now().timestamp() - 120,
        expected_state: None,
        code_challenge: None,
        code_challenge_method: None,
    };
    let mut redis_conn = app
        .redis_pool
        .get()
        .await
        .expect("Failed to get Redis connection");
    let _: () = redis_conn
        .set_ex(
            format!("rustchat:oauth:code:{expired_code}"),
            serde_json::to_string(&expired_payload).expect("Failed to serialize payload"),
            60u64,
        )
        .await
        .expect("Failed to seed expired exchange code");

    let expired = app
        .api_client
        .post(&exchange_url)
        .header("X-Forwarded-For", &exchange_ip)
        .json(&json!({ "code": expired_code }))
        .send()
        .await
        .expect("Failed expired-code request");
    assert_eq!(expired.status(), StatusCode::BAD_REQUEST);
}

/// Test WebSocket token handling
#[tokio::test]
async fn test_websocket_token_sources() {
    let app = spawn_app().await;
    let token = create_test_user_token(&app).await;
    let ws_base = app.address.replacen("http://", "ws://", 1);
    let ws_url = format!("{}/api/v1/ws", ws_base);

    // Authorization header works.
    let mut auth_req = ws_url
        .clone()
        .into_client_request()
        .expect("Failed to build WS request");
    auth_req
        .headers_mut()
        .insert("Authorization", format!("Bearer {token}").parse().unwrap());
    auth_req
        .headers_mut()
        .insert("X-Forwarded-For", unique_test_ip().parse().unwrap());
    let (mut auth_socket, _) = connect_async(auth_req)
        .await
        .expect("Authorization header websocket connection should succeed");
    auth_socket
        .close(None)
        .await
        .expect("Failed to close socket");

    // Query token is rejected.
    let mut query_req = format!("{ws_url}?token={token}")
        .into_client_request()
        .expect("Failed to build query-token WS request");
    query_req
        .headers_mut()
        .insert("X-Forwarded-For", unique_test_ip().parse().unwrap());
    let query_err = connect_async(query_req)
        .await
        .expect_err("Query-token websocket connection should fail");
    match query_err {
        tokio_tungstenite::tungstenite::Error::Http(response) => {
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        other => panic!("Expected HTTP error for query token, got {other:?}"),
    }

    // Sec-WebSocket-Protocol token fallback works.
    let mut protocol_req = ws_url
        .into_client_request()
        .expect("Failed to build protocol-token WS request");
    protocol_req
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", token.parse().unwrap());
    protocol_req
        .headers_mut()
        .insert("X-Forwarded-For", unique_test_ip().parse().unwrap());
    let (mut protocol_socket, response) = connect_async(protocol_req)
        .await
        .expect("Protocol header websocket connection should succeed");
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    assert_eq!(
        response
            .headers()
            .get("Sec-WebSocket-Protocol")
            .and_then(|value| value.to_str().ok()),
        Some(token.as_str())
    );
    protocol_socket
        .close(None)
        .await
        .expect("Failed to close protocol socket");
}

/// Test secret validation at startup
#[test]
fn test_secret_entropy_validation() {
    use rustchat::config::security::validate_secrets;
    use rustchat::config::{Config, DbPoolConfig};

    let build_config = |jwt_secret: &str, encryption_key: &str| Config {
        environment: "test".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 3000,
        database_url: "postgres://rustchat:rustchat@localhost:5432/rustchat".to_string(),
        db_pool: DbPoolConfig::default(),
        redis_url: "redis://localhost:6379".to_string(),
        require_cluster_fanout: false,
        jwt_secret: jwt_secret.to_string(),
        jwt_issuer: None,
        jwt_audience: None,
        encryption_key: encryption_key.to_string(),
        jwt_expiry_hours: 24,
        log_level: "info".to_string(),
        s3_endpoint: None,
        s3_public_endpoint: None,
        s3_bucket: "rustchat".to_string(),
        s3_access_key: None,
        s3_secret_key: None,
        s3_region: "us-east-1".to_string(),
        admin_user: None,
        admin_password: None,
        cors_allowed_origins: None,
        allow_dev_cors: false,
        turnstile: Default::default(),
        calls: Default::default(),
        security: Default::default(),
        keycloak_sync: Default::default(),
        messaging: Default::default(),
        unread: Default::default(),
        compatibility: rustchat::config::CompatibilityConfig {
            mobile_sso_code_exchange: true,
        },
    };

    // Test: Weak secret with low entropy
    let weak_cfg = build_config("password123", "2Rr8Q7VY9r!Y5K2w7pQ4mN8uX6tB1dF3sL0aZ5hJ"); // pragma: allowlist secret
    let result = validate_secrets(&weak_cfg);
    assert!(!result.is_valid);

    // Test: Secret with common pattern
    let common_cfg = build_config(
        "RUSTCHAT_SECRET_KEY_123_do_not_use_in_prod", // pragma: allowlist secret
        "9mT4zL8vC2xN6bQ1jK7hS3dF0pR5wY9uI2oA6eG4",   // pragma: allowlist secret
    );
    let result = validate_secrets(&common_cfg);
    assert!(!result.is_valid);

    // Test: Strong random secret
    let strong_cfg = build_config(
        "7M@xQ2vL9!bN4#pR6$hT8%yU1^cD3&kJ5*zW0+fS", // pragma: allowlist secret
        "3P!nV7@qL2#xR9$gT4%kY6^dM1&hC8*zB5+uF0wJ", // pragma: allowlist secret
    );
    let result = validate_secrets(&strong_cfg);
    assert!(result.is_valid);
}

/// Test permission system
#[tokio::test]
async fn test_authorization_policy() {
    use rustchat::auth::policy::{permissions::*, AuthzResult, PolicyEngine};

    // Test: Admin can delete any post
    let result = PolicyEngine::check_permission("system_admin", &POST_DELETE);
    assert_eq!(result, AuthzResult::Allow);

    // Test: Member cannot delete others' posts without permission
    let result = PolicyEngine::check_permission("member", &USER_MANAGE);
    assert!(matches!(result, AuthzResult::Deny(_)));

    // Test: Owner can update own post
    let user_id = uuid::Uuid::new_v4();
    let result = PolicyEngine::check_ownership("member", &POST_UPDATE, user_id, user_id);
    assert_eq!(result, AuthzResult::Allow);
}

/// Test circuit breaker behavior
#[tokio::test]
async fn test_circuit_breaker() {
    use rustchat::middleware::reliability::{CircuitBreaker, CircuitBreakerConfig, CircuitState};

    let cb = CircuitBreaker::new(
        "test",
        CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_millis(100),
            success_threshold: 1,
        },
    );

    // Initial state is closed
    assert_eq!(cb.state().await, CircuitState::Closed);

    // Fail 3 times
    for _ in 0..3 {
        let _ = cb.execute(|| async { Err::<(), ()>(()) }).await;
    }

    // Circuit should be open
    assert_eq!(cb.state().await, CircuitState::Open);

    // Wait for recovery timeout
    tokio::time::sleep(Duration::from_millis(150)).await;

    // One success should close circuit
    let _ = cb.execute(|| async { Ok::<(), ()>(()) }).await;
    assert_eq!(cb.state().await, CircuitState::Closed);
}
